use std::cmp;
use std::fmt;
use std::io::{prelude::*, BufReader};
use std::path::Path;
use std::thread;

use anyhow::{Context, Result};
use clap::{App, Arg, SubCommand};
use crossbeam::channel;
use diesel::{prelude::*, SqliteConnection};
use env_logger;
use flate2::bufread::GzDecoder;
use log::{debug, info, warn};
use num_cpus;
use reqwest;
use thiserror::Error;
use threadpool::ThreadPool;

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

fn is_valid_word(word: &str) -> bool {
    unimplemented!();
}

#[derive(Debug, Error)]
enum EntryError {
    #[error("invalid {0}-gram line: {1}")]
    InvalidLine(i64, String),

    #[error("invalid {0}-gram len: {1}")]
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

pub fn run() -> Result<()> {
    env_logger::init();

    let args = parse_args().context("failed to parse args")?;

    let filename = db_filename(&args.dir);
    let conn = SqliteConnection::establish(&filename)
        .with_context(|| format!("failed to connect db: {}", &filename))?;

    do_one_grams(&conn, &args).with_context(|| format!("failed to do one grams: {:?}", &args))?;
    let wordidx = get_wordidx();

    do_two_to_five_grams();

    finalize();

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
    let dir = matches.value_of("dir").unwrap_or_else(|| ".").to_string();
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
    let n = 1;
    let lang = args.lang.clone();

    // Download
    let (dl_tx, dl_rx) = channel::bounded(args.dl_parallel());
    let dl_pool = ThreadPool::with_name("1-gram download".to_string(), args.dl_parallel());
    dl_pool.execute(move || {
        for idx in 0..total_file_num(lang, 1) {
            let gz_body = download(lang, n, idx)
                .with_context(|| format!("failed to download {}-gram {}", n, idx));

            dl_tx
                .send((Query { lang, n, idx }, gz_body))
                .with_context(|| format!("failed to download {}-gram {}", n, idx))
                .unwrap();
        }
    });

    // Parse
    let (parse_tx, parse_rx) = channel::bounded(args.worker_parallel());
    let parse_pool = ThreadPool::with_name("1-gram parse".to_string(), args.worker_parallel());
    parse_pool.execute(move || {
        for (query, gz_body) in dl_rx {
            match gz_body {
                Ok(gz_body) => {
                    let entries = parse_gz_data_to_entries(&gz_body, n).with_context(|| {
                        format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                    });
                    parse_tx
                        .send((query.clone(), entries))
                        .with_context(|| {
                            format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                        })
                        .unwrap();
                }
                Err(e) => {
                    parse_tx
                        .send((query.clone(), Err(e)))
                        .with_context(|| {
                            format!("failed to parse {}-gram {} gz data", &query.n, &query.idx)
                        })
                        .unwrap();
                }
            }
        }
    });

    // Save
    for (query, entries) in parse_rx {
        match entries {
            Ok(entries) => save_one_gram(conn, entries).with_context(|| {
                format!("failed to save {}-gram {} entries", query.n, query.idx)
            })?,
            Err(e) => return Err(e),
        }
    }

    // Wait
    dl_pool.join();
    parse_pool.join();

    Ok(())
}

fn download(lang: Language, n: i64, idx: i64) -> Result<Vec<u8>> {
    let url = remote_url(lang, n, idx);
    let mut res = vec![];

    reqwest::blocking::get(&url)
        .with_context(|| format!("failed to download {}", &url))?
        .read_to_end(&mut res)
        .with_context(|| format!("failed to download {}", &url))?;

    Ok(res)
}

fn parse_gz_data_to_entries(gz_data: &[u8], n: i64) -> Result<Vec<Entry>> {
    let gz = GzDecoder::new(&gz_data[..]);
    let mut res = vec![];

    for line in BufReader::new(gz).lines() {
        let line = line?;
        let entry = match Entry::parse(&line, n) {
            Ok(ent) => ent,
            Err(e) => {
                warn!("{}", e);
                continue;
            }
        };
        res.push(entry);
    }

    Ok(res)
}

fn save_one_gram(conn: &SqliteConnection, entries: Vec<Entry>) -> Result<()> {
    unimplemented!();
}

fn get_wordidx() {}

fn do_two_to_five_grams() {}

fn finalize() {}

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
}
