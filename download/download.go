package download

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"net/http"
	"strconv"
	"strings"
	"sync"

	backoff "github.com/cenkalti/backoff/v4"
	"github.com/dghubble/trie"
	"github.com/high-moctane/mocword/entities"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
)

var dbFilename = flag.String("db", "", "DB file name")
var language = flag.String("lang", "eng", "language: eng (default)")
var parallel = flag.Int("parallel", 1, "Parallel worker number")
var dlSema chan struct{}
var workerSema chan struct{}

var fileLangNIndex = map[string]map[int]int{
	"eng": {
		1: 24,
		2: 589,
		3: 6881,
		4: 6668,
		5: 19423,
	},
}

type runeInt64Trie struct {
	t *trie.RuneTrie
}

func newRuneInt64Trie() *runeInt64Trie {
	return &runeInt64Trie{trie.NewRuneTrie()}
}

func (t *runeInt64Trie) Get(key string) (int64, bool) {
	res := t.t.Get(key)
	if res == nil {
		return 0, false
	}
	return res.(int64), true
}

func (t *runeInt64Trie) Put(key string, value int64) bool {
	return t.t.Put(key, value)
}

func Run(ctx context.Context) error {
	flag.Parse()
	dlSema = make(chan struct{}, 2)
	workerSema = make(chan struct{}, *parallel)

	fname := newDSN()

	db, err := gorm.Open(sqlite.Open(fname), &gorm.Config{
		Logger: logger.Default.LogMode(logger.Silent),
	})
	if err != nil {
		return fmt.Errorf("failed to run: %w", err)
	}
	sqlDB, err := db.DB()
	if err != nil {
		return fmt.Errorf("failed to run: %w", err)
	}
	defer sqlDB.Close()
	sqlDB.SetMaxOpenConns(1)

	if err := migrate(ctx, db); err != nil {
		return fmt.Errorf("failed to run: %w", err)
	}

	if err := doOneGrams(ctx, db); err != nil {
		return fmt.Errorf("failed to run: %w", err)
	}

	wordIdx, err := newWordIdx(ctx, db)
	if err != nil {
		return fmt.Errorf("failed to run: %w", err)
	}

	if err := doNGrams(ctx, db, wordIdx); err != nil {
		return fmt.Errorf("failed to run: %w", err)
	}

	if err := finalize(ctx, db); err != nil {
		return fmt.Errorf("failed to run: %w", err)
	}

	return nil
}

func newDSN() string {
	var filename string
	if *dbFilename != "" {
		filename = *dbFilename
	} else {
		switch *language {
		case "eng":
			filename = "download-" + *language + ".sqlite"
		default:
			log.Fatalf("invalid language: %s", *language)
		}
	}

	return fmt.Sprintf("file:%s?cache=shared", filename)
}

func migrate(ctx context.Context, db *gorm.DB) error {
	dst := []interface{}{
		&entities.FetchedFile{},
		&entities.OneGram{},
		&entities.TwoGram{},
		&entities.ThreeGram{},
		&entities.FourGram{},
		&entities.FiveGram{},
	}

	if err := db.AutoMigrate(dst...); err != nil {
		return fmt.Errorf("failed to migrate: %w", err)
	}

	return nil
}

func doOneGrams(ctx context.Context, db *gorm.DB) error {
	n := 1
	totalFilenum := fileLangNIndex[*language][n]

	var wg sync.WaitGroup

	for idx := 0; idx < totalFilenum; idx++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()

			workerSema <- struct{}{}
			defer func() { <-workerSema }()

			if err := doOneGram(ctx, db, idx); err != nil {
				panic(fmt.Sprintf("failed to do %d-gram %d of %d: %v", n, idx, totalFilenum, err))
			}
		}(idx)
	}

	wg.Wait()

	return nil
}

func doOneGram(ctx context.Context, db *gorm.DB, idx int) error {
	n := 1
	total := fileLangNIndex[*language][n]

	ok, err := isFetched(ctx, db, n, idx)
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	if ok {
		return nil
	}

	gzbody, err := download(ctx, n, idx)
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	gzr, err := gzip.NewReader(bytes.NewReader(gzbody))
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	defer gzr.Close()

	log.Printf("start: parse %d-gram %d of %d", n, idx, total)
	entries, err := newEntries(ctx, gzr, n)
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	log.Printf("end  : parse %d-gram %d of %d", n, idx, total)

	if err := saveOneGrams(ctx, db, entries, idx); err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}

	return nil
}

