use crate::schema::{fetched_files, one_gram_scores};

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
pub struct OneGramScore {
    pub id: i64,
    pub word: String,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "one_gram_scores"]
pub struct NewOneGramScore {
    pub word: String,
    pub score: i64,
}
