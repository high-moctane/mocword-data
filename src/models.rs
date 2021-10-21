use crate::schema::{
    fetched_files, five_grams, four_grams, one_grams, three_grams, two_grams, words,
};

pub trait Ngram {
    fn n(&self) -> i64;
    fn get_id(&self) -> i64;
}

#[derive(Queryable)]
pub struct Word {
    pub id: i64,
    pub word: String,
}

#[derive(Insertable)]
#[table_name = "words"]
pub struct NewWord {
    pub word: String,
}

#[derive(Queryable, Copy, Clone)]
pub struct OneGram {
    pub id: i64,
    pub word1_id: i64,
    pub score: i64,
}

impl Ngram for OneGram {
    fn n(&self) -> i64 {
        1
    }

    fn get_id(&self) -> i64 {
        self.id
    }
}

#[derive(Insertable)]
#[table_name = "one_grams"]
pub struct NewOneGram {
    pub word1_id: i64,
    pub score: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct TwoGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
    pub score: i64,
}

impl Ngram for TwoGram {
    fn n(&self) -> i64 {
        2
    }

    fn get_id(&self) -> i64 {
        self.id
    }
}

#[derive(Insertable)]
#[table_name = "two_grams"]
pub struct NewTwoGram {
    pub word1_id: i64,
    pub word2_id: i64,
    pub score: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct ThreeGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub score: i64,
}

impl Ngram for ThreeGram {
    fn n(&self) -> i64 {
        3
    }

    fn get_id(&self) -> i64 {
        self.id
    }
}

#[derive(Insertable)]
#[table_name = "three_grams"]
pub struct NewThreeGram {
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub score: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct FourGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub word4_id: i64,
    pub score: i64,
}

impl Ngram for FourGram {
    fn n(&self) -> i64 {
        4
    }

    fn get_id(&self) -> i64 {
        self.id
    }
}

#[derive(Insertable)]
#[table_name = "four_grams"]
pub struct NewFourGram {
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub word4_id: i64,
    pub score: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct FiveGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub word4_id: i64,
    pub word5_id: i64,
    pub score: i64,
}

impl Ngram for FiveGram {
    fn n(&self) -> i64 {
        5
    }

    fn get_id(&self) -> i64 {
        self.id
    }
}

#[derive(Insertable)]
#[table_name = "five_grams"]
pub struct NewFiveGram {
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub word4_id: i64,
    pub word5_id: i64,
    pub score: i64,
}

#[derive(Queryable)]
pub struct FetchedFile {
    pub n: i64,
    pub idx: i64,
}

#[derive(Insertable)]
#[table_name = "fetched_files"]
pub struct NewFetchedFile {
    pub n: i64,
    pub idx: i64,
}
