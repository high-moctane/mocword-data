package mocword

import (
	"bufio"
	"compress/gzip"
	"context"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"os/signal"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"syscall"

	"github.com/cenkalti/backoff"
	"github.com/pierrec/lz4"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
)

var sema = make(chan struct{}, 10)
var dlSema = make(chan struct{}, 2)
var parseSema = make(chan struct{}, 10)
var saveSema = make(chan struct{}, 1)

type Cache map[string]int64

const cacheOffset = 10000

func RunDownload(ctx context.Context) error {
	ctx, stop := signal.NotifyContext(ctx, os.Interrupt, syscall.SIGTERM)
	defer stop()

	conn, err := NewConn()
	if err != nil {
		return fmt.Errorf("failed to open conn: %w", err)
	}

	if err := Migrate(ctx, conn); err != nil {
		return fmt.Errorf("failed to migrate: %w", err)
	}

	if err := DownloadAndSaveAll(ctx, conn, 1, nil); err != nil {
		return fmt.Errorf("failed to download and save 1-grams: %w", err)
	}

	cache, err := FetchOneGramCache(ctx, conn)
	if err != nil {
		return fmt.Errorf("failed to create cache: %w", err)
	}

	for n := 2; n <= 5; n++ {
		if err := DownloadAndSaveAll(ctx, conn, n, cache); err != nil {
			return fmt.Errorf("failed to download and save %v-grams: %w", n, err)
		}
	}

	return nil
}

func NewConn() (*gorm.DB, error) {
	conn, err := gorm.Open(sqlite.Open("file:data.sqlite?cache=shared"), &gorm.Config{
		Logger: logger.Default.LogMode(logger.Silent),
	})
	if err != nil {
		return nil, fmt.Errorf("failed to open conn: %w", err)
	}
	db, err := conn.DB()
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %w", err)
	}
	db.SetMaxOpenConns(1)

	return conn, nil
}

func Migrate(ctx context.Context, conn *gorm.DB) error {
	log.Println("start: migrate")
	defer log.Println("end  : migrate")

	conn.WithContext(ctx).AutoMigrate(&Query{})
	conn.WithContext(ctx).AutoMigrate(&OneGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&TwoGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&ThreeGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&FourGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&FiveGramRecord{})
	return nil
}

func DownloadAndSaveAll(ctx context.Context, conn *gorm.DB, n int, cache Cache) error {
	log.Printf("start: download and save all %v", n)
	defer log.Printf("end  : download and save all %v", n)

	var wg sync.WaitGroup

	for idx := 0; idx < TotalFileNum(n); idx++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()

			query := Query{n, idx}
			if err := DownloadAndSave(ctx, conn, query, cache); err != nil {
				log.Printf("failed to download and save %v: %v", query, err)
			}
		}(idx)
	}

	wg.Wait()

	return nil
}

func DownloadAndSave(ctx context.Context, conn *gorm.DB, query Query, cache Cache) error {
	select {
	case <-ctx.Done():
		return nil
	default:
	}

	sema <- struct{}{}
	defer func() { <-sema }()

	// log.Printf("start: download and save %v", query)
	// defer log.Printf("end  : download and save %v", query)

	fetched, err := IsFetched(ctx, conn, query)
	if err != nil {
		return fmt.Errorf("failed to download and save: %w", err)
	}
	if fetched {
		return nil
	}

	// Download
	gzFile, err := os.CreateTemp("", "gz")
	if err != nil {
		return fmt.Errorf("failed to create temp gz file: %w", err)
	}
	defer os.Remove(gzFile.Name())

	if err := RetryDownload(ctx, query, gzFile); err != nil {
		return fmt.Errorf("failed to download: %w", err)
	}

	// Parse
	parsedFile, err := os.CreateTemp("", "parsed")
	if err != nil {
		return fmt.Errorf("failed to create parsed file: %w", err)
	}
	defer os.Remove(parsedFile.Name())

	if err := ParseGz(ctx, gzFile, parsedFile, query); err != nil {
		return fmt.Errorf("failed to parse gz: %w", err)
	}
	if _, err := parsedFile.Seek(0, 0); err != nil {
		return fmt.Errorf("failed to seek: %w", err)
	}

	// Save
	if err := Save(ctx, conn, parsedFile, query, cache); err != nil {
		return fmt.Errorf("failed to save: %w", err)
	}

	return nil
}

