package mocword

import (
	"bufio"
	"compress/gzip"
	"context"
	"fmt"
	"github.com/cenkalti/backoff"
	"github.com/pierrec/lz4"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"io"
	"log"
	"net/http"
	"os"
	"regexp"
	"strconv"
	"strings"
	"sync"
)

var PartOfSpeeches = []string{
	"_NOUN_", "_._", "_VERB_", "_ADP_", "_DET_", "_ADJ_", "_PRON_", "_ADV_", "_NUM_", "_CONJ_",
	"_PRT_", "_X_",
}

var PartOfSpeechSuffixes = []string{
	"_NOUN", "_.", "_VERB", "_ADP", "_DET", "_ADJ", "_PRON", "_ADV", "_NUM", "_CONJ", "_PRT", "_X",
}

var reg = regexp.MustCompile("_(?:NOUN|.|VERB|ADP|DET|ADJ|PRON|ADV|NUM|CONJ|PRT|X)")

var dlSema = make(chan struct{}, 2)
var parseSema = make(chan struct{}, 30)
var saveSema = make(chan struct{}, 1)

func RunDownload(ctx context.Context) error {
	conn, err := NewConn()
	if err != nil {
		return fmt.Errorf("failed to open conn: %w", err)
	}

	if err := Migrate(ctx, conn); err != nil {
		return fmt.Errorf("failed to migrate: %w", err)
	}

	if err := DownloadAndSaveAll(ctx, conn, 1); err != nil {
		return fmt.Errorf("failed to download and save 1-grams: %w", err)
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

func DownloadAndSaveAll(ctx context.Context, conn *gorm.DB, n int) error {
	var wg sync.WaitGroup

	for idx := 0; idx < TotalFileNum(n); idx++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()

			query := Query{n, idx}
			if err := DownloadAndSave(ctx, conn, query); err != nil {
				log.Printf("failed to download and save %v: %v", query, err)
			}
		}(idx)
	}

	return nil
}

func DownloadAndSave(ctx context.Context, conn *gorm.DB, query Query) error {
	// Download
	gzFile, err := os.CreateTemp("", "gz")
	if err != nil {
		return fmt.Errorf("failed to create temp gz file: %w", err)
	}
	defer os.Remove(gzFile.Name())
	defer gzFile.Close()

	if err := Download(ctx, query, gzFile); err != nil {
		return fmt.Errorf("failed to download: %w", err)
	}
	if _, err := gzFile.Seek(0, 0); err != nil {
		return fmt.Errorf("failed to seek: %w", err)
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
	if err := Save(ctx, conn, parsedFile); err != nil {
		return fmt.Errorf("failed to save: %w", err)
	}

	return nil
}

func RetryDownload(ctx context.Context, query Query, w io.Writer) error {
	operation := func() error {
		return Download(ctx, query, w)
	}

	return backoff.Retry(operation, backoff.NewExponentialBackOff())
}

func Download(ctx context.Context, query Query, w io.Writer) error {
	dlSema <- struct{}{}
	defer func() { <-dlSema }()

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

	bufw := bufio.NewWriter(w)
	defer bufw.Flush()

	_, err = io.Copy(bufw, resp.Body)
	if err != nil {
		return fmt.Errorf("failed to write body to writer: %w", err)
	}

	return nil
}

func ParseGz(ctx context.Context, r io.Reader, w io.Writer, query Query) error {
	parseSema <- struct{}{}
	defer func() { <-parseSema }()

	bufr := bufio.NewReader(r)
	gzr, err := gzip.NewReader(bufr)
	if err != nil {
		return fmt.Errorf("failed to open gzip reader: %w", err)
	}
	defer gzr.Close()
	sc := bufio.NewScanner(gzr)

	bufw := bufio.NewWriter(w)
	defer bufw.Flush()
	lz4w := lz4.NewWriter(bufw)
	defer lz4w.Close()

	record := OneGramRecord{}

	for sc.Scan() {
		word, score, err := ParseRecord(ctx, sc.Text())
		if err != nil {
			continue
		}
		if word != record.Word {
			if err := WriteRecord(ctx, w, record.Word, record.Score); err != nil {
				return fmt.Errorf("failed to parse: %w", err)
			}
			record = OneGramRecord{Word: word}
		}
		record.Score += score
	}
	if sc.Err() != nil {
		return fmt.Errorf("failed to parse: %w", sc.Err())
	}
	if record.Score != 0 {
		if err := WriteRecord(ctx, w, record.Word, record.Score); err != nil {
			return fmt.Errorf("failed to parse: %w", err)
		}
	}

	return nil
}

func ParseRecord(ctx context.Context, line string) (words string, score int64, err error) {
	elems := strings.Split(line, "\t")
	words = elems[0]
	if reg.MatchString(words) {
		err = fmt.Errorf("invalid word: %w", err)
		return
	}
	score = 0
	for i := 1; i < len(elems); i++ {
		triple := strings.Split(elems[i], ",")
		var sco int64
		sco, err = strconv.ParseInt(triple[1], 10, 64)
		if err != nil {
			err = fmt.Errorf("failed to parse score: %w", err)
			return
		}
		score += sco
	}
	return
}

func WriteRecord(ctx context.Context, w io.Writer, words string, score int64) error {
	_, err := fmt.Fprintf(w, "%s\t%d\n", words, score)
	if err != nil {
		return fmt.Errorf("failed to write record: %w", err)
	}
	return nil
}

func ParseGzNGram(ctx context.Context, r io.Reader, w io.Writer, query Query) error {
	return nil
}

func Save(ctx context.Context, conn *gorm.DB, r io.Reader) error {
	saveSema <- struct{}{}
	defer func() { <-saveSema }()

	bufr := bufio.NewReader(r)
	lz4r := lz4.NewReader(bufr)
	sc := bufio.NewScanner(lz4r)

	for sc.Scan() {
		elems := strings.Split(sc.Text(), "\t")
		words := strings.Split(elems[0], " ")
		score, err := strconv.Atoi(elems[1])
		if err != nil {
			return fmt.Errorf("failed to save: %w", err)
		}
	}
	if err := sc.Err; err != nil {
		return fmt.Errorf("failed to save: %w", err)
	}

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
