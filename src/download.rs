use crate::embedded_migrations;
use crate::models;
use crate::schema;
use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{prelude::*, Connection, SqliteConnection};
use flate2::read::GzDecoder;
use log::info;
use radix_trie::Trie;
use reqwest::blocking::Client;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::fmt;
use std::io::{self, prelude::*, BufReader, BufWriter, SeekFrom};
use tempfile::NamedTempFile;
use thiserror::Error;
use threadpool::ThreadPool;

const PART_OF_SPEECHES: [&str; 12] = [
    "_NOUN_", "_._", "_VERB_", "_ADP_", "_DET_", "_ADJ_", "_PRON_", "_ADV_", "_NUM_", "_CONJ_",
    "_PRT_", "_X_",
];

const PART_OF_SPEECH_SUFFIXES: [&str; 12] = [
    "_NOUN", "_.", "_VERB", "_ADP", "_DET", "_ADJ", "_PRON", "_ADV", "_NUM", "_CONJ", "_PRT", "_X",
];

pub fn run() -> Result<()> {
    initialize().context("failed to initalize")?;
    let args = get_args()?;
    let conn_pool = new_conn_pool(&args).context("failed to establish conn")?;
    migrate(&conn_pool).context("failed to migrate")?;
    download_and_save_all(&args, &conn_pool).context("failed to download and save")?;
    finalize(&args, &conn_pool).context("failed to finalize")?;
    Ok(())
}

pub fn initialize() -> Result<()> {
    SimpleLogger::init(LevelFilter::Info, Config::default())?;
    Ok(())
}

#[derive(Debug)]
struct Args {
    lang: Language,
    parallel: usize,
    dsn: String,
}

fn get_args() -> Result<Args> {
    let matches = App::new("Mocword-data")
        .author("high-moctane <high.moctane@gmail.com>")
        .about("Mocword data downloader")
        .arg(
            Arg::with_name("lang")
                .short("l")
                .long("lang")
                .value_name("lang")
                .help("Language (eng)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("parallel")
                .short("p")
                .long("parallel")
                .value_name("parallel")
                .help("Parallel num (default is NumCPU)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dsn")
                .long("dsn")
                .value_name("dsn")
                .help("Database DSN")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cache")
                .short("c")
                .long("cache")
                .value_name("cache")
                .help("LRU cache size")
                .takes_value(true),
        )
        .get_matches();

    let lang: Language = Language::from(matches.value_of("lang").unwrap_or("eng"));
    // TODO
    let parallel = matches
        .value_of("parallel")
        .unwrap_or(&format!("{}", 6))
        .parse()?;
    let dsn = matches.value_of("dsn").unwrap_or(&dsn()).to_string();

    Ok(Args {
        lang,
        parallel,
        dsn,
    })
}

fn dsn() -> String {
    format!("file:data.sqlite?cache=shared")
}

#[derive(Copy, Clone, Debug)]
enum Language {
    English,
}

impl Language {
    fn url_name(&self) -> String {
        match *self {
            Language::English => "eng".to_string(),
        }
    }
}

impl From<&str> for Language {
    fn from(from: &str) -> Language {
        match &*from {
            "eng" => Language::English,
            _ => unimplemented!("unimplemented language ({})", from),
        }
    }
}

fn new_conn_pool(args: &Args) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let manager = ConnectionManager::<SqliteConnection>::new(&args.dsn);
    Pool::builder()
        .max_size(1)
        .build(manager)
        .context("failed to establish conn pool")
}

fn migrate(pool: &Pool<ConnectionManager<SqliteConnection>>) -> Result<()> {
    info!("start: migrate");
    embedded_migrations::run_with_output(&pool.get()?, &mut io::stdout())?;
    info!("end  : migrate");
    Ok(())
}

