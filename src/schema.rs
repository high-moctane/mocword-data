table! {
    fetched_files (n, idx) {
        n -> Bigint,
        idx -> Bigint,
    }
}

table! {
    five_grams (id) {
        id -> Bigint,
        prefix_id -> Bigint,
        suffix_id -> Bigint,
    }
}

table! {
    five_gram_scores (prefix_id, suffix_id) {
        prefix_id -> Bigint,
        suffix_id -> Bigint,
        score -> Bigint,
    }
}

table! {
    four_grams (id) {
        id -> Bigint,
        prefix_id -> Bigint,
        suffix_id -> Bigint,
    }
}

table! {
    four_gram_scores (prefix_id, suffix_id) {
        prefix_id -> Bigint,
        suffix_id -> Bigint,
        score -> Bigint,
    }
}

table! {
    one_grams (id) {
        id -> Bigint,
        word -> Text,
    }
}

table! {
    one_gram_scores (word) {
        word -> Text,
        score -> Bigint,
    }
}

table! {
    three_grams (id) {
        id -> Bigint,
        prefix_id -> Bigint,
        suffix_id -> Bigint,
    }
}

table! {
    three_gram_scores (prefix_id, suffix_id) {
        prefix_id -> Bigint,
        suffix_id -> Bigint,
        score -> Bigint,
    }
}

table! {
    two_grams (id) {
        id -> Bigint,
        prefix_id -> Bigint,
        suffix_id -> Bigint,
    }
}

table! {
    two_gram_scores (prefix_id, suffix_id) {
        prefix_id -> Bigint,
        suffix_id -> Bigint,
        score -> Bigint,
    }
}

joinable!(five_grams -> four_grams (prefix_id));
joinable!(five_grams -> one_grams (suffix_id));
joinable!(four_grams -> one_grams (suffix_id));
joinable!(four_grams -> three_grams (prefix_id));
joinable!(three_grams -> one_grams (suffix_id));
joinable!(three_grams -> two_grams (prefix_id));

allow_tables_to_appear_in_same_query!(
    fetched_files,
    five_grams,
    five_gram_scores,
    four_grams,
    four_gram_scores,
    one_grams,
    one_gram_scores,
    three_grams,
    three_gram_scores,
    two_grams,
    two_gram_scores,
);
