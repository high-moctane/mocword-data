pub fn download() {
    for n in 1..=5 {
        for idx in 0..total_files_by_n(n) {
            println!("{}", file_url(n, idx));
        }
    }
}

fn total_files_by_n(n: i8) -> i16 {
    match n {
        1 => 24,
        2 => 589,
        3 => 6881,
        4 => 6668,
        5 => 19423,
        _ => panic!("unexpected n: {}", n),
    }
}

fn file_url(n: i8, idx: i16) -> String {
    let max_idx = total_files_by_n(n);
    assert!(0 <= idx && idx < max_idx);
    format!(
        "http://storage.googleapis.com/books/ngrams/books/20200217/eng/{}-{:05}-of-{:05}.gz",
        n, idx, max_idx
    )
}