type Entry struct {
	ngram []string
	score int64
}

func newEntry(s string, n int) (Entry, error) {
	elems := strings.Split(s, "\t")
	if len(elems) < 2 {
		return Entry{}, fmt.Errorf("invalid line: %q", s)
	}

	ngramLine := elems[0]
	yearCounts := elems[1:]

	ngram := strings.Split(ngramLine, " ")
	if len(ngram) != n {
		return Entry{}, fmt.Errorf("ngram length is %d, want %d", len(ngram), n)
	}
	for _, word := range ngram {
		if !isValidWord(word) {
			return Entry{}, fmt.Errorf("invalid ngram: %q", word)
		}
	}

	score, err := calcScore(yearCounts)
	if err != nil {
		return Entry{}, fmt.Errorf("invalid year counts %v: %w", yearCounts, err)
	}

	return Entry{ngram, score}, nil
}

var partOfSpeeches = []string{
	"_NOUN_", "_._", "_VERB_", "_ADP_", "_DET_", "_ADJ_", "_PRON_", "_ADV_", "_NUM_", "_CONJ_", "_PRT_", "_X_",
}

var partOfSpeechSuffixes = []string{
	"_NOUN", "_.", "_VERB", "_ADP", "_DET", "_ADJ", "_PRON", "_ADV", "_NUM", "_CONJ", "_PRT", "_X",
}

func isValidWord(word string) bool {
	if word == "" {
		return false
	}

	for _, pos := range partOfSpeeches {
		if word == pos {
			return false
		}
	}

	for _, suffix := range partOfSpeechSuffixes {
		if strings.HasSuffix(word, suffix) {
			return false
		}
	}

	return true
}

func calcScore(yearCounts []string) (int64, error) {
	var res int64

	for _, line := range yearCounts {
		elems := strings.Split(line, ",")
		if len(elems) != 3 {
			return 0, fmt.Errorf("invalid length of year counts: %q", line)
		}

		matchCount, err := strconv.ParseInt(elems[1], 10, 64)
		if err != nil {
			return 0, fmt.Errorf("failed to parse match_count: %q", line)
		}

		res += matchCount
	}

	return res, nil
}

func newEntries(ctx context.Context, r io.Reader, n int) ([]Entry, error) {
	var res []Entry
	sc := bufio.NewScanner(r)

	for sc.Scan() {
		select {
		case <-ctx.Done():
			return nil, fmt.Errorf("failed to create new one grams: %w", ctx.Err())
		default:
			// NOP
		}

		entry, err := newEntry(sc.Text(), n)
		if err != nil {
			continue
		}
		res = append(res, entry)
	}
	if sc.Err() != nil {
		return res, fmt.Errorf("failed to create new entries: %w", sc.Err())
	}

	return res, nil
}

type IndexEntry struct {
	indexNgram []int64
	score      int64
}

func newIndexEntry(line string, n int, wordIdx *runeInt64Trie) (IndexEntry, error) {
	entry, err := newEntry(line, n)
	if err != nil {
		return IndexEntry{}, fmt.Errorf("failed to create new index entry: %w", err)
	}

	var indexNgram []int64
	for _, word := range entry.ngram {
		idx, ok := wordIdx.Get(word)
		if !ok {
			return IndexEntry{}, fmt.Errorf("word %q not found in one_grams", word)
		}
		indexNgram = append(indexNgram, idx)
	}

	return IndexEntry{indexNgram, entry.score}, nil
}

func newIndexEntries(ctx context.Context, r io.Reader, n int, wordIdx *runeInt64Trie) ([]IndexEntry, error) {
	var res []IndexEntry
	sc := bufio.NewScanner(r)

	for sc.Scan() {
		select {
		case <-ctx.Done():
			return res, fmt.Errorf("faield to create new index entries: %w", ctx.Err())
		default:
			// NOP
		}

		idxEntry, err := newIndexEntry(sc.Text(), n, wordIdx)
		if err != nil {
			continue
		}

		res = append(res, idxEntry)
	}
	if sc.Err() != nil {
		return res, fmt.Errorf("failed to create new index entries: %w", sc.Err())
	}

	return res, nil
}

