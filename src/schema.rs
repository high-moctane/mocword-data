table! {
    fetched_data (n, idx) {
        n -> Integer,
        idx -> Integer,
    }
}

table! {
    five_gram_entries (id) {
        id -> BigInt,
        five_gram_id -> BigInt,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
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
    }
}

table! {
    four_gram_entries (id) {
        id -> BigInt,
        four_gram_id -> BigInt,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    four_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        word2_id -> BigInt,
        word3_id -> BigInt,
        word4_id -> BigInt,
    }
}

table! {
    one_gram_entries (id) {
        id -> BigInt,
        one_gram_id -> BigInt,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    one_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
    }
}

table! {
    three_gram_entries (id) {
        id -> BigInt,
        three_gram_id -> BigInt,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    three_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        word2_id -> BigInt,
        word3_id -> BigInt,
    }
}

table! {
    two_gram_entries (id) {
        id -> BigInt,
        two_gram_id -> BigInt,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    two_grams (id) {
        id -> BigInt,
        word1_id -> BigInt,
        word2_id -> BigInt,
    }
}

table! {
    words (id) {
        id -> BigInt,
        word -> Text,
    }
}

joinable!(five_gram_entries -> five_grams (five_gram_id));
joinable!(four_gram_entries -> four_grams (four_gram_id));
joinable!(one_gram_entries -> one_grams (one_gram_id));
joinable!(one_grams -> words (word1_id));
joinable!(three_gram_entries -> three_grams (three_gram_id));
joinable!(two_gram_entries -> two_grams (two_gram_id));

allow_tables_to_appear_in_same_query!(
    fetched_data,
    five_gram_entries,
    five_grams,
    four_gram_entries,
    four_grams,
    one_gram_entries,
    one_grams,
    three_gram_entries,
    three_grams,
    two_gram_entries,
    two_grams,
    words,
);
