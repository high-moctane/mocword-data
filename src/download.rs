use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::io::{prelude::*, BufReader};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{App, Arg};
use crossbeam::channel;
use crossbeam::utils::Backoff;
use diesel::{self, prelude::*, SqliteConnection};
use env_logger;
use flate2::bufread::GzDecoder;
use log::{debug, error, info};
use num_cpus;
use reqwest::blocking::Client;
use thiserror::Error;
use threadpool::ThreadPool;

use crate::models;
use crate::schema;

const MAX_PARALLEL_DOWNLOAD: usize = 2;

#[derive(Debug)]
struct Args {
    lang: Language,
    dir: String,
    parallel: usize,
}

impl Args {
    fn worker_parallel(&self) -> usize {
        match self.parallel {
            0 => num_cpus::get(),
            _ => self.parallel,
        }
    }

    fn dl_parallel(&self) -> usize {
        cmp::min(self.worker_parallel(), MAX_PARALLEL_DOWNLOAD)
    }
}

#[derive(Clone, Copy, Debug)]
enum Language {
    English,
}

impl From<&str> for Language {
    fn from(from: &str) -> Language {
        match from {
            "eng" => Language::English,
            _ => panic!("invalid language: {}", from),
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::English => write!(f, "eng"),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Entry {
    ngram: Vec<String>,
    score: i64,
}

impl Entry {
    fn parse(line: &str, n: i64) -> Result<Entry> {
        let elems: Vec<&str> = line.split("\t").collect();
        if elems.len() < 2 {
            return Err(EntryError::InvalidLine(n, line.to_string()))?;
        }

        let ngram: Vec<String> = elems[0].split(" ").map(|s| s.to_string()).collect();
        if ngram.len() != n as usize || ngram.iter().any(|word| !is_valid_word(word)) {
            return Err(EntryError::InvalidNgram(n, line.to_string()))?;
        }

        let mut score = 0;
        for year_summary in elems[1..].iter() {
            let values: Vec<&str> = year_summary.split(",").collect();
            if values.len() != 3 {
                return Err(EntryError::InvalidYearSummary(n, line.to_string()))?;
            }

            let match_count: i64 = values[1]
                .parse()
                .with_context(|| EntryError::InvalidYearSummary(n, line.to_string()))?;

            score += match_count;
        }

        Ok(Entry { ngram, score })
    }
}

#[derive(Debug, PartialEq)]
struct IndexedEntry {
    indexed_ngram: Vec<i64>,
    score: i64,
}

impl IndexedEntry {
    fn parse(line: &str, n: i64, wordidx: &HashMap<String, i64>) -> Result<Option<IndexedEntry>> {
        let elems: Vec<&str> = line.split("\t").collect();
        if elems.len() < 2 {
            return Err(EntryError::InvalidLine(n, line.to_string()))?;
        }

        let ngram: Vec<String> = elems[0].split(" ").map(|s| s.to_string()).collect();
        if ngram.len() != n as usize || ngram.iter().any(|word| !is_valid_word(word)) {
            return Err(EntryError::InvalidNgram(n, line.to_string()))?;
        }
        let mut indexed_ngram = vec![];
        for word in ngram.into_iter() {
            match wordidx.get(&word) {
                Some(idx) => indexed_ngram.push(idx.clone().to_owned()),
                None => return Ok(None),
            }
        }

        let mut score = 0;
        for year_summary in elems[1..].iter() {
            let values: Vec<&str> = year_summary.split(",").collect();
            if values.len() != 3 {
                return Err(EntryError::InvalidYearSummary(n, line.to_string()))?;
            }

            let match_count: i64 = values[1]
                .parse()
                .with_context(|| EntryError::InvalidYearSummary(n, line.to_string()))?;

            score += match_count;
        }

        Ok(Some(IndexedEntry {
            indexed_ngram,
            score,
        }))
    }
}

fn is_valid_word(word: &str) -> bool {
    let part_of_speech = vec![
        "NOUN", ".", "VERB", "ADP", "DET", "ADJ", "PRON", "ADV", "NUM", "CONJ", "PRT", "X",
    ];
    let part_of_sppech_itself: Vec<_> = part_of_speech
        .iter()
        .map(|pos| format!("_{}_", pos))
        .collect();
    let part_of_speech_suffix: Vec<_> = part_of_speech
        .iter()
        .map(|pos| format!("_{}", pos))
        .collect();

    word.len() > 0
        && part_of_sppech_itself.iter().all(|pos| pos != word)
        && part_of_speech_suffix.iter().all(|pos| !word.ends_with(pos))
        && true
}

#[derive(Debug, Error)]
enum EntryError {
    #[error("invalid {0}-gram line: {1}")]
    InvalidLine(i64, String),

    #[error("invalid {0}-gram: {1}")]
    InvalidNgram(i64, String),

    #[error("invalid {0}-gram year summary: {1}")]
    InvalidYearSummary(i64, String),
}

#[derive(Clone)]
struct Query {
    lang: Language,
    n: i64,
    idx: i64,
}

#[derive(Debug, Error)]
enum ThreadError {
    #[error("failed to channel send: {0}")]
    ChannelError(String),
}

pub fn run() -> Result<()> {
    env_logger::init();

    let args = parse_args().context("failed to parse args")?;

    let filename = db_filename(&args.dir);
    let conn = SqliteConnection::establish(&filename)
        .with_context(|| format!("failed to connect db: {}", &filename))?;

    do_one_grams(&conn, &args).with_context(|| format!("failed to do one grams: {:?}", &args))?;
    let wordidx = get_wordidx(&conn).context("failed to fetch wordidx")?;

    do_two_to_five_grams(&conn, &args, &wordidx)
        .with_context(|| format!("failed to do two to five grams: {:?}", &args))?;

    finalize(&conn).context("failed to finalize")?;

    Ok(())
}

fn parse_args() -> Result<Args> {
    let languages = vec!["eng"];

    let matches = App::new("Mocword Download")
        .author("high-moctane <high.moctane@gmail.com>")
        .about("Download and build mocword ngram data")
        .arg(
            Arg::with_name("language")
                .short("l")
                .long("lang")
                .value_name("LANG")
                .help(format!("Sets a language (default: eng): {}", languages.join(", ")).as_str())
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dir")
                .long("dir")
                .short("d")
                .value_name("DIR")
                .help("Destination directory path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("parallel")
                .long("parallel")
                .short("p")
                .value_name("NUM")
                .help("Parallel workers number. 0 uses all core. Default is 1.")
                .takes_value(true),
        )
        .get_matches();

    let lang = matches.value_of("language").unwrap_or_else(|| "eng").into();
    let dir = matches
        .value_of("dir")
        .unwrap_or_else(|| "build")
        .to_string();
    let parallel_str = matches.value_of("parallel").unwrap_or_else(|| "1");
    let parallel = parallel_str
        .parse()
        .with_context(|| format!("invalid parallel number: {}", parallel_str))?;

    Ok(Args {
        lang,
        dir,
        parallel,
    })
}

fn do_one_grams(conn: &SqliteConnection, args: &Args) -> Result<()> {
    info!("start: do_one_grams");

    let n = 1;

    // Download
    let (dl_tx, dl_rx) = channel::bounded(0);
    let dl_pool = parallel_download(conn, args, n, dl_tx);

    // Parse
    let (parse_tx, parse_rx) = channel::bounded(0);
    let parse_pool = parallel_parse_gz_data_to_entries(args, dl_rx, parse_tx, n);

    // Save
    for (query, entries) in parse_rx {
        let total = total_file_num(query.lang, query.n);
        info!("start: save {}-gram {} of {}", query.n, query.idx, total);

        match entries {
            Ok(entries) => {
                conn.transaction::<_, Box<dyn std::error::Error>, _>(|| {
                    save_one_gram(conn, entries)?;
                    save_fetched_file(conn, n, query.idx)?;
                    Ok(())
                })
                .unwrap();
            }

            Err(e) => return Err(e),
        }

        info!("end  : save {}-gram {} of {}", query.n, query.idx, total);
    }

    // Wait
    dl_pool.join();
    parse_pool.join();
    if dl_pool.panic_count() != 0 {
        Err(ThreadError::ChannelError("dl_pool panicked".to_string()))?;
    }
    if parse_pool.panic_count() != 0 {
        Err(ThreadError::ChannelError("parse_pool panicked".to_string()))?;
    }

    info!("end  : do_one_grams");
    Ok(())
}

fn parallel_download(
    conn: &SqliteConnection,
    args: &Args,
    n: i64,
    tx: channel::Sender<(Query, Result<Vec<u8>>)>,
) -> ThreadPool {
    let lang = args.lang.clone();
    let pool = ThreadPool::with_name("1-gram download".to_string(), args.dl_parallel());
    for idx in 0..total_file_num(lang, 1) {
        if is_fetched_file(conn, n, idx).unwrap() {
            continue;
        }

        let tx = tx.clone();
        pool.execute(move || {
            let total = total_file_num(lang, n);
            info!("start: download {}-gram {} of {}", n, idx, total);

            let gz_body = download(lang, n, idx)
                .with_context(|| format!("failed to download {}-gram {}", n, idx));

            tx.send((Query { lang, n, idx }, gz_body))
                .with_context(|| format!("failed to download {}-gram {}", n, idx))
                .unwrap();

            info!("end  : download {}-gram {} of {}", n, idx, total);
        })
    }
    pool
}

fn download(lang: Language, n: i64, idx: i64) -> Result<Vec<u8>> {
    let backoff = Backoff::new();

    loop {
        let url = remote_url(lang, n, idx);
        let mut ret = vec![];

        let resp = Client::new()
            .get(&url)
            .send()
            .with_context(|| format!("failed to download {}", &url));

        if let Err(e) = resp {
            error!("failed to get {}: {}", url, e);
            backoff.spin();
            continue;
        }

        let result = resp
            .unwrap()
            .read_to_end(&mut ret)
            .with_context(|| format!("failed to download {}", &url));

        if let Err(e) = result {
            error!("failed to read {}: {}", url, e);
            backoff.spin();
            continue;
        }

        return Ok(ret);
    }
}

fn parallel_parse_gz_data_to_entries(
    args: &Args,
    dl_rx: channel::Receiver<(Query, Result<Vec<u8>>)>,
    entries_tx: channel::Sender<(Query, Result<Vec<Entry>>)>,
    n: i64,
) -> ThreadPool {
    let pool = ThreadPool::with_name("1-gram parse".to_string(), args.worker_parallel());
    for _ in 0..total_file_num(args.lang, n) {
        let dl_rx = dl_rx.clone();
        let entries_tx = entries_tx.clone();
        pool.execute(move || {
            let (query, gz_body) = dl_rx.recv().unwrap();

            let total = total_file_num(query.lang, query.n);
            info!("start: parse {}-gram {} of {}", query.n, query.idx, total);

            match gz_body {
                Ok(gz_body) => {
                    let entries = parse_gz_data_to_entries(&gz_body, n).with_context(|| {
                        format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                    });
                    entries_tx
                        .send((query.clone(), entries))
                        .with_context(|| {
                            format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                        })
                        .unwrap();
                }
                Err(e) => {
                    entries_tx
                        .send((query.clone(), Err(e)))
                        .with_context(|| {
                            format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                        })
                        .unwrap();
                }
            }

            info!("end  : parse {}-gram {} of {}", query.n, query.idx, total);
        });
    }
    pool
}

fn parse_gz_data_to_entries(gz_data: &[u8], n: i64) -> Result<Vec<Entry>> {
    let gz = GzDecoder::new(&gz_data[..]);
    let mut res = vec![];

    for line in BufReader::new(gz).lines() {
        let line = line?;
        let entry = match Entry::parse(&line, n) {
            Ok(ent) => ent,
            Err(e) => {
                debug!("{}", e);
                continue;
            }
        };
        res.push(entry);
    }

    Ok(res)
}

fn save_one_gram(conn: &SqliteConnection, entries: Vec<Entry>) -> Result<()> {
    let one_grams: Vec<_> = entries
        .into_iter()
        .map(|entry| models::NewOneGram {
            word: entry.ngram[0].to_owned(),
            score: entry.score,
        })
        .collect();

    diesel::insert_into(schema::one_grams::table)
        .values(&one_grams)
        .execute(conn)
        .context("failed to save one gram")?;

    Ok(())
}

fn get_wordidx(conn: &SqliteConnection) -> Result<HashMap<String, i64>> {
    use schema::one_grams::dsl;

    info!("start: get wordidx");

    let one_grams = dsl::one_grams
        .load::<models::OneGram>(conn)
        .context("failed to get wordidx")?;

    info!("end  : get wordidx");

    Ok(HashMap::from(
        one_grams
            .into_iter()
            .map(|one_gram| (one_gram.word, one_gram.id))
            .collect(),
    ))
}

fn do_two_to_five_grams(
    conn: &SqliteConnection,
    args: &Args,
    wordidx: &HashMap<String, i64>,
) -> Result<()> {
    // Download
    for n in 2..=5 {
        let (dl_tx, dl_rx) = channel::bounded(0);
        let dl_pool = parallel_download(conn, args, n, dl_tx);

        // Parse
        let (parse_tx, parse_rx) = channel::bounded(0);
        let parse_pool =
            parallel_parse_gz_data_to_indexed_entries(args, dl_rx, parse_tx, n, wordidx.to_owned());

        // Save
        for (query, entries) in parse_rx {
            let total = total_file_num(query.lang, query.n);
            info!("start: save {}-gram {} of {}", query.n, query.idx, total);

            match entries {
                Ok(entries) => {
                    conn.transaction::<_, Box<dyn std::error::Error>, _>(|| {
                        save_ngram(conn, entries, n)?;
                        save_fetched_file(conn, n, query.idx)?;
                        Ok(())
                    })
                }
                .unwrap(),
                Err(e) => return Err(e),
            }

            info!("end  : save {}-gram {} of {}", query.n, query.idx, total);
        }

        // Wait
        dl_pool.join();
        parse_pool.join();
        if dl_pool.panic_count() != 0 {
            Err(ThreadError::ChannelError("dl_pool panicked".to_string()))?;
        }
        if parse_pool.panic_count() != 0 {
            Err(ThreadError::ChannelError("parse_pool panicked".to_string()))?;
        }
    }

    Ok(())
}

fn parallel_parse_gz_data_to_indexed_entries(
    args: &Args,
    dl_rx: channel::Receiver<(Query, Result<Vec<u8>>)>,
    entries_tx: channel::Sender<(Query, Result<Vec<IndexedEntry>>)>,
    n: i64,
    wordidx: HashMap<String, i64>,
) -> ThreadPool {
    let pool = ThreadPool::with_name(format!("{}-gram parse", n), args.worker_parallel());
    let wordidx = Arc::new(wordidx);

    for _ in 0..total_file_num(args.lang, n) {
        let entries_tx = entries_tx.clone();
        let wordidx = Arc::clone(&wordidx);
        let dl_rx = dl_rx.clone();

        pool.execute(move || {
            let (query, gz_body) = dl_rx.recv().unwrap();

            let total = total_file_num(query.lang, query.n);
            info!(
                "start: parse to indexed entries {}-gram {} of {}",
                query.n, query.idx, total
            );

            match gz_body {
                Ok(gz_body) => {
                    let entries = parse_gz_data_to_indexed_entries(&gz_body, n, &wordidx)
                        .with_context(|| {
                            format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                        });
                    entries_tx
                        .send((query.clone(), entries))
                        .with_context(|| {
                            format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                        })
                        .unwrap();
                }
                Err(e) => {
                    entries_tx
                        .send((query.clone(), Err(e)))
                        .with_context(|| {
                            format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                        })
                        .unwrap();
                }
            }

            info!(
                "end  : parse to indexed entries {}-gram {} of {}",
                query.n, query.idx, total
            );
        });
    }
    pool
}

fn parse_gz_data_to_indexed_entries(
    gz_data: &[u8],
    n: i64,
    wordidx: &HashMap<String, i64>,
) -> Result<Vec<IndexedEntry>> {
    info!("start: parse to indexed entries {}-gram", n);

    let gz = GzDecoder::new(&gz_data[..]);
    let mut res = vec![];

    for line in BufReader::new(gz).lines() {
        let line = line?;
        let entry = match IndexedEntry::parse(&line, n, wordidx) {
            Ok(ent) => ent,
            Err(e) => {
                debug!("{}", e);
                continue;
            }
        };
        res.push(entry);
    }

    info!("end  : parse to indexed entries {}-gram", n);

    Ok(res
        .into_iter()
        .filter(|opt_entry| opt_entry.is_some())
        .map(|opt_entry| opt_entry.unwrap())
        .collect())
}

fn save_ngram(conn: &SqliteConnection, indexed_entries: Vec<IndexedEntry>, n: i64) -> Result<()> {
    match n {
        2 => save_two_gram(conn, indexed_entries),
        3 => save_three_gram(conn, indexed_entries),
        4 => save_four_gram(conn, indexed_entries),
        5 => save_five_gram(conn, indexed_entries),
        _ => panic!("invalid ngram: {}", n),
    }
}

fn save_two_gram(conn: &SqliteConnection, indexed_entries: Vec<IndexedEntry>) -> Result<()> {
    info!("start: save two gram");

    let two_grams: Vec<_> = indexed_entries
        .into_iter()
        .map(|entry| models::NewTwoGram {
            word1_id: entry.indexed_ngram[0].to_owned(),
            word2_id: entry.indexed_ngram[1].to_owned(),
            score: entry.score,
        })
        .collect();

    diesel::insert_into(schema::two_grams::table)
        .values(&two_grams)
        .execute(conn)
        .context("failed to save two gram")?;

    info!("end  : save two gram");

    Ok(())
}

fn save_three_gram(conn: &SqliteConnection, indexed_entries: Vec<IndexedEntry>) -> Result<()> {
    info!("start: save three gram");

    let three_grams: Vec<_> = indexed_entries
        .into_iter()
        .map(|entry| models::NewThreeGram {
            word1_id: entry.indexed_ngram[0].to_owned(),
            word2_id: entry.indexed_ngram[1].to_owned(),
            word3_id: entry.indexed_ngram[2].to_owned(),
            score: entry.score,
        })
        .collect();

    diesel::insert_into(schema::three_grams::table)
        .values(&three_grams)
        .execute(conn)
        .context("failed to save three gram")?;

    info!("end  : save three gram");

    Ok(())
}

fn save_four_gram(conn: &SqliteConnection, indexed_entries: Vec<IndexedEntry>) -> Result<()> {
    info!("start: save four gram");

    let four_grams: Vec<_> = indexed_entries
        .into_iter()
        .map(|entry| models::NewFourGram {
            word1_id: entry.indexed_ngram[0].to_owned(),
            word2_id: entry.indexed_ngram[1].to_owned(),
            word3_id: entry.indexed_ngram[2].to_owned(),
            word4_id: entry.indexed_ngram[3].to_owned(),
            score: entry.score,
        })
        .collect();

    diesel::insert_into(schema::four_grams::table)
        .values(&four_grams)
        .execute(conn)
        .context("failed to save four gram")?;

    info!("end  : save four gram");

    Ok(())
}

fn save_five_gram(conn: &SqliteConnection, indexed_entries: Vec<IndexedEntry>) -> Result<()> {
    info!("start: save five gram");

    let five_grams: Vec<_> = indexed_entries
        .into_iter()
        .map(|entry| models::NewFiveGram {
            word1_id: entry.indexed_ngram[0].to_owned(),
            word2_id: entry.indexed_ngram[1].to_owned(),
            word3_id: entry.indexed_ngram[2].to_owned(),
            word4_id: entry.indexed_ngram[3].to_owned(),
            word5_id: entry.indexed_ngram[4].to_owned(),
            score: entry.score,
        })
        .collect();

    diesel::insert_into(schema::five_grams::table)
        .values(&five_grams)
        .execute(conn)
        .context("failed to save five gram")?;

    info!("end  : save five gram");

    Ok(())
}

fn save_fetched_file(conn: &SqliteConnection, n: i64, idx: i64) -> Result<()> {
    diesel::insert_into(schema::fetched_files::table)
        .values(models::NewFetchedFile { n, idx })
        .execute(conn)
        .with_context(|| format!("failed to save fetched_files: {}-gram {}", n, idx))?;
    Ok(())
}

fn is_fetched_file(conn: &SqliteConnection, n: i64, idx: i64) -> Result<bool> {
    use schema::fetched_files::dsl;

    let res = dsl::fetched_files
        .filter(dsl::n.eq_all(n))
        .filter(dsl::idx.eq_all(idx))
        .load::<models::FetchedFile>(conn)
        .with_context(|| format!("failed to get fetched file: {}-ngram {}", n, idx))?;

    Ok(res.len() > 0)
}

fn finalize(conn: &SqliteConnection) -> Result<()> {
    // Vacuum
    info!("start: vacuum");
    diesel::sql_query("vacuum")
        .execute(conn)
        .context("vacuum failed")?;
    info!("end  : vacuum");

    Ok(())
}

fn total_file_num(lang: Language, n: i64) -> i64 {
    match lang {
        Language::English => match n {
            1 => 24,
            2 => 589,
            3 => 6881,
            4 => 6668,
            5 => 19423,
            _ => panic!("invalid ngram number: {}", n),
        },
    }
}

fn remote_url(lang: Language, n: i64, idx: i64) -> String {
    format!(
        "http://storage.googleapis.com/books/ngrams/books/20200217/{}/{}-{:05}-of-{:05}.gz",
        lang,
        n,
        idx,
        total_file_num(lang, n)
    )
}

fn db_filename(dir: &str) -> String {
    Path::new(dir)
        .join("download.sqlite")
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_entry() {
        use super::*;

        // OK
        let res = Entry::parse("abc def\t1992,534,756\t1993,645,423", 2);
        match res {
            Ok(res) => assert_eq!(
                res,
                Entry {
                    ngram: vec!["abc".to_string(), "def".to_string()],
                    score: 534 + 645,
                }
            ),
            Err(e) => panic!("{}", e),
        }

        // Invalid line
        let res = Entry::parse("hello, test", 2);
        match res {
            Ok(res) => panic!("{:?}", res),
            Err(_) => {}
        };

        // Invalid ngram
        let res = Entry::parse("abc def\t1992,534,756\t1993,645,423", 3);
        match res {
            Ok(res) => panic!("{:?}", res),
            Err(_) => {}
        };
        let res = Entry::parse("abc .\t1992,534,756\t1993,645,423", 3);
        match res {
            Ok(res) => panic!("{:?}", res),
            Err(_) => {}
        };

        // Invalid year_summary
        let res = Entry::parse("abc def\t1992,534\t1993,645,423", 3);
        match res {
            Ok(res) => panic!("{:?}", res),
            Err(_) => {}
        };
    }

    #[test]
    fn test_is_valid_word() {
        use super::*;

        let valid = vec!["a", "b", "agroije", "r%@#@%!2342"];
        let invalid = vec!["", "_PRON_", "party_NOUN"];

        for w in valid.iter() {
            println!("{}", w);
            assert!(is_valid_word(w));
        }
        for w in invalid.iter() {
            println!("{}", w);
            assert!(!is_valid_word(w));
        }
    }

    #[test]
    fn test_parse_indexed_entry() {
        use super::*;

        let mut hashmap = HashMap::new();
        hashmap.insert("a".to_string(), 10);
        hashmap.insert("bc".to_string(), 1);
        hashmap.insert("def".to_string(), 22);

        let indexed_entry = IndexedEntry::parse("a\t1996,243,12\t1999,5645,4234", 1, &hashmap);
        assert_eq!(
            indexed_entry.unwrap().unwrap(),
            IndexedEntry {
                indexed_ngram: vec![10],
                score: 243 + 5645
            }
        );

        let indexed_entry = IndexedEntry::parse("def a\t1996,243,12\t1999,5645,4234", 2, &hashmap);
        assert_eq!(
            indexed_entry.unwrap().unwrap(),
            IndexedEntry {
                indexed_ngram: vec![22, 10],
                score: 243 + 5645
            }
        );

        let indexed_entry =
            IndexedEntry::parse("def powa\t1996,243,12\t1999,5645,4234", 2, &hashmap);
        assert_eq!(indexed_entry.unwrap(), None,);
    }
}
