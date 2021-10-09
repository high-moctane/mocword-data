use anyhow::{Context, Result};
use flate2::bufread::GzDecoder;
use std::io::prelude::*;
use std::io::BufReader;

use reqwest;

use diesel;
use diesel::prelude::*;

use crate::models;

use crate::schema;
use std::collections::HashMap;

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
    let conn = SqliteConnection::establish("download.sql")?;

    let url = file_url(n, idx);

    let resp = reqwest::blocking::get(url)?;
    let br = BufReader::new(resp);
    let gz = GzDecoder::new(br);
    let r = BufReader::new(gz);

    r.lines()
        .try_for_each(|line| parse_line_and_insert(&conn, &line?))?;

    Ok(())
}

fn parse_line_and_insert(conn: &SqliteConnection, line: &str) -> Result<()> {
    let v: Vec<&str> = line.split("\t").collect();
    assert_eq!(v.len(), 2);
    let ngram: Vec<&str> = v[0].split(" ").collect();
    let entries: Vec<Vec<&str>> = v[1]
        .split(" ")
        .map(|entry| entry.split(",").collect())
        .collect();

    let mut word_ids = HashMap::<&str, i64>::new();

    for word in ngram.iter() {
        let w = models::NewWord { word };

        diesel::insert_into(schema::words::table)
            .values(&w)
            .execute(conn)?;

        use crate::schema::words::dsl;
        let res = dsl::words
            .filter(dsl::word.eq_all(word))
            .limit(1)
            .load::<models::Word>(conn)?;

        word_ids.insert(word, res[0].id);
    }

    for entry in entries.iter() {
        assert_eq!(entry.len(), 3);
        let year: i16 = entry[0].parse()?;
        let match_count: i64 = entry[1].parse()?;
        let volume_count: i64 = entry[2].parse()?;

        let ent = models:
    }

    Ok(())
}
