package mocword

import (
	"bufio"
	"compress/gzip"
	"context"
	"errors"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/high-moctane/mocword/model"
	"github.com/high-moctane/mocword/model/dlmodel"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
)

var NMaxIndex = map[int]int{
	1: 24,
	2: 589,
	3: 6881,
	4: 6668,
	5: 19423,
}

func Download(ctx context.Context) error {
	db, err := gorm.Open(sqlite.Open("download.sqlite"), nil)
	if err != nil {
		return fmt.Errorf("download failed: %w", err)
	}
	db = db.WithContext(ctx)

	for n := 1; n <= 5; n++ {
		for idx := 0; idx < NMaxIndex[n]; idx++ {
			if err := DownloadFile(ctx, db, n, idx); err != nil {
				return fmt.Errorf("download failed: %w", err)
			}
		}
	}

	return nil
}

func FileURL(n, idx int) string {
	maxIdx := NMaxIndex[n]
	return fmt.Sprintf(
		"http://storage.googleapis.com/books/ngrams/books/20200217/eng/%d-%05d-of-%05d.gz",
		n,
		idx,
		maxIdx,
	)
}

func DownloadFile(ctx context.Context, db *gorm.DB, n, idx int) error {
	url := FileURL(n, idx)
	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return fmt.Errorf("download file %q failed: %w", url, err)
	}
	client := new(http.Client)

	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("download file %q failed: %w", url, err)
	}
	defer resp.Body.Close()

	bodybuf := bufio.NewReader(resp.Body)
	gz, err := gzip.NewReader(bodybuf)
	if err != nil {
		return fmt.Errorf("download file %q failed: %w", url, err)
	}
	defer gz.Close()
	sc := bufio.NewScanner(gz)

	for sc.Scan() {
		if err := parseLineAndSave(ctx, db, sc.Text()); err != nil {
			return fmt.Errorf("download file %q failed: %w", url, err)
		}
	}
	if sc.Err() != nil {
		return fmt.Errorf("download file %q failed: %w", url, err)
	}

	return nil
}

func parseLineAndSave(ctx context.Context, db *gorm.DB, line string) error {
	lineElem := strings.Split(line, "\t")
	if len(lineElem) != 2 {
		return fmt.Errorf("invalid line len num: %q", line)
	}

	ngram := strings.Split(lineElem[0], " ")
	if len(ngram) < 1 || 5 < len(ngram) {
		return fmt.Errorf("invalid ngram: %q", line)
	}

	entries, err := parseEntries(lineElem[1])
	if err != nil {
		return fmt.Errorf("invalid ngram: %w", err)
	}
	if len(entries) < 1 {
		return fmt.Errorf("empty entries: %q", line)
	}

	ngramModel, err := saveNgram(ctx, db, ngram)
	if err != nil {
		return fmt.Errorf("parseLineAndSave %q failed: %w", line, err)
	}

	if err := saveEntries(ctx, db, ngramModel, entries); err != nil {
		return fmt.Errorf("failed to parseLineAndSave %q: %w", line, err)
	}

	return nil
}

type Entry struct {
	Year        int
	MatchCount  int64
	VolumeCount int64
}

func parseEntries(line string) ([]Entry, error) {
	var res []Entry

	for _, entry := range strings.Split(line, " ") {
		elems := strings.Split(entry, ",")

		if len(elems) != 3 {
			return nil, fmt.Errorf("invalid entry %v", line)
		}

		year, err := strconv.Atoi(elems[0])
		if err != nil {
			return nil, fmt.Errorf("invalid entry %v: %w", line, err)
		}
		matchCount, err := strconv.ParseInt(elems[1], 10, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid entry %v: %w", line, err)
		}
		volumeCount, err := strconv.ParseInt(elems[2], 10, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid entry %v: %w", line, err)
		}

		res = append(res, Entry{year, matchCount, volumeCount})
	}

	return res, nil
}

