table! {
    fetched_data (n, idx) {
        n -> Integer,
        idx -> Integer,
    }
}

table! {
    five_gram_entries (id) {
        id -> Integer,
        five_gram_id -> Integer,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    five_grams (id) {
        id -> Integer,
        word1_id -> Integer,
        word2_id -> Integer,
        word3_id -> Integer,
        word4_id -> Integer,
        word5_id -> Integer,
    }
}

table! {
    four_gram_entries (id) {
        id -> Integer,
        four_gram_id -> Integer,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    four_grams (id) {
        id -> Integer,
        word1_id -> Integer,
        word2_id -> Integer,
        word3_id -> Integer,
        word4_id -> Integer,
    }
}

table! {
    one_gram_entries (id) {
        id -> Integer,
        one_gram_id -> Integer,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    one_grams (id) {
        id -> Integer,
        word1_id -> Integer,
    }
}

table! {
    three_gram_entries (id) {
        id -> Integer,
        three_gram_id -> Integer,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    three_grams (id) {
        id -> Integer,
        word1_id -> Integer,
        word2_id -> Integer,
        word3_id -> Integer,
    }
}

table! {
    two_gram_entries (id) {
        id -> Integer,
        two_gram_id -> Integer,
        year -> Integer,
        match_count -> Integer,
        volume_count -> Integer,
    }
}

table! {
    two_grams (id) {
        id -> Integer,
        word1_id -> Integer,
        word2_id -> Integer,
    }
}

table! {
    words (id) {
        id -> Integer,
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
