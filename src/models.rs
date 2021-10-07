#[derive(Queryable)]
pub struct Word {
    pub id: i64,
    pub word: String,
}

#[derive(Queryable)]
pub struct OneGram {
    pub id: i64,
    pub word1: i64,
}

#[derive(Queryable)]
pub struct OneGramEntries {
    pub id: i64,
    pub one_gram_id: i64,
    pub year: i16,
    pub match_count: i64,
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
