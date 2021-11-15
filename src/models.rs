use crate::schema::{
    fetched_files, five_gram_scores, five_grams, four_gram_scores, four_grams, one_gram_scores,
    one_grams, three_gram_scores, three_grams, two_gram_scores, two_grams,
};

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
pub struct TwoGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "two_gram_scores"]
pub struct NewTwoGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct TwoGram {
    pub id: i64,
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "two_grams"]
pub struct NewTwoGram {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct ThreeGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "three_gram_scores"]
pub struct NewThreeGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct ThreeGram {
    pub id: i64,
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "three_grams"]
pub struct NewThreeGram {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct FourGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "four_gram_scores"]
pub struct NewFourGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct FourGram {
    pub id: i64,
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "four_grams"]
pub struct NewFourGram {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct FiveGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "five_gram_scores"]
pub struct NewFiveGramScore {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Queryable, Clone)]
pub struct FiveGram {
    pub id: i64,
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}

#[derive(Insertable, Debug)]
#[table_name = "five_grams"]
pub struct NewFiveGram {
    pub prefix_id: i64,
    pub suffix_id: i64,
    pub score: i64,
}
