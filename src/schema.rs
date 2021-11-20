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
        score -> Bigint,
    }
}

table! {
    four_grams (id) {
        id -> Bigint,
        prefix_id -> Bigint,
        suffix_id -> Bigint,
        score -> Bigint,
    }
}

table! {
    one_grams (id) {
        id -> Bigint,
        word -> Text,
        score -> Bigint,
    }
}

table! {
    three_grams (id) {
        id -> Bigint,
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
