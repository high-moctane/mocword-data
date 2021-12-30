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
	"strconv"
	"strings"
	"sync"

	"github.com/cenkalti/backoff"
	"github.com/pierrec/lz4"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
)

var dlSema = make(chan struct{}, 2)
var parseSema = make(chan struct{}, 30)
var saveSema = make(chan struct{}, 1)

type Cache map[string]int64

const cacheOffset = 10000

func RunDownload(ctx context.Context) error {
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
	conn, err := gorm.Open(sqlite.Open("file:data.sqlite?cache=shared"), &gorm.Config{})
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
	conn.WithContext(ctx).AutoMigrate(&Query{})
	conn.WithContext(ctx).AutoMigrate(&OneGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&TwoGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&ThreeGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&FourGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&FiveGramRecord{})
	return nil
}

func DownloadAndSaveAll(ctx context.Context, conn *gorm.DB, n int, cache Cache) error {
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

	return nil
}

func DownloadAndSave(ctx context.Context, conn *gorm.DB, query Query, cache Cache) error {
	// Download
	gzFile, err := os.CreateTemp("", "gz")
	if err != nil {
		return fmt.Errorf("failed to create temp gz file: %w", err)
	}
	defer os.Remove(gzFile.Name())
	defer gzFile.Close()

	if err := RetryDownload(ctx, query, gzFile); err != nil {
		return fmt.Errorf("failed to download: %w", err)
	}

	// Parse
	parsedFile, err := os.CreateTemp("", "parsed")
	if err != nil {
		return fmt.Errorf("failed to create parsed file: %w", err)
	}
	defer os.Remove(parsedFile.Name())
	defer parsedFile.Close()

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
	if strings.Contains(rec.words, "_") {
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

func WriteRecord(ctx context.Context, w io.Writer, rec Record) error {
	_, err := w.Write([]byte(rec.String()))
	if err != nil {
		return fmt.Errorf("failed to write record: %w", err)
	}
	return nil
}

func ParseGzNGram(ctx context.Context, r io.Reader, w io.Writer, query Query) error {
	return nil
}

func Save(ctx context.Context, conn *gorm.DB, r io.Reader, query Query, cache Cache) error {
	saveSema <- struct{}{}
	defer func() { <-saveSema }()

	bufr := bufio.NewReader(r)
	lz4r := lz4.NewReader(bufr)
	sc := bufio.NewScanner(lz4r)

	conn.WithContext(ctx).Transaction(func(conn *gorm.DB) error {
		for sc.Scan() {
			elems := strings.Split(sc.Text(), "\t")
			words := strings.Split(elems[0], " ")
			score, err := strconv.ParseInt(elems[1], 10, 64)
			if err != nil {
				return fmt.Errorf("invalid score: %w", err)
			}

			var val interface{}

			switch len(words) {
			case 1:
				val = OneGramRecord{Word: words[0], Score: score}
			case 2:
				word1, ok := cache[words[0]]
				if !ok {
					continue
				}
				word2, ok := cache[words[1]]
				if !ok {
					continue
				}
				val = TwoGramRecord{word1, word2, score}
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
				val = ThreeGramRecord{word1, word2, word3, score}
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
				val = FourGramRecord{word1, word2, word3, word4, score}
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
				val = FiveGramRecord{word1, word2, word3, word4, word5, score}
			default:
				log.Panicf("invalid words: %s", words)
			}

			res := conn.Save(val)
			if res.Error != nil {
				return fmt.Errorf("failed to save: %w", res.Error)
			}
		}
		if err := sc.Err(); err != nil {
			return fmt.Errorf("failed to save: %w", err)
		}

		res := conn.Save(query)
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
	}
	log.Panicf("invalid n: %v", n)
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