fn download_and_save_all(
    args: &Args,
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<()> {
    download_and_save_one_grams(args, conn_pool)
        .context("failed to download and save one grams")?;
    let cache = get_one_grams_cache(args, conn_pool).context("failed to get one grams cache")?;
    for n in 2..=5 {
        download_and_save_ngrams(args, conn_pool, &cache, n)
            .with_context(|| format!("failed to download and save {}-grams", n))?;
    }

    Ok(())
}

fn download_and_save_one_grams(
    args: &Args,
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<()> {
    let n = 1;

    let thread_pool = ThreadPool::new(args.parallel);
    let client = Client::builder().pool_max_idle_per_host(2).build()?;

    for query in gen_queries(args.lang, n).into_iter() {
        let conn_pool = conn_pool.clone();
        let client = client.clone();
        thread_pool.execute(move || download_and_save_one_gram(conn_pool, client, query).unwrap());
    }

    thread_pool.join();
    if thread_pool.panic_count() > 0 {
        bail!("panic occured on download_and_save");
    }

    Ok(())
}

fn download_and_save_one_gram(
    conn_pool: Pool<ConnectionManager<SqliteConnection>>,
    client: Client,
    query: Query,
) -> Result<()> {
    if is_fetched_file(&conn_pool, &query)? {
        return Ok(());
    }

    let mut gz_file = NamedTempFile::new().context("failed to create tempfile")?;
    download(&client, &query, &mut gz_file)
        .with_context(|| format!("failed to download: {:?}", &query))?;
    gz_file
        .seek(SeekFrom::Start(0))
        .context("failed to seek file")?;

    let mut tsv_file = NamedTempFile::new().context("failed to create tempfile")?;
    save_to_tsv(&query, &mut gz_file, &mut tsv_file)
        .with_context(|| format!("failed to save tsv: {:?}", &query))?;
    tsv_file
        .seek(SeekFrom::Start(0))
        .context("failed to seek file")?;

    let conn = conn_pool.get()?;
    conn.transaction::<_, anyhow::Error, _>(|| {
        save_to_db(&conn, &query, tsv_file.path().to_str().unwrap())
            .with_context(|| format!("failed to read and save: {:?}", &query))?;
        mark_fetched_file(&conn, &query)
            .with_context(|| format!("failed to mark fetched file: {:?}", &query))?;
        Ok(())
    })?;

    unimplemented!();
}

fn download_and_save_ngrams(
    args: &Args,
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
    cache: &Trie<String, i64>,
    n: i64,
) -> Result<()> {
    unimplemented!();
}

fn download(client: &Client, query: &Query, w: &mut impl Write) -> Result<()> {
    let url = file_url(&query);

    let response = client
        .get(&url)
        .send()
        .with_context(|| format!("failed to download {}", &url))?;

    info!("start: download {:?}", &query);

    io::copy(&mut BufReader::new(response), &mut BufWriter::new(w))
        .with_context(|| format!("failed to save {:?}", query))?;

    info!("end  : download {:?}", &query);

    Ok(())
}

fn save_to_tsv_one_gram(query: &Query, gz: &mut impl Read, tsv: &mut impl Write) -> Result<()> {
    let r = GzDecoder::new(gz);
    let r = BufReader::new(r);
    let mut w = BufWriter::new(tsv);

    for line in r.lines() {
        let entry = match Entry::new(&line?, query.n) {
            Ok(entry) => entry,
            Err(_) => continue,
        }
        writeln!(w, "{}", entry)
    }

    unimplemented!();
}

fn save_to_db(conn: &SqliteConnection, query: &Query, tsv_filename: &str) -> Result<()> {
    unimplemented!();
}

fn get_one_grams_cache(
    args: &Args,
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<Trie<String, i64>> {
    unimplemented!();
}

fn finalize(args: &Args, conn_pool: &Pool<ConnectionManager<SqliteConnection>>) -> Result<()> {
    unimplemented!();
}

#[derive(Debug)]
struct Query {
    lang: Language,
    n: i64,
    idx: i64,
}

fn gen_queries(lang: Language, n: i64) -> Vec<Query> {
    (0..total_file_num(lang, n))
        .into_iter()
        .map(|idx| Query { lang, n, idx })
        .collect()
}

fn total_file_num(lang: Language, n: i64) -> i64 {
    match lang {
        Language::English => match n {
            1 => 24,
            2 => 589,
            3 => 6881,
            4 => 6668,
            5 => 19423,
            _ => panic!("invalid ngram number: {:?}", n),
        },
    }
}

fn file_url(query: &Query) -> String {
    format!(
        "http://storage.googleapis.com/books/ngrams/books/20200217/{}/{}-{:05}-of-{:05}.gz",
        query.lang.url_name(),
        query.n,
        query.idx,
        total_file_num(query.lang, query.n)
    )
}

struct Entry {
    ngram: Vec<String>,
    score: i64,
}

impl Entry {
    fn new(line: &str, n: i64) -> Result<Entry> {
        let elems: Vec<_> = line.split("\t").collect();
        if elems.len() < 2 {
            return Err(EntryError::InvalidLengthEntry {
                line: line.to_string(),
            })?;
        }

        let ngram = &elems[0];
        let counts = &elems[1..];

        let ngram: Vec<_> = ngram.split(" ").map(|w| w.to_string()).collect();
        if ngram.len() != n as usize {
            return Err(EntryError::InvalidLengthNgram {
                n,
                ngram: ngram.join(" "),
            })?;
        }
        if ngram.iter().any(|word| !is_valid_word(&word)) {
            return Err(EntryError::InvalidWord {
                ngram: ngram.join(" "),
            })?;
        }

        let mut score = 0;
        for l in counts.iter() {
            let vals: Vec<_> = l.split(",").collect();
            if vals.len() != 3 {
                return Err(EntryError::InvalidCounts {
                    counts: l.to_string(),
                })?;
            }
            score += vals[1].parse::<i64>()?;
        }

        Ok(Entry { ngram, score })
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t{}", self.ngram.join(" "), self.score)
    }
}

fn is_valid_word(word: &str) -> bool {
    return word.len() > 0
        && PART_OF_SPEECHES.iter().all(|ps| word != *ps)
        && PART_OF_SPEECH_SUFFIXES.iter().all(|ps| !word.ends_with(ps));
}

#[derive(Debug, Error)]
enum EntryError {
    #[error("invalid line ({line})")]
    InvalidLengthEntry { line: String },

    #[error("invalid length of {n}-gram ({ngram})")]
    InvalidLengthNgram { n: i64, ngram: String },

    #[error("invalid counts ({counts})")]
    InvalidCounts { counts: String },

    #[error("invalid word ({ngram})")]
    InvalidWord { ngram: String },
}

fn is_fetched_file(
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
    query: &Query,
) -> Result<bool> {
    use schema::fetched_files::dsl;

    let res = dsl::fetched_files
        .filter(dsl::n.eq_all(query.n))
        .filter(dsl::idx.eq_all(query.idx))
        .load::<models::FetchedFile>(&conn_pool.get()?)
        .with_context(|| format!("failed to fetch fetched_files ({:?})", &query))?;

    Ok(res.len() > 0)
}

fn mark_fetched_file(conn: &SqliteConnection, query: &Query) -> Result<()> {
    let value = models::NewFetchedFile {
        n: query.n,
        idx: query.idx,
    };

    diesel::insert_into(schema::fetched_files::table)
        .values(&value)
        .execute(conn)
        .with_context(|| format!("failed to insert fetched_files: {:?}", query))?;

    Ok(())
}
