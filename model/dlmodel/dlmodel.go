package dlmodel

type Word struct {
	ID   int64
	Word string `gorm:"not null;unique"`
}

func (wo Word) GetID() int64 {
	return wo.ID
}

type OneGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;unique"`
	Word1   Word
}

func (og OneGram) GetID() int64 {
	return og.ID
}

type OneGramEntry struct {
	ID          int64
	OneGramID   int64 `gorm:"not null;uniqueIndex:idx_one_gram_entries"`
	OneGram     OneGram
	Year        int   `gorm:"not null;uniqueIndex:idx_one_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (oge OneGramEntry) GetID() int64 {
	return oge.ID
}

type TwoGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;uniqueIndex:idx_two_grams"`
	Word1   Word
	Word2ID int64 `gorm:"not null;uniqueIndex:idx_two_grams"`
	Word2   Word
}

func (tg TwoGram) GetID() int64 {
	return tg.ID
}

type TwoGramEntry struct {
	ID          int64
	TwoGramID   int64 `gorm:"not null;uniqueIndex:idx_two_gram_entries"`
	TwoGram     TwoGram
	Year        int   `gorm:"not null;uniqueIndex:idx_two_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (tge TwoGramEntry) GetID() int64 {
	return tge.ID
}

type ThreeGram struct {
	ID      int64
	Word1ID int64 `gorm:"not null;uniqueIndex:idx_three_grams"`
	Word1   Word
	Word2ID int64 `gorm:"not null;uniqueIndex:idx_three_grams"`
	Word2   Word
	Word3ID int64 `gorm:"not null;uniqueIndex:idx_three_grams"`
	Word3   Word
}

func (tg ThreeGram) GetID() int64 {
	return tg.ID
}

type ThreeGramEntry struct {
	ID          int64
	ThreeGramID int64 `gorm:"not null;uniqueIndex:idx_three_gram_entries"`
	ThreeGram   ThreeGram
	Year        int   `gorm:"not null;uniqueIndex:idx_three_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (tge ThreeGramEntry) GetID() int64 {
	return tge.ID
}

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

func (fg FourGram) GetID() int64 {
	return fg.ID
}

type FourGramEntry struct {
	ID          int64
	FourGramID  int64 `gorm:"not null;uniqueIndex:idx_four_gram_entries"`
	FourGram    FourGram
	Year        int   `gorm:"not null;uniqueIndex:idx_four_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (fge FourGramEntry) GetID() int64 {
	return fge.ID
}

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

func (fg FiveGram) GetID() int64 {
	return fg.ID
}

type FiveGramEntry struct {
	ID          int64
	FiveGramID  int64 `gorm:"not null;uniqueIndex:idx_five_gram_entries"`
	FiveGram    FiveGram
	Year        int   `gorm:"not null;uniqueIndex:idx_five_gram_entries"`
	MatchCount  int64 `gorm:"not null"`
	VolumeCount int64 `gorm:"not null"`
}

func (fge FiveGramEntry) GetID() int64 {
	return fge.ID
}

type Loaded struct {
	N   int `gorm:"not null;primary key"`
	Idx int `gorm:"not null;primary key"`
}
