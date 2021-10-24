use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};
use crossbeam::channel;
use diesel::{self, prelude::*, result, SqliteConnection};
use env_logger;
use flate2::bufread::GzDecoder;
use log::{info, trace, warn};
use reqwest::blocking;
use thiserror::Error;
use threadpool::ThreadPool;

use crate::models;
use crate::schema;

static DST_DIR: &str = "build";
static WORKER_NUM: usize = 4;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

    #[error("failed to copy from {0} to {1}: {2}")]
    CopyError(String, String, String),
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

#[derive(Debug, PartialEq, Eq)]
struct Query {
    lang: Language,
    n: i64,
    idx: i64,
}

pub fn run() -> Result<()> {
    env_logger::init();

    let lang = Language::English;

    download_all(&lang).with_context(|| format!("failed to download all: {:?}", &lang))?;

    Ok(())
}

fn download_all(lang: &Language) -> Result<()> {
    // 1-gram
    let sqlite_one_gram = format!("{}/download.sqlite", DST_DIR);
    download_one_grams(&sqlite_one_gram, lang)
        .with_context(|| format!("failed to download one gram to {}", &sqlite_one_gram))?;

    // 2-gram
    let filenames = db_clone(&sqlite_one_gram, WORKER_NUM)
        .with_context(|| format!("failed to clone {}", &sqlite_one_gram))?;

    let pool = ThreadPool::new(WORKER_NUM);
    let (tx, rx): (channel::Sender<Query>, channel::Receiver<Query>) = channel::unbounded();
    let n_sentinel = 10000;
    for filename in filenames.into_iter() {
        let rx = rx.clone();
        pool.execute(move || download_ngrams(&filename, &rx).expect("failed to download_ngrams"));
    }

    for n in 2..=5 {
        for idx in 0..total_file_num(lang, n) {
            tx.send(Query {
                lang: lang.clone().to_owned(),
                n,
                idx,
            })?;
        }
    }
    for _ in 0..WORKER_NUM {
        tx.send(Query {
            lang: lang.clone().to_owned(),
            n: n_sentinel,
            idx: 0,
        })?;
    }

    pool.join();

    Ok(())
}

fn download_ngrams(filename: &str, rx: &channel::Receiver<Query>) -> Result<()> {
    let conn = SqliteConnection::establish(filename)
        .with_context(|| format!("failed to connect {}", filename))?;

    for query in rx.iter() {
        if query.n > 5 {
            return Ok(());
        }
        conn.transaction(|| {
            download_ngram(&conn, &query)
                .with_context(|| format!("failed to download ngram {:?}", query))
        })
        .with_context(|| format!("failed to transaction {:?}", &query))?;
    }

    panic!("unexpected drop of rx");
}

fn download_ngram(conn: &SqliteConnection, query: &Query) -> Result<()> {
    let lang = &query.lang;
    let n = query.n;
    let idx = query.idx;

    let total = total_file_num(lang, n);
    info!("start: {}-gram {} of {}", n, idx, total);

    if is_already_downloaded(&conn, n, idx)? {
        info!("already downloaded {}gram {} of {}", n, idx, total);
        return Ok(());
    }

    // Download
    let url = gz_url(lang, n, idx);
    let mut body = vec![];
    blocking::get(&url)?
        .read_to_end(&mut body)
        .with_context(|| format!("failed to download {}", &url))?;

    // Parsing
    let all_data = parse_body(body).context("failed to parse body")?;

    // Saving
    save_ngrams(&conn, n, all_data).with_context(|| format!("failed to save {} {}", n, idx))?;

    // Flagging
    save_flag(&conn, n, idx).context("failed to save flag")?;

    info!("end  : {}-gram {} of {}", n, idx, total);

    Ok(())
}

fn save_ngrams(conn: &SqliteConnection, n: i64, all_data: Vec<Data>) -> Result<()> {
    unimplemented!();
}