func RetryDownload(ctx context.Context, query Query, gzFile *os.File) error {
	operation := func() error {
		return Download(ctx, query, gzFile)
	}

	return backoff.Retry(operation, backoff.NewExponentialBackOff())
}

func Download(ctx context.Context, query Query, gzFile *os.File) error {
	dlSema <- struct{}{}
	defer func() { <-dlSema }()

	select {
	case <-ctx.Done():
		return nil
	default:
	}

	log.Printf("start: download %v", query)
	defer log.Printf("end  : download %v", query)

	defer gzFile.Seek(0, 0)

	client := http.DefaultClient
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, FileURL(query.N, query.Idx), nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("failed to get response: %w", err)
	}
	defer resp.Body.Close()

	bufw := bufio.NewWriter(gzFile)
	defer bufw.Flush()

	_, err = io.Copy(bufw, resp.Body)
	if err != nil {
		return fmt.Errorf("failed to write body to writer: %w", err)
	}

	return nil
}

type Record struct {
	words string
	score int64
}

func (rec Record) String() string {
	return fmt.Sprintf("%s\t%v", rec.words, rec.score)
}

func ParseGz(ctx context.Context, gzFile, parsedFile *os.File, query Query) error {
	parseSema <- struct{}{}
	defer func() { <-parseSema }()

	select {
	case <-ctx.Done():
		return nil
	default:
	}

	log.Printf("start: parse gz %v", query)
	defer log.Printf("end  : parse gz %v", query)

	defer parsedFile.Seek(0, 0)

	bufr := bufio.NewReader(gzFile)
	gzr, err := gzip.NewReader(bufr)
	if err != nil {
		return fmt.Errorf("failed to open gzip reader: %w", err)
	}
	defer gzr.Close()
	sc := bufio.NewScanner(gzr)

	bufw := bufio.NewWriter(parsedFile)
	defer bufw.Flush()
	lz4w := lz4.NewWriter(bufw)
	defer lz4w.Close()

	record := Record{}

	for sc.Scan() {
		select {
		case <-ctx.Done():
			return nil
		default:
		}

		newRecord, err := ParseRecord(ctx, sc.Text())
		if err != nil {
			continue
		}
		if record.words == newRecord.words {
			record.score += newRecord.score
		} else {
			if err := WriteRecord(ctx, lz4w, record); err != nil {
				return fmt.Errorf("failed to parse: %w", err)
			}
			record = newRecord
		}
	}
	if sc.Err() != nil {
		return fmt.Errorf("failed to parse: %w", sc.Err())
	}
	if record.score != 0 {
		if err := WriteRecord(ctx, lz4w, record); err != nil {
			return fmt.Errorf("failed to parse: %w", err)
		}
	}

	return nil
}

func ParseRecord(ctx context.Context, line string) (rec Record, err error) {
	elems := strings.Split(line, "\t")
	rec.words = elems[0]
	if !IsValidWords(rec.words) {
		err = fmt.Errorf("invalid word: %w", err)
		return
	}
	for i := 1; i < len(elems); i++ {
		triple := strings.Split(elems[i], ",")
		var score int64
		score, err = strconv.ParseInt(triple[1], 10, 64)
		if err != nil {
			err = fmt.Errorf("failed to parse score: %w", err)
			return
		}
		rec.score += score
	}
	return
}

var reg = regexp.MustCompile(`(?:^\pP|_)`)

func IsValidWords(s string) bool {
	return true &&
		len(s) > 0 &&
		!reg.MatchString(s) &&
		true
}

func WriteRecord(ctx context.Context, w io.Writer, rec Record) error {
	if _, err := w.Write([]byte(rec.String() + "\n")); err != nil {
		return fmt.Errorf("failed to write record: %w", err)
	}
	return nil
}

