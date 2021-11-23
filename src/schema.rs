table! {
    fetched_files (n, idx) {
        n -> Integer,
        idx -> Integer,
    }
}

table! {
    five_grams (word1, word2, word3, word4, word5) {
        word1 -> Integer,
        word2 -> Integer,
        word3 -> Integer,
        word4 -> Integer,
        word5 -> Integer,
        score -> Bigint,
    }
}

table! {
    four_grams (word1, word2, word3, word4) {
        word1 -> Integer,
        word2 -> Integer,
        word3 -> Integer,
        word4 -> Integer,
        score -> Bigint,
    }
}

table! {
    one_grams (id) {
        id -> Integer,
        word -> Text,
        score -> Bigint,
    }
}

table! {
    three_grams (word1, word2, word3) {
        word1 -> Integer,
        word2 -> Integer,
        word3 -> Integer,
        score -> Bigint,
    }
}

table! {
    two_grams (word1, word2) {
        word1 -> Integer,
        word2 -> Integer,
        score -> Bigint,
    }
}

allow_tables_to_appear_in_same_query!(
    fetched_files,
    five_grams,
    four_grams,
    one_grams,
    three_grams,
    two_grams,
);
