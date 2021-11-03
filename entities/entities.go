package entities

type OneGram struct {
	ID    int64
	Word  string
	Score int64
}

type TwoGram struct {
	ID      int64
	Word1ID int64
	Word2ID int64
	Score   int64
}

type ThreeGram struct {
	ID      int64
	Word1ID int64
	Word2ID int64
	Word3ID int64
	Score   int64
}

type FourGram struct {
	ID      int64
	Word1ID int64
	Word2ID int64
	Word3ID int64
	Word4ID int64
	Score   int64
}

type FiveGram struct {
	ID      int64
	Word1ID int64
	Word2ID int64
	Word3ID int64
	Word4ID int64
	Word5ID int64
	Score   int64
}

type FetchedFile struct {
	N   int `gorm:"primaryKey"`
	Idx int `gorm:"primaryKey"`
}
