use super::schema::one_gram_entries;
use super::schema::one_grams;
use super::schema::words;

#[derive(Queryable)]
pub struct Word {
    pub id: i64,
    pub word: String,
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
pub struct NewOngGram {
    pub word1_id: i64,
}

#[derive(Queryable)]
pub struct OneGramEntries {
    pub id: i64,
    pub one_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Insertable)]
#[table_name = "one_gram_entries"]
pub struct NewOngGramEntry {
    pub one_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
    pub volume_count: i64,
}

#[derive(Queryable)]
pub struct TwoGram {
    pub id: i64,
    pub word1: i64,
    pub word2: i64,
}

#[derive(Queryable)]
pub struct TwoGramEntries {
    pub id: i64,
    pub two_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
}

#[derive(Queryable)]
pub struct ThreeGram {
    pub id: i64,
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
}

#[derive(Queryable)]
pub struct ThreeGramEntries {
    pub id: i64,
    pub three_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
}

#[derive(Queryable)]
pub struct FourGram {
    pub id: i64,
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub word4: i64,
}

#[derive(Queryable)]
pub struct FourGramEntries {
    pub id: i64,
    pub four_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
}

#[derive(Queryable)]
pub struct FiveGram {
    pub id: i64,
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub word4: i64,
    pub word5: i64,
}

#[derive(Queryable)]
pub struct FiveGramEntries {
    pub id: i64,
    pub five_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
}

#[derive(Queryable)]
pub struct FetchedFile {
    pub n: i8,
    pub idx: i16,
}