func newWordIdx(ctx context.Context, db *gorm.DB) (wordIdx *runeInt64Trie, err error) {
	log.Println("start: wordidx")
	defer log.Println("end  : wordidx")

	var oneGrams []entities.OneGram

	res := db.WithContext(ctx).Find(&oneGrams)
	if res.Error != nil {
		return nil, fmt.Errorf("failed to make new word idx: %w", res.Error)
	}

	wordIdx = newRuneInt64Trie()

	for _, oneGram := range oneGrams {
		wordIdx.Put(oneGram.Word, oneGram.ID)
	}

	return
}

func doNGrams(ctx context.Context, db *gorm.DB, wordIdx *runeInt64Trie) error {
	var wg sync.WaitGroup

	for n := 2; n < 5; n++ {
		totalFilenum := fileLangNIndex[*language][n]

		for idx := 0; idx < totalFilenum; idx++ {
			wg.Add(1)
			go func(idx int, totalFilenum int) {
				defer wg.Done()

				workerSema <- struct{}{}
				defer func() { <-workerSema }()

				if err := doNGram(ctx, db, wordIdx, n, idx); err != nil {
					log.Printf("failed to do %d-gram %d of %d: %v", n, idx, totalFilenum, err)
				}
			}(idx, totalFilenum)
		}
	}

	wg.Wait()

	return nil
}

func doNGram(ctx context.Context, db *gorm.DB, wordIdx *runeInt64Trie, n, idx int) error {
	total := fileLangNIndex[*language][n]

	ok, err := isFetched(ctx, db, n, idx)
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	if ok {
		return nil
	}

	gzbody, err := download(ctx, n, idx)
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	gzr, err := gzip.NewReader(bytes.NewReader(gzbody))
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	defer gzr.Close()

	log.Printf("start: parse %d-gram %d of %d", n, idx, total)
	idxEntries, err := newIndexEntries(ctx, gzr, n, wordIdx)
	if err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	log.Printf("end  : parse %d-gram %d of %d", n, idx, total)

	if err := saveNGrams(ctx, db, idxEntries, n, idx); err != nil {
		return fmt.Errorf("failed to do %d-gram %d of %d: %w", n, idx, total, err)
	}
	log.Printf("end  : save %d-gram %d of %d", n, idx, total)

	return nil
}

func finalize(ctx context.Context, db *gorm.DB) error {
	res := db.WithContext(ctx).Raw("vacuum")
	if res.Error != nil {
		return fmt.Errorf("failed to finalize: %w", res.Error)
	}

	return nil
}

func download(ctx context.Context, n, idx int) ([]byte, error) {
	var body []byte
	total := fileLangNIndex[*language][n]
	url := newURL(n, idx)

	dlSema <- struct{}{}
	defer func() { <-dlSema }()

	log.Printf("start: download %d-gram %d of %d", n, idx, total)
	defer log.Printf("end  : download %d-gram %d of %d", n, idx, total)

	op := func() error {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
		if err != nil {
			return err
		}

		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			return err
		}
		defer resp.Body.Close()

		body, err = io.ReadAll(resp.Body)
		if err != nil {
			return err
		}

		return nil
	}

	if err := backoff.Retry(op, backoff.NewExponentialBackOff()); err != nil {
		return nil, fmt.Errorf("failed to download %q: %w", url, err)
	}

	return body, nil
}

func newURL(n, idx int) string {
	return fmt.Sprintf(
		"http://storage.googleapis.com/books/ngrams/books/20200217/%s/%d-%05d-of-%05d.gz",
		*language,
		n,
		idx,
		fileLangNIndex[*language][n],
	)
}

func saveOneGrams(ctx context.Context, db *gorm.DB, entries []Entry, idx int) error {
	n := 1
	total := fileLangNIndex[*language][n]
	err := db.Transaction(func(db *gorm.DB) error {
		log.Printf("start: save %d-gram %d of %d", n, idx, total)
		defer log.Printf("end  : save %d-gram %d of %d", n, idx, total)

		for _, entry := range entries {
			select {
			case <-ctx.Done():
				return fmt.Errorf("failed to save one grams: %w", ctx.Err())
			default:
				// NOP
			}

			val := entities.OneGram{
				Word:  entry.ngram[0],
				Score: entry.score,
			}

			res := db.Create(&val)
			if res.Error != nil {
				return res.Error
			}
		}

		return doFetchedFileFlag(ctx, db, 1, idx)
	})
	if err != nil {
		return fmt.Errorf("failed to save one grams: %w", err)
	}

	return nil
}