func Save(ctx context.Context, conn *gorm.DB, r io.Reader, query Query, cache Cache) error {
	saveSema <- struct{}{}
	defer func() { <-saveSema }()

	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	log.Printf("start: save %v", query)
	defer log.Printf("end  : save %v", query)

	bufr := bufio.NewReader(r)
	lz4r := lz4.NewReader(bufr)
	sc := bufio.NewScanner(lz4r)

	conn.WithContext(ctx).Transaction(func(conn *gorm.DB) error {
		for sc.Scan() {
			select {
			case <-ctx.Done():
				return ctx.Err()
			default:
			}

			elems := strings.Split(sc.Text(), "\t")
			words := strings.Split(elems[0], " ")
			score, err := strconv.ParseInt(elems[1], 10, 64)
			if err != nil {
				return fmt.Errorf("invalid score: %w", err)
			}
			if len(words[0]) == 0 || score == 0 {
				continue
			}

			switch len(words) {
			case 1:
				val := OneGramRecord{Word: words[0], Score: score}
				res := conn.Create(&val)
				if res.Error != nil {
					return fmt.Errorf("failed to save: %w", res.Error)
				}
			case 2:
				word1, ok := cache[words[0]]
				if !ok {
					continue
				}
				word2, ok := cache[words[1]]
				if !ok {
					continue
				}
				val := TwoGramRecord{word1, word2, score}
				res := conn.Create(&val)
				if res.Error != nil {
					return fmt.Errorf("failed to save: %w", res.Error)
				}
			case 3:
				word1, ok := cache[words[0]]
				if !ok {
					continue
				}
				word2, ok := cache[words[1]]
				if !ok {
					continue
				}
				word3, ok := cache[words[2]]
				if !ok {
					continue
				}
				val := ThreeGramRecord{word1, word2, word3, score}
				res := conn.Create(&val)
				if res.Error != nil {
					return fmt.Errorf("failed to save: %w", res.Error)
				}
			case 4:
				word1, ok := cache[words[0]]
				if !ok {
					continue
				}
				word2, ok := cache[words[1]]
				if !ok {
					continue
				}
				word3, ok := cache[words[2]]
				if !ok {
					continue
				}
				word4, ok := cache[words[3]]
				if !ok {
					continue
				}
				val := FourGramRecord{word1, word2, word3, word4, score}
				res := conn.Create(&val)
				if res.Error != nil {
					return fmt.Errorf("failed to save: %w", res.Error)
				}
			case 5:
				word1, ok := cache[words[0]]
				if !ok {
					continue
				}
				word2, ok := cache[words[1]]
				if !ok {
					continue
				}
				word3, ok := cache[words[2]]
				if !ok {
					continue
				}
				word4, ok := cache[words[3]]
				if !ok {
					continue
				}
				word5, ok := cache[words[4]]
				if !ok {
					continue
				}
				val := FiveGramRecord{word1, word2, word3, word4, word5, score}
				res := conn.Create(&val)
				if res.Error != nil {
					return fmt.Errorf("failed to save: %w", res.Error)
				}
			default:
				log.Panicf("invalid words: %s", words)
			}

		}
		if err := sc.Err(); err != nil {
			return fmt.Errorf("failed to save: %w", err)
		}

		res := conn.Save(&query)
		if res.Error != nil {
			return fmt.Errorf("failed to save: %w", res.Error)
		}

		return nil
	})

	return nil
}

func TotalFileNum(n int) int {
	switch n {
	case 1:
		return 24
	case 2:
		return 589
	case 3:
		return 6881
	case 4:
		return 6668
	case 5:
		return 19423
	default:
		log.Panicf("invalid n: %v", n)
	}
	return 0
}

func FileURL(n, idx int) string {
	return fmt.Sprintf(
		"http://storage.googleapis.com/books/ngrams/books/20200217/eng/%d-%05d-of-%05d.gz",
		n,
		idx,
		TotalFileNum(n),
	)
}

func FetchOneGramCache(ctx context.Context, conn *gorm.DB) (cache Cache, err error) {
	log.Println("start: make cache")
	defer log.Println("end  : make cache")

	cache = make(Cache)

	start := 0
	end := start + cacheOffset

	var records []OneGramRecord

	for {
		res := conn.
			WithContext(ctx).
			Where("id between ? and ?", start, end).
			Find(&records)
		if res.Error != nil {
			err = fmt.Errorf("failed to fetch cache: %w", res.Error)
			return
		}

		start = end
		end = end + cacheOffset

		if len(records) == 0 {
			break
		}

		for _, rec := range records {
			cache[rec.Word] = rec.ID
		}
	}

	return
}

func IsFetched(ctx context.Context, conn *gorm.DB, query Query) (bool, error) {
	var vals []Query
	res := conn.
		WithContext(ctx).
		Find(&vals, "n = ? and idx = ?", query.N, query.Idx)

	return len(vals) > 0, res.Error
}
