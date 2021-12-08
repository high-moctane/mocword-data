use crate::schema::{fetched_files, five_grams, four_grams, one_grams, three_grams, two_grams};

#[derive(Queryable, Clone)]
pub struct FetchedFile {
    pub n: i64,
    pub idx: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "fetched_files"]
pub struct NewFetchedFile {
    pub n: i64,
    pub idx: i64,
}

#[derive(Queryable, Clone)]
pub struct OneGram {
    pub id: i64,
    pub word: String,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "one_grams"]
pub struct NewOneGram {
    pub word: String,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct TwoGram {
    pub word1: i64,
    pub word2: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "two_grams"]
pub struct NewTwoGram {
    pub word1: i64,
    pub word2: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct ThreeGram {
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "three_grams"]
pub struct NewThreeGram {
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct FourGram {
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub word4: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "four_grams"]
pub struct NewFourGram {
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub word4: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct FiveGram {
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub word4: i64,
    pub word5: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "five_grams"]
pub struct NewFiveGram {
    pub word1: i64,
    pub word2: i64,
    pub word3: i64,
    pub word4: i64,
    pub word5: i64,
    pub score: i64,
}
