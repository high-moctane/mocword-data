table! {
    fetched_files (n, idx) {
        n -> BigInt,
        idx -> BigInt,
    }
}

table! {
    five_grams (word1, word2, word3, word4, word5) {
        word1 -> BigInt,
        word2 -> BigInt,
        word3 -> BigInt,
        word4 -> BigInt,
        word5 -> BigInt,
        score -> BigInt,
    }
}

table! {
    four_grams (word1, word2, word3, word4) {
        word1 -> BigInt,
        word2 -> BigInt,
        word3 -> BigInt,
        word4 -> BigInt,
        score -> BigInt,
    }
}

table! {
    one_grams (id) {
        id -> BigInt,
        word -> Text,
        score -> BigInt,
    }
}

table! {
    three_grams (word1, word2, word3) {
        word1 -> BigInt,
        word2 -> BigInt,
        word3 -> BigInt,
        score -> BigInt,
    }
}

table! {
    two_grams (word1, word2) {
        word1 -> BigInt,
        word2 -> BigInt,
        score -> BigInt,
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
