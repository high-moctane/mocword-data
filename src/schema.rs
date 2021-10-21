table! {
    fetched_files (n, idx) {
        n -> BigInt,
        idx -> BigInt,
    }
}

table! {
    five_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        word2_id -> BigInt,
        word3_id -> BigInt,
        word4_id -> BigInt,
        word5_id -> BigInt,
        score -> BigInt,
    }
}

table! {
    four_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        word2_id -> BigInt,
        word3_id -> BigInt,
        word4_id -> BigInt,
        score -> BigInt,
    }
}

table! {
    one_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        score -> BigInt,
    }
}

table! {
    three_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        word2_id -> BigInt,
        word3_id -> BigInt,
        score -> BigInt,
    }
}

table! {
    two_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        word2_id -> BigInt,
        score -> BigInt,
    }
}

table! {
    words (id) {
        id -> BigInt,
        word -> Text,
    }
}

joinable!(one_grams -> words (word1_id));
joinable!(two_grams -> words (word1_id));
joinable!(three_grams -> words (word1_id));
joinable!(four_grams -> words (word1_id));
joinable!(five_grams -> words (word1_id));

allow_tables_to_appear_in_same_query!(
    fetched_files,
    five_grams,
    four_grams,
    one_grams,
    three_grams,
    two_grams,
    words,
);
