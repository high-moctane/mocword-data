use anyhow::{Context, Result};
use env_logger;
use log::{info, trace, warn};
use thiserror::Error;

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

#[derive(Debug, PartialEq)]
struct Data {
    ngram: Ngram,
    score: i64,
}

type Ngram = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    year: i64,
    match_count: i64,
    volume_count: i64,
}

pub fn run() -> Result<()> {
    env_logger::init();

    info!("info message");
    trace!("trace message");
    warn!("warn message");
    println!("println");
    Ok(())
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