fn download_one_grams(filename: &str, lang: &Language) -> Result<()> {
    trace!("download one gram");

    let conn = SqliteConnection::establish(filename)
        .with_context(|| format!("failed to connect {}", filename))?;

    // save
    let n = 1;
    for idx in 0..total_file_num(lang, n) {
        download_one_gram(&conn, lang, idx)
            .with_context(|| format!("failed to download one gram {}", idx))?;
    }

    // finalize
    finalize_one_gram(&conn).with_context(|| format!("failed to finalize {}", filename))?;

    Ok(())
}

fn finalize_one_gram(conn: &SqliteConnection) -> Result<()> {
    info!("start: indexing");
    diesel::sql_query("create unique index idx_one_grams_word on one_grams(word)")
        .execute(conn)
        .context("failed to create unique index")?;
    info!("end  : indexing");

    info!("start: vacuum");
    diesel::sql_query("vacuum")
        .execute(conn)
        .context("failed to vacuum")?;
    info!("end  : vacuum");

    Ok(())
}

fn download_one_gram(conn: &SqliteConnection, lang: &Language, idx: i64) -> Result<()> {
    let n = 1;
    let total = total_file_num(lang, n);
    info!("start: 1-gram {} of {}", idx, total);

    if is_already_downloaded(&conn, n, idx)? {
        info!(
            "already downloaded {}gram {} of {}",
            n,
            idx,
            total_file_num(lang, n)
        );
        return Ok(());
    }

    // Download
    let url = gz_url(lang, n, idx);
    let mut body = vec![];
    blocking::get(&url)?
        .read_to_end(&mut body)
        .with_context(|| format!("failed to download {}", &url))?;

    // Parsing
    let all_data = parse_body(body).context("failed to parse body")?;

    // Saving
    save_one_grams(&conn, all_data).with_context(|| format!("failed to save {} {}", n, idx))?;

    // Flagging
    save_flag(&conn, n, idx).context("failed to save flag")?;

    info!("end  : 1-gram {} of {}", idx, total);

    Ok(())
}

fn save_flag(conn: &SqliteConnection, n: i64, idx: i64) -> Result<()> {
    use schema::fetched_files::dsl;

    let new_flag = models::NewFetchedFile { n, idx };

    diesel::insert_into(dsl::fetched_files)
        .values(&new_flag)
        .execute(conn)
        .with_context(|| format!("failed to save flag {} {}", n, idx))?;

    Ok(())
}

fn save_one_grams(conn: &SqliteConnection, all_data: Vec<Data>) -> Result<()> {
    use schema::one_grams::dsl;

    let new_one_grams: Vec<models::NewOneGram> = all_data
        .iter()
        .map(|d| models::NewOneGram {
            word: d.ngram[0].to_owned(),
            score: d.score,
        })
        .collect();

    diesel::insert_into(dsl::one_grams)
        .values(&new_one_grams)
        .execute(conn)
        .context("failed to save one grams")?;

    Ok(())
}

fn parse_body(body: Vec<u8>) -> Result<Vec<Data>> {
    let gz = GzDecoder::new(&body[..]);
    let mut data = vec![];

    for line in BufReader::new(gz).lines() {
        let line = line?;
        data.push(parse_line(&line).with_context(|| format!("failed to parse line: {}", &line))?);
    }

    Ok(data)
}

fn is_already_downloaded(conn: &SqliteConnection, n: i64, idx: i64) -> Result<bool> {
    use schema::fetched_files::dsl;

    let res = dsl::fetched_files
        .filter(dsl::n.eq_all(n))
        .filter(dsl::idx.eq_all(idx))
        .load::<models::FetchedFile>(conn)
        .with_context(|| format!("failed to load fetched_file: n({}), idx({})", n, idx))?;

    Ok(res.len() > 0)
}

fn db_clone(src: &str, num: usize) -> Result<Vec<String>> {
    let filenames: Vec<String> = (0..num)
        .map(|i| format!("{}/download-{}.sqlite", DST_DIR, i))
        .collect();

    for filename in filenames.iter() {
        if Path::new(filename).exists() {
            trace!("{} already exists", filename);
            continue;
        }

        if let Err(e) = fs::copy(src, filename) {
            return Err(DownloadError::CopyError(
                src.to_string(),
                filename.to_string(),
                e.to_string(),
            ))?;
        }
    }

    Ok(filenames)
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