func saveNGrams(ctx context.Context, db *gorm.DB, idxEntries []IndexEntry, n, idx int) error {
	return db.Transaction(func(db *gorm.DB) error {
		var err error
		switch n {
		case 2:
			err = saveTwoGrams(ctx, db, idxEntries)
		case 3:
			err = saveThreeGrams(ctx, db, idxEntries)
		case 4:
			err = saveFourGrams(ctx, db, idxEntries)
		case 5:
			err = saveFiveGrams(ctx, db, idxEntries)
		default:
			panic("invalid ngram")
		}
		if err != nil {
			return err
		}

		return doFetchedFileFlag(ctx, db, n, idx)
	})
}

func saveTwoGrams(ctx context.Context, db *gorm.DB, idxEntries []IndexEntry) error {
	for _, ent := range idxEntries {
		select {
		case <-ctx.Done():
			return fmt.Errorf("failed to save two grams: %w", ctx.Err())
		default:
			// NOP
		}

		val := entities.TwoGram{
			Word1ID: ent.indexNgram[0],
			Word2ID: ent.indexNgram[1],
			Score:   ent.score,
		}

		res := db.WithContext(ctx).Create(&val)
		if res.Error != nil {
			return fmt.Errorf("failed to save two grams: %w", res.Error)
		}
	}

	return nil
}

func saveThreeGrams(ctx context.Context, db *gorm.DB, idxEntries []IndexEntry) error {
	for _, ent := range idxEntries {
		select {
		case <-ctx.Done():
			return fmt.Errorf("failed to save three grams: %w", ctx.Err())
		default:
			// NOP
		}

		val := entities.ThreeGram{
			Word1ID: ent.indexNgram[0],
			Word2ID: ent.indexNgram[1],
			Word3ID: ent.indexNgram[2],
			Score:   ent.score,
		}

		res := db.WithContext(ctx).Create(&val)
		if res.Error != nil {
			return fmt.Errorf("failed to save three grams: %w", res.Error)
		}
	}

	return nil
}

func saveFourGrams(ctx context.Context, db *gorm.DB, idxEntries []IndexEntry) error {
	for _, ent := range idxEntries {
		select {
		case <-ctx.Done():
			return fmt.Errorf("failed to save four grams: %w", ctx.Err())
		default:
			// NOP
		}

		val := entities.FourGram{
			Word1ID: ent.indexNgram[0],
			Word2ID: ent.indexNgram[1],
			Word3ID: ent.indexNgram[2],
			Word4ID: ent.indexNgram[3],
			Score:   ent.score,
		}

		res := db.WithContext(ctx).Create(&val)
		if res.Error != nil {
			return fmt.Errorf("failed to save four grams: %w", res.Error)
		}
	}

	return nil
}

func saveFiveGrams(ctx context.Context, db *gorm.DB, idxEntries []IndexEntry) error {
	for _, ent := range idxEntries {
		select {
		case <-ctx.Done():
			return fmt.Errorf("failed to save five grams: %w", ctx.Err())
		default:
			// NOP
		}

		val := entities.FiveGram{
			Word1ID: ent.indexNgram[0],
			Word2ID: ent.indexNgram[1],
			Word3ID: ent.indexNgram[2],
			Word4ID: ent.indexNgram[3],
			Word5ID: ent.indexNgram[4],
			Score:   ent.score,
		}

		res := db.WithContext(ctx).Create(&val)
		if res.Error != nil {
			return fmt.Errorf("failed to save five grams: %w", res.Error)
		}
	}

	return nil
}

func isFetched(ctx context.Context, db *gorm.DB, n, idx int) (bool, error) {
	var fetchedFile entities.FetchedFile

	res := db.WithContext(ctx).Where("n = ?", n).Where("idx = ?", idx).Take(&fetchedFile)
	if res.Error != nil {
		if errors.Is(res.Error, gorm.ErrRecordNotFound) {
			return false, nil
		}
		return false, fmt.Errorf("failed to read fetched_files: %w", res.Error)
	}

	return true, nil
}

func doFetchedFileFlag(ctx context.Context, db *gorm.DB, n, idx int) error {
	val := entities.FetchedFile{N: n, Idx: idx}
	res := db.WithContext(ctx).Create(&val)
	if res.Error != nil {
		return fmt.Errorf("failed to do fetched file flag: %w", res.Error)
	}

	return nil
}
