package dlmodel

type Ngram interface {
	ngram()
}

type Entry interface {
	entry()
}

type Word struct {
	ID   int64
	Word string `gorm:"not null;unique"`
}

type OneGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;unique"`
	Word1   Word
}

func (OneGram) ngram() {}

type OneGramEntry struct {
	ID          int64
	OneGramID   int64 `gorm:"not null;uniqueIndex:idx_one_gram_entries"`
	OneGram     OneGram
	Year        int   `gorm:"not null;uniqueIndex:idx_one_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (OneGramEntry) entry() {}

type TwoGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;uniqueIndex:idx_two_grams"`
	Word1   Word
	Word2ID int64 `gorm:"not null;uniqueIndex:idx_two_grams"`
	Word2   Word
}

func (TwoGram) ngram() {}

type TwoGramEntry struct {
	ID          int64
	TwoGramID   int64 `gorm:"not null;uniqueIndex:idx_two_gram_entries"`
	TwoGram     TwoGram
	Year        int   `gorm:"not null;uniqueIndex:idx_two_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (TwoGramEntry) entry() {}

type ThreeGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;uniqueIndex:idx_three_grams"`
	Word1   Word
	Word2ID int64 `gorm:"not null;uniqueIndex:idx_three_grams"`
	Word2   Word
	Word3ID int64 `gorm:"not null;uniqueIndex:idx_three_grams"`
	Word3   Word
}

func (ThreeGram) ngram() {}

type ThreeGramEntry struct {
	ID          int64
	ThreeGramID int64 `gorm:"not null;uniqueIndex:idx_three_gram_entries"`
	ThreeGram   ThreeGram
	Year        int   `gorm:"not null;uniqueIndex:idx_three_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (ThreeGramEntry) entry() {}

type FourGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;uniqueIndex:idx_four_grams"`
	Word1   Word
	Word2ID int64 `gorm:"not null;uniqueIndex:idx_four_grams"`
	Word2   Word
	Word3ID int64 `gorm:"not null;uniqueIndex:idx_four_grams"`
	Word3   Word
	Word4ID int64 `gorm:"not null;uniqueIndex:idx_four_grams"`
	Word4   Word
}

func (FourGram) ngram() {}

type FourGramEntry struct {
	ID          int64
	FourGramID  int64 `gorm:"not null;uniqueIndex:idx_four_gram_entries"`
	FourGram    FourGram
	Year        int   `gorm:"not null;uniqueIndex:idx_four_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (FourGramEntry) entry() {}

type FiveGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;uniqueIndex:idx_five_grams"`
	Word1   Word
	Word2ID int64 `gorm:"not null;uniqueIndex:idx_five_grams"`
	Word2   Word
	Word3ID int64 `gorm:"not null;uniqueIndex:idx_five_grams"`
	Word3   Word
	Word4ID int64 `gorm:"not null;uniqueIndex:idx_five_grams"`
	Word4   Word
	Word5ID int64 `gorm:"not null;uniqueIndex:idx_five_grams"`
	Word5   Word
}

func (FiveGram) ngram() {}

type FiveGramEntry struct {
	ID          int64
	FiveGramID  int64 `gorm:"not null;uniqueIndex:idx_five_gram_entries"`
	FiveGram    FiveGram
	Year        int   `gorm:"not null;uniqueIndex:idx_five_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (FiveGramEntry) entry() {}

type Downloaded struct {
	N   int `gorm:"not null;primary key"`
	Idx int `gorm:"not null;primary key"`
}
