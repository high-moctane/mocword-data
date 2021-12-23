package mocword

type Query struct {
	N   int `gorm:"primaryKey"`
	Idx int `gorm:"primaryKey"`
}

type OneGramRecord struct {
	ID    int64
	Word  string
	Score int64
}

type TwoGramRecord struct {
	Word1 int64 `gorm:"primaryKey"`
	Word2 int64 `gorm:"primaryKey"`
	Score int64
}

type ThreeGramRecord struct {
	Word1 int64 `gorm:"primaryKey"`
	Word2 int64 `gorm:"primaryKey"`
	Word3 int64 `gorm:"primaryKey"`
	Score int64
}

type FourGramRecord struct {
	Word1 int64 `gorm:"primaryKey"`
	Word2 int64 `gorm:"primaryKey"`
	Word3 int64 `gorm:"primaryKey"`
	Word4 int64 `gorm:"primaryKey"`
	Score int64
}

type FiveGramRecord struct {
	Word1 int64 `gorm:"primaryKey"`
	Word2 int64 `gorm:"primaryKey"`
	Word3 int64 `gorm:"primaryKey"`
	Word4 int64 `gorm:"primaryKey"`
	Word5 int64 `gorm:"primaryKey"`
	Score int64
}
