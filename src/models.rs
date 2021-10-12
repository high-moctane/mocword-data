use crate::schema::{
    fetched_files, five_gram_entries, five_grams, four_gram_entries, four_grams, one_gram_entries,
    one_grams, three_gram_entries, three_grams, two_gram_entries, two_grams, words,
};

#[derive(Queryable)]
pub struct Word<'a> {
    pub id: i64,
    pub word: &'a str,
}

#[derive(Insertable)]
#[table_name = "words"]
pub struct NewWord<'a> {
    pub word: &'a str,
}

#[derive(Queryable)]
pub struct OneGram {
    pub id: i64,
    pub word1_id: i64,
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

#[derive(Queryable)]
pub struct TwoGram {
    pub id: i64,
    pub word1_id: i64,
}

#[derive(Insertable)]
#[table_name = "two_grams"]
pub struct NewTwoGram {
    pub word1_id: i64,
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

#[derive(Queryable)]
pub struct ThreeGram {
    pub id: i64,
    pub word1_id: i64,
}

#[derive(Insertable)]
#[table_name = "three_grams"]
pub struct NewThreeGram {
    pub word1_id: i64,
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

#[derive(Queryable)]
pub struct FourGram {
    pub id: i64,
    pub word1_id: i64,
}

#[derive(Insertable)]
#[table_name = "four_grams"]
pub struct NewFourGram {
    pub word1_id: i64,
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

#[derive(Queryable)]
pub struct FiveGram {
    pub id: i64,
    pub word1_id: i64,
}

#[derive(Insertable)]
#[table_name = "five_grams"]
pub struct NewFiveGram {
    pub word1_id: i64,
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
