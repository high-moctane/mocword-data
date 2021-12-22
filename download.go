package mocword

import (
	"context"
	"fmt"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"log"
	"os"
	"sync"
)

func RunDownload(ctx context.Context) error {
	conn, err := NewConn()
	if err != nil {
		return fmt.Errorf("failed to open conn: %w", err)
	}

	if err := Migrate(ctx, conn); err != nil {
		return fmt.Errorf("failed to migrate: %w", err)
	}

	if err := DownloadAndSave(ctx, conn, 1); err != nil {
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
	conn.WithContext(ctx).AutoMigrate(&FetchedFile{})
	conn.WithContext(ctx).AutoMigrate(&OneGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&TwoGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&ThreeGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&FourGramRecord{})
	conn.WithContext(ctx).AutoMigrate(&FiveGramRecord{})
	return nil
}

func DownloadAndSave(ctx context.Context, conn *gorm.DB, n int) error {
	queryCtx, queryCancel := context.WithCancel(ctx)
	queryCh := make(chan FetchedFile)
	var queryWg sync.WaitGroup

	// Query
	for idx := 0; idx < TotalFileNum(n); idx++ {
		queryWg.Add(1)
		go func(queryCtx context.Context, idx int) {
			defer queryWg.Done()

			queryCh <- FetchedFile{n, idx}
		}(queryCtx, idx)
	}

	// Download
	dlCtx, dlCancel := context.WithCancel(ctx)
	dlCh := make(chan os.File)
	var dlWg sync.WaitGroup
	dlWg.Add(1)
	go func(queryCtx, dlCtx context.Context) {
		defer dlWg.Done()

		for {
			select {
			case <-ctx.Done():
				return
			default:
			}

			var query FetchedFile

			select {
			case q := <-queryCh:
				query = q
			case <-queryCtx.Done():
				q, ok := <-queryCh
				if !ok {
					return
				}
				query = q
			case <-dlCtx.Done():
				return
			}

			dlWg.Add(1)
			go func() {
				defer dlWg.Done()

                client := http.DefaultClient
			}()
		}
	}(queryCtx, dlCtx)

	queryWg.Wait()
	queryCancel()

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
