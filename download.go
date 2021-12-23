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
	"sync"
)

const Sentinel = 0xFFFF

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

	bufw := bufio.NewWriter(w)
	defer bufw.Flush()
	lz4w := lz4.NewWriter(bufw)
	defer lz4w.Close()

	switch query.N {
	case 1:
		return ParseGzOneGram(ctx, gzr, lz4w, query)
	default:
		return ParseGzNGram(ctx, gzr, lz4w, query)
	}
}

func ParseGzOneGram(ctx context.Context, r io.Reader, w io.Writer, query Query) error {
	return nil
}

func ParseGzNGram(ctx context.Context, r io.Reader, w io.Writer, query Query) error {
	return nil
}

func Save(ctx context.Context, conn *gorm.DB, r io.Reader) error {
	saveSema <- struct{}{}
	defer func() { <-saveSema }()

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
	log.Panic("invalid n: %v", n)
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
