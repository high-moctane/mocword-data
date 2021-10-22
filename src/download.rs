use std::collections::{HashMap, HashSet};
use std::io::{prelude::*, BufRead, BufReader};

use anyhow::{Context, Result};
use chrono::Local;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use flate2::bufread::GzDecoder;
use reqwest::blocking;
use thiserror::Error;

use crate::models;
use crate::schema;

#[derive(Debug, Copy, Clone)]
enum Language {
    English,
    AmericanEnglish,
    BritishEnglish,
    EnglishFiction,
    Chinese,
    French,
    German,
    Hebrew,
    Italian,
    Russian,
    Spanish,
}

impl Language {
    fn parse(lang_name: &str) -> Language {
        match &*lang_name.to_lowercase() {
            "eng" => Language::English,
            _ => unimplemented!("not implemented language"),
        }
    }

    fn url_name(&self) -> String {
        match self {
            Language::English => "eng".to_string(),
            _ => unimplemented!("not implemented language"),
        }
    }
}

fn total_file_num(lang: &Language, n: i64) -> i64 {
    match lang {
        Language::English => match n {
            1 => 24,
            2 => 589,
            3 => 6881,
            4 => 6668,
            5 => 19423,
            _ => panic!("invalid ngram number: {}", n),
        },
        _ => unimplemented!("not implemented language: {:?}", lang),
    }
}

fn gz_url(lang: &Language, n: i64, idx: i64) -> String {
    let total = total_file_num(lang, n);

    format!(
        "http://storage.googleapis.com/books/ngrams/books/20200217/{}/{}-{:05}-of-{:05}.gz",
        lang.url_name(),
        n,
        idx,
        total
    )
}

pub fn run() -> Result<()> {
    let conn = SqliteConnection::establish("build/download.sqlite")
        .with_context(|| "failed to establish sqlite connection")?;

    download_all(&conn, &Language::English).with_context(|| format!("failed to download all"))?;

    Ok(())
}

fn download_all(conn: &SqliteConnection, lang: &Language) -> Result<()> {
    let mut all = (1..=5).fold(0, |acc, n| acc + total_file_num(lang, n));

    for n in 1..=5 {
        let total = total_file_num(lang, n);
        for idx in 0..total {
            println!("remains: {}", all);
            all -= 1;

            println!("{} start: {}gram, {} of {}", Local::now(), n, idx, total);

            download(conn, lang, n, idx).with_context(|| {
                format!(
                    "failed to download: {}, n: {}, idx: {}",
                    lang.url_name(),
                    n,
                    idx
                )
            })?;

            println!("{} end:   {}gram, {} of {}", Local::now(), n, idx, total);
        }
    }

    Ok(())
}

fn download(conn: &SqliteConnection, lang: &Language, n: i64, idx: i64) -> Result<()> {
    use schema::fetched_files::dsl;

    let res = dsl::fetched_files
        .filter(dsl::n.eq_all(n))
        .filter(dsl::idx.eq_all(idx))
        .load::<models::FetchedFile>(conn)?;

    if res.len() > 0 {
        return Ok(());
    }

    let url = gz_url(lang, n, idx);

    let mut body = vec![];

    blocking::get(&url)?
        .read_to_end(&mut body)
        .with_context(|| format!("failed to download {}", &url))?;
    let gz = GzDecoder::new(&body[..]);

    let mut data = vec![];

    for line in BufReader::new(gz).lines() {
        data.push(parse_line(&line?).context("failed to parse line")?);
        if data.len() >= 10000 {
            save(conn, &data).context("failed to save data")?;
            data = vec![];
        }
    }
    if data.len() > 0 {
        save(conn, &data).context("failed to save data")?;
    }

    let new_fetched_file = models::NewFetchedFile { n, idx };
    diesel::insert_or_ignore_into(dsl::fetched_files)
        .values(&new_fetched_file)
        .execute(conn)
        .with_context(|| format!("failed to save fetched file: {} {}", n, idx))?;

    Ok(())
}

#[derive(Debug, PartialEq)]
struct Data {
    ngram: Ngram,
    score: i64,
}

#[derive(Debug)]
struct Record {
    ngram: Vec<i64>,
    score: i64,
}

type Ngram = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    year: i64,
    match_count: i64,
    volume_count: i64,
}