func saveNgram(ctx context.Context, db *gorm.DB, ngram []string) (ngramModel dlmodel.Ngram, err error) {
	var id int64
	var wordIDs map[string]int64

	for _, word := range ngram {
		id, err = saveWord(ctx, db, word)
		if err != nil {
			err = fmt.Errorf("save ngram %v failed: %w", ngram, err)
			return
		}
		wordIDs[word] = id
	}

	switch len(ngram) {
	case 1:
		og := dlmodel.OneGram{
			Word1ID: wordIDs[ngram[0]],
		}
		ngramModel = og

		err = db.Transaction(func(tx *gorm.DB) error {
			tx.
				Where("word1_id = ?", og.Word1ID).
				Take(&og)

			if err := tx.Error; err != nil {
				if errors.Is(err, gorm.ErrRecordNotFound) {
					tx.Save(&og)
				}
				return err
			}

			return nil
		})

	case 2:
		tg := dlmodel.TwoGram{
			Word1ID: wordIDs[ngram[0]],
			Word2ID: wordIDs[ngram[1]],
		}
		ngramModel = tg

		err = db.Transaction(func(tx *gorm.DB) error {
			tx.
				Where("word1_id = ?", tg.Word1ID).
				Where("word2_id = ?", tg.Word2ID).
				Take(&tg)

			if err := tx.Error; err != nil {
				if errors.Is(err, gorm.ErrRecordNotFound) {
					if err := tx.Save(&tg).Error; err != nil {
						return err
					}
				}
				return err
			}

			return nil
		})

	case 3:
		tg := dlmodel.ThreeGram{
			Word1ID: wordIDs[ngram[0]],
			Word2ID: wordIDs[ngram[1]],
			Word3ID: wordIDs[ngram[2]],
		}
		ngramModel = tg

		err = db.Transaction(func(tx *gorm.DB) error {
			tx.
				Where("word1_id = ?", tg.Word1ID).
				Where("word2_id = ?", tg.Word2ID).
				Where("word3_id = ?", tg.Word3ID).
				Take(&tg)

			if err := tx.Error; err != nil {
				if errors.Is(err, gorm.ErrRecordNotFound) {
					if err := tx.Save(&tg).Error; err != nil {
						return err
					}
				}
				return err
			}

			return nil
		})

	case 4:
		fg := dlmodel.FourGram{
			Word1ID: wordIDs[ngram[0]],
			Word2ID: wordIDs[ngram[1]],
			Word3ID: wordIDs[ngram[2]],
			Word4ID: wordIDs[ngram[3]],
		}
		ngramModel = fg

		err = db.Transaction(func(tx *gorm.DB) error {
			tx.
				Where("word1_id = ?", fg.Word1ID).
				Where("word2_id = ?", fg.Word2ID).
				Where("word3_id = ?", fg.Word3ID).
				Where("word4_id = ?", fg.Word4ID).
				Take(&fg)

			if err := tx.Error; err != nil {
				if errors.Is(err, gorm.ErrRecordNotFound) {
					if err := tx.Save(&fg).Error; err != nil {
						return err
					}
				}
				return err
			}

			return nil
		})

	case 5:
		fg := dlmodel.FiveGram{
			Word1ID: wordIDs[ngram[0]],
			Word2ID: wordIDs[ngram[1]],
			Word3ID: wordIDs[ngram[2]],
			Word4ID: wordIDs[ngram[3]],
			Word5ID: wordIDs[ngram[4]],
		}
		ngramModel = fg

		err = db.Transaction(func(tx *gorm.DB) error {
			tx.
				Where("word1_id = ?", fg.Word1ID).
				Where("word2_id = ?", fg.Word2ID).
				Where("word3_id = ?", fg.Word3ID).
				Where("word4_id = ?", fg.Word4ID).
				Where("word5_id = ?", fg.Word5ID).
				Take(&fg)

			if err := tx.Error; err != nil {
				if errors.Is(err, gorm.ErrRecordNotFound) {
					if err := tx.Save(&fg).Error; err != nil {
						return err
					}
				}
				return err
			}

			return nil
		})

	}
	if err != nil {
		err = fmt.Errorf("save ngram %v failed: %w", ngram, err)
		return
	}

	return
}

func ngramSelect(ctx context.Context, db *gorm.DB, wordIDs map[string]int64) (ngramModel dlmodel.Ngram, err error) {
	switch len(wordIDs) {
	case 1:

	case 2:
	case 3:
	case 4:
	case 5:
	}
}

func saveWord(ctx context.Context, db *gorm.DB, word string) (id int64, err error) {
	var wo dlmodel.Word

	err = db.Transaction(func(tx *gorm.DB) error {
		tx.Where("word = ?", word).Take(&wo)
		if tx.Error != nil {
			if errors.Is(tx.Error, gorm.ErrRecordNotFound) {
				wo.Word = word
				if err := tx.Save(&wo).Error; err != nil {
					return err
				}
			}

			return tx.Error
		}

		return nil
	})
	if err != nil {
		err = fmt.Errorf("failed to save word %q: %w", word, err)
		return
	}

	id = wo.ID
	return
}

func saveEntries(ctx context.Context, db *gorm.DB, ngramModel model.ModelWithID, entries []string) error {
	for _, entry := range entries {
		for _, entElems := range strings.Split(entry, ",") {
			if err := saveEntry(ctx, db, ngramModel, entElems); err != nil {
				return fmt.Errorf("failed to save entries %v: %w", err)
			}
		}
	}

	return nil
}

func saveEntry(ctx context.Context, db *gorm.DB, ngramModel model.ModelWithID, entry []string) error {
	if len(entElems) != 3 {
		return fmt.Errorf("invalid entry len: %d", len(entElems))
	}

	var ent interface{}
	switch ngramModel.(type) {
	case dlmodel.OneGram:
		og := ngramModel.(dlmodel.OneGram)
		ent = dlmodel.OneGramEntry{
			OneGramID:   og.ID,
			Year:        year,
			MatchCount:  matchCount,
			VolumeCount: volumeCount,
		}
	case dlmodel.TwoGram:
		tg := ngramModel.(dlmodel.TwoGram)
		ent = dlmodel.TwoGramEntry{
			TwoGramID:   tg.ID,
			Year:        year,
			MatchCount:  matchCount,
			VolumeCount: volumeCount,
		}
	case dlmodel.ThreeGram:
		tg := ngramModel.(dlmodel.ThreeGram)
		ent = dlmodel.ThreeGramEntry{
			ThreeGramID: tg.ID,
			Year:        year,
			MatchCount:  matchCount,
			VolumeCount: volumeCount,
		}
	case dlmodel.FourGram:
		fg := ngramModel.(dlmodel.FourGram)
		ent = dlmodel.FourGramEntry{
			FourGramID:  fg.ID,
			Year:        year,
			MatchCount:  matchCount,
			VolumeCount: volumeCount,
		}
	case dlmodel.FiveGram:
		fg := ngramModel.(dlmodel.FiveGram)
		ent = dlmodel.FiveGramEntry{
			FiveGramID:  fg.ID,
			Year:        year,
			MatchCount:  matchCount,
			VolumeCount: volumeCount,
		}
	}

	return nil
}
