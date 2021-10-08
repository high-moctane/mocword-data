use anyhow::{Context, Result};
use flate2::bufread::GzDecoder;
use std::io::prelude::*;
use std::io::BufReader;

use reqwest;

pub fn download() -> Result<()> {
    for n in 1..=5 {
        for idx in 0..total_files_by_n(n) {
            download_file(n, idx);
        }
    }

    Ok(())
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

fn download_file(n: i8, idx: i16) -> Result<()> {
    let url = file_url(n, idx);

    let resp = reqwest::blocking::get(url)?;
    let br = BufReader::new(resp);
    let mut gz = GzDecoder::new(br);
    let r = BufReader::new(gz);

    for line in r.lines() {
        let line = line?;
        let v: Vec<&str> = line.split("\t").collect();
        assert_eq!(v.len(), 2);
        let ngram = v[0];
        let entries: Vec<Vec<&str>> = v[1]
            .split(" ")
            .map(|entry| entry.split(",").collect())
            .collect();
    }

    Ok(())
}