fn save(conn: &SqliteConnection, data: &[Data]) -> Result<()> {
    let words: Vec<String> = data.iter().map(|d| d.ngram.clone()).flatten().collect();
    let word_records = save_words(conn, words.as_slice())
        .with_context(|| format!("failed to save words: {:?}", &words))?;

    let mut word_ids = HashMap::new();
    for rec in word_records.iter() {
        word_ids.insert(&rec.word, rec.id.to_owned());
    }

    let records: Vec<Record> = data
        .iter()
        .map(|d| Record {
            ngram: d
                .ngram
                .iter()
                .map(|w| word_ids.get(w).unwrap().to_owned())
                .collect(),
            score: d.score,
        })
        .collect();

    save_records(conn, &records)
        .with_context(|| format!("failed to save records: {:?}", &records))?;

    Ok(())
}

fn save_records(conn: &SqliteConnection, records: &[Record]) -> Result<()> {
    match records[0].ngram.len() {
        1 => save_one_grams(conn, records).context("failed to save records")?,
        2 => save_two_grams(conn, records).context("failed to save records")?,
        3 => save_three_grams(conn, records).context("failed to save records")?,
        4 => save_four_grams(conn, records).context("failed to save records")?,
        5 => save_five_grams(conn, records).context("failed to save records")?,
        _ => panic!("invalid ngram: {:?}", records),
    };

    Ok(())
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("invalid line: {0}")]
    InvalidLine(String),

    #[error("invalid ngram: {0:?}")]
    InvalidNgram(Ngram),

    #[error("invalid entry: {0}")]
    InvalidEntry(String),

    #[error("invalid query: {0}, {1}: {2}")]
    InvalidQuery(i64, i64, String),
}

fn parse_line(line: &str) -> Result<Data> {
    let ngram_entries: Vec<&str> = line.split("\t").collect();
    if ngram_entries.len() < 2 {
        return Err(DownloadError::InvalidLine(line.to_string()))?;
    }

    let ngram = parse_ngram(ngram_entries[0]);
    let entries = parse_entries(&ngram_entries[1..])
        .with_context(|| format!("failed to parse entries: {:?}", &ngram_entries[1..]))?;
    let score = calc_score(&entries);

    Ok(Data { ngram, score })
}

fn parse_ngram(ngram_vec: &str) -> Vec<String> {
    ngram_vec.split(" ").map(|w| w.to_string()).collect()
}

fn parse_entries(entries: &[&str]) -> Result<Vec<Entry>> {
    let mut res = Vec::new();
    for s in entries.iter() {
        res.push(parse_entry(s).with_context(|| format!("failed to parse entry: {:?}", s))?);
    }
    Ok(res)
}

fn parse_entry(entry: &str) -> Result<Entry> {
    let elems: Vec<&str> = entry.split(",").collect();
    if elems.len() != 3 {
        return Err(DownloadError::InvalidEntry(entry.to_string()))?;
    }

    Ok(Entry {
        year: elems[0]
            .parse()
            .with_context(|| format!("failed to parse elems0: {}", elems[0]))?,
        match_count: elems[1]
            .parse()
            .with_context(|| format!("failed to parse elems1: {}", elems[1]))?,
        volume_count: elems[2]
            .parse()
            .with_context(|| format!("failed to parse elems2: {}", elems[2]))?,
    })
}

fn calc_score(entries: &[Entry]) -> i64 {
    entries
        .iter()
        .fold(0, |score, entry| score + entry.match_count)
}

fn save_one_grams(conn: &SqliteConnection, records: &[Record]) -> Result<()> {
    use schema::one_grams::dsl;

    let one_grams: Vec<models::NewOneGram> = records
        .iter()
        .map(|rec| models::NewOneGram {
            word1_id: rec.ngram[0],
            score: rec.score,
        })
        .collect();

    diesel::insert_or_ignore_into(dsl::one_grams)
        .values(&one_grams)
        .execute(conn)
        .with_context(|| format!("failed to save one grams: {:?}", &one_grams))?;

    Ok(())
}

fn save_two_grams(conn: &SqliteConnection, records: &[Record]) -> Result<()> {
    use schema::two_grams::dsl;

    let two_grams: Vec<models::NewTwoGram> = records
        .iter()
        .map(|rec| models::NewTwoGram {
            word1_id: rec.ngram[0],
            word2_id: rec.ngram[1],
            score: rec.score,
        })
        .collect();

    diesel::insert_or_ignore_into(dsl::two_grams)
        .values(&two_grams)
        .execute(conn)
        .with_context(|| format!("failed to save two grams: {:?}", &two_grams))?;

    Ok(())
}

