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
	ID    int64
	Word1 int64
	Word2 int64
	Score int64
}

type ThreeGramRecord struct {
	ID    int64
	Word1 int64
	Word2 int64
	Word3 int64
	Score int64
}

type FourGramRecord struct {
	ID    int64
	Word1 int64
	Word2 int64
	Word3 int64
	Word4 int64
	Score int64
}

type FiveGramRecord struct {
	ID    int64
	Word1 int64
	Word2 int64
	Word3 int64
	Word4 int64
	Word5 int64
	Score int64
}
