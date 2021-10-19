use crate::schema::{
    fetched_files, five_gram_entries, five_grams, four_gram_entries, four_grams, one_gram_entries,
    one_grams, three_gram_entries, three_grams, two_gram_entries, two_grams, words,
};

pub trait Ngram {
    fn n(&self) -> i16;
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
}

impl Ngram for OneGram {
    fn n(&self) -> i16 {
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
}

#[derive(Queryable)]
pub struct OneGramEntry {
    pub one_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Insertable)]
#[table_name = "one_gram_entries"]
pub struct NewOneGramEntry {
    pub one_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct TwoGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
}

impl Ngram for TwoGram {
    fn n(&self) -> i16 {
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
}

#[derive(Queryable)]
pub struct TwoGramEntry {
    pub two_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Insertable)]
#[table_name = "two_gram_entries"]
pub struct NewTwoGramEntry {
    pub two_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct ThreeGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
}

impl Ngram for ThreeGram {
    fn n(&self) -> i16 {
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
}

#[derive(Queryable)]
pub struct ThreeGramEntry {
    pub three_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Insertable)]
#[table_name = "three_gram_entries"]
pub struct NewThreeGramEntry {
    pub three_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct FourGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub word4_id: i64,
}

impl Ngram for FourGram {
    fn n(&self) -> i16 {
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
}

#[derive(Queryable)]
pub struct FourGramEntry {
    pub four_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Insertable)]
#[table_name = "four_gram_entries"]
pub struct NewFourGramEntry {
    pub four_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Queryable, Copy, Clone)]
pub struct FiveGram {
    pub id: i64,
    pub word1_id: i64,
    pub word2_id: i64,
    pub word3_id: i64,
    pub word4_id: i64,
    pub word5_id: i64,
}

impl Ngram for FiveGram {
    fn n(&self) -> i16 {
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
}

#[derive(Queryable)]
pub struct FiveGramEntry {
    pub five_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Insertable)]
#[table_name = "five_gram_entries"]
pub struct NewFiveGramEntry {
    pub five_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Queryable)]
pub struct FetchedFile {
    pub n: i16,
    pub idx: i16,
}

#[derive(Insertable)]
#[table_name = "fetched_files"]
pub struct NewFetchedFile {
    pub n: i16,
    pub idx: i16,
}