fn save_three_grams(conn: &SqliteConnection, records: &[Record]) -> Result<()> {
    use schema::three_grams::dsl;

    let three_grams: Vec<models::NewThreeGram> = records
        .iter()
        .map(|rec| models::NewThreeGram {
            word1_id: rec.ngram[0],
            word2_id: rec.ngram[1],
            word3_id: rec.ngram[2],
            score: rec.score,
        })
        .collect();

    diesel::insert_or_ignore_into(dsl::three_grams)
        .values(&three_grams)
        .execute(conn)
        .with_context(|| format!("failed to save three grams: {:?}", &three_grams))?;

    Ok(())
}

fn save_four_grams(conn: &SqliteConnection, records: &[Record]) -> Result<()> {
    use schema::four_grams::dsl;

    let four_grams: Vec<models::NewFourGram> = records
        .iter()
        .map(|rec| models::NewFourGram {
            word1_id: rec.ngram[0],
            word2_id: rec.ngram[1],
            word3_id: rec.ngram[2],
            word4_id: rec.ngram[3],
            score: rec.score,
        })
        .collect();

    diesel::insert_or_ignore_into(dsl::four_grams)
        .values(&four_grams)
        .execute(conn)
        .with_context(|| format!("failed to save four grams: {:?}", &four_grams))?;

    Ok(())
}

fn save_five_grams(conn: &SqliteConnection, records: &[Record]) -> Result<()> {
    use schema::five_grams::dsl;

    let five_grams: Vec<models::NewFiveGram> = records
        .iter()
        .map(|rec| models::NewFiveGram {
            word1_id: rec.ngram[0],
            word2_id: rec.ngram[1],
            word3_id: rec.ngram[2],
            word4_id: rec.ngram[3],
            word5_id: rec.ngram[4],
            score: rec.score,
        })
        .collect();

    diesel::insert_or_ignore_into(dsl::five_grams)
        .values(&five_grams)
        .execute(conn)
        .with_context(|| format!("failed to save five grams: {:?}", &five_grams))?;

    Ok(())
}

fn save_words(conn: &SqliteConnection, words: &[String]) -> Result<Vec<models::Word>> {
    use schema::words::dsl;

    let mut unique_words = HashSet::new();

    for word in words.iter() {
        unique_words.insert(word);
    }

    let new_words: Vec<models::NewWord> = unique_words
        .iter()
        .map(|w| models::NewWord {
            word: w.to_string(),
        })
        .collect();

    diesel::insert_or_ignore_into(dsl::words)
        .values(&new_words)
        .execute(conn)
        .with_context(|| format!("failed to save words: {:?}", &new_words))?;

    let mut res = vec![];

    for word in unique_words.iter() {
        let word_record = schema::words::dsl::words
            .filter(dsl::word.eq_all(word.clone()))
            .load::<models::Word>(conn)
            .with_context(|| format!("failed to load words: {:?}", words))?;
        res.push(word_record);
    }

    Ok(res.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        // OK
        let input = "hello world\t2012,195943,849381\t2013,598483,57483\t2014,483584,4731";
        let want_ngram: Vec<String> = vec!["hello".to_string(), "world".to_string()];
        let want_entries = vec![
            Entry {
                year: 2012,
                match_count: 195943,
                volume_count: 849381,
            },
            Entry {
                year: 2013,
                match_count: 598483,
                volume_count: 57483,
            },
            Entry {
                year: 2014,
                match_count: 483584,
                volume_count: 4731,
            },
        ];

        let data = parse_line(&input).unwrap();
        assert_eq!(
            &data,
            &Data {
                ngram: vec!["hello".to_string(), "world".to_string()],
                score: 1278010
            }
        );

        // NG
        assert!(parse_line("hello world 1773,2").is_err());
        assert!(parse_line("hello world 1773,2,5 143").is_err());
    }

    #[test]
    fn test_insert() {
        return;

        let lines = vec![
            "powa\t2012,4,35\t2015,53,165",
            "dousite\t2010,11,31\t2020,61,172",
            "meu powa\t2006,11,30\t2024,61,176",
            "majika meu\t2001,15,38\t2032,54,181",
            "moyasu meu powa\t2005,23,48\t2015,53,165\t2016,65,544",
            "moyasu dousite powa\t2011,17,53\t2027,56,167\t2013,61,546",
            "very moyasu meu powa\t2005,23,48",
            "moctane very moyasu powa\t1999,35,55",
            "very moyasu meu powa nemu\t1434,23,534\t2005,23,48\t1214,534,12",
            "very nemu meu powa nemu\t1440,17,537\t2005,23,48\t1214,534,12",
        ];

        let data: Vec<Data> = lines.iter().map(|l| parse_line(l).unwrap()).collect();
        println!("{:?}", &data);

        let conn = SqliteConnection::establish("build/download.test.sqlite").unwrap();

        save(&conn, &data).unwrap();
    }
}
