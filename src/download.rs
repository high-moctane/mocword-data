use std::io;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, SeekFrom};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{prelude::*, Connection, MysqlConnection};
use exponential_backoff::Backoff;
use flate2::read::GzDecoder;
use indoc::indoc;
use log::{error, info};
use num_cpus;
use reqwest;
use simplelog::{Config, LevelFilter, SimpleLogger};
use tempfile;
use thiserror::Error;
use threadpool::ThreadPool;

use crate::embedded_migrations;
use crate::models;
use crate::schema;

const MAX_BUFFER_SIZE: usize = 10000;

const PART_OF_SPEECHES: [&str; 12] = [
    "_NOUN_", "_._", "_VERB_", "_ADP_", "_DET_", "_ADJ_", "_PRON_", "_ADV_", "_NUM_", "_CONJ_",
    "_PRT_", "_X_",
];
const PART_OF_SPEECH_SUFFIXES: [&str; 12] = [
    "_NOUN", "_.", "_VERB", "_ADP", "_DET", "_ADJ", "_PRON", "_ADV", "_NUM", "_CONJ", "_PRT", "_X",
];

pub fn run() -> Result<()> {
    initialize().context("failed to initialize")?;
    let args = get_args().context("failed to get args")?;
    info!("args: {:?}", &args);
    let pool = new_conn_pool(&args).context("failed to establish conn")?;
    migrate(&pool).context("failed to migrate")?;
    do_one_grams(&args, &pool).context("failed to process one grams")?;
    do_two_grams().context("failed to process two grams")?;
    do_three_grams().context("failed to process three grams")?;
    do_four_grams().context("failed to process four grams")?;
    do_five_grams().context("failed to process five grams")?;
    Ok(())
}

fn initialize() -> Result<()> {
    SimpleLogger::init(LevelFilter::Debug, Config::default())?;
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
        .get_matches();

    let lang: Language = Language::from(matches.value_of("lang").unwrap_or("eng"));
    let parallel = matches
        .value_of("parallel")
        .unwrap_or(&format!("{}", num_cpus::get()))
        .parse()?;
    let dsn = matches
        .value_of("dsn")
        .unwrap_or(&mariadb_dsn())
        .to_string();

    Ok(Args {
        lang,
        parallel,
        dsn,
    })
}

fn mariadb_dsn() -> String {
    format!("mysql://moctane:pw@mariadb:3306/mocword")
}

#[derive(Debug, Error)]
enum NetworkError {
    #[error("failed to establish connection")]
    DBConnectionError(),
}

fn new_conn_pool(args: &Args) -> Result<Pool<ConnectionManager<MysqlConnection>>> {
    let backoff = Backoff::new(8, Duration::from_millis(100), Duration::from_secs(10));

    for duration in &backoff {
        let manager = ConnectionManager::<MysqlConnection>::new(&args.dsn);
        let pool = Pool::builder()
            .max_size(args.parallel as u32)
            .build(manager);

        match pool {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                error!("failed to establish pool: {}", e);
                thread::sleep(duration);
            }
        };
    }

    Err(NetworkError::DBConnectionError())?
}

fn migrate(pool: &Pool<ConnectionManager<MysqlConnection>>) -> Result<()> {
    info!("start: migrate");
    embedded_migrations::run_with_output(&pool.get()?, &mut io::stdout())?;
    info!("end  : migrate");
    Ok(())
}

fn do_one_grams(args: &Args, pool: &Pool<ConnectionManager<MysqlConnection>>) -> Result<()> {
    info!("start: one grams");

    let n = 1;

    let thread_pool = ThreadPool::new(args.parallel);

    for query in gen_queries(args.lang, n).into_iter() {
        let pool = pool.clone();
        thread_pool.execute(move || do_one_gram(pool, query).unwrap());
    }

    thread_pool.join();
    if thread_pool.panic_count() > 0 {
        bail!("panic occured on do_one_grams");
    }

    finalize_one_grams(pool).context("failed to finalize one grams")?;

    info!("end  : one grams");
    Ok(())
}

fn do_one_gram(pool: Pool<ConnectionManager<MysqlConnection>>, query: Query) -> Result<()> {
    if is_fetched_file(&pool, &query)? {
        return Ok(());
    }

    info!("start: one gram {:?}", &query);

    let conn = pool.get()?;
    conn.transaction::<_, diesel::result::Error, _>(|| {
        let mut file = tempfile::tempfile().map_err(|e| {
            error!("{:?}", e);
            diesel::result::Error::RollbackTransaction
        })?;
        download(&query, &mut file)
            .with_context(|| format!("failed to download: {:?}", &query))
            .map_err(|e| {
                error!("{:?}", e);
                diesel::result::Error::RollbackTransaction
            })?;
        file.seek(SeekFrom::Start(0)).map_err(|e| {
            error!("{:?}", e);
            diesel::result::Error::RollbackTransaction
        })?;

        read_and_save(&conn, &query, &mut file)
            .with_context(|| format!("failed to read and save: {:?}", &query))
            .map_err(|e| {
                error!("{:?}", e);
                diesel::result::Error::RollbackTransaction
            })?;
        mark_fetched_file(&conn, &query)
            .with_context(|| format!("failed to mark fetched file: {:?}", &query))
            .map_err(|e| {
                error!("{:?}", e);
                diesel::result::Error::RollbackTransaction
            })?;
        Ok(())
    })?;

    info!("end  : one gram {:?}", &query);

    Ok(())
}

fn finalize_one_grams(pool: &Pool<ConnectionManager<MysqlConnection>>) -> Result<()> {
    info!("start: finalize one grams");

    let conn = pool.get()?;

    conn.transaction::<_, diesel::result::Error, _>(|| {
        use schema::one_grams::dsl;

        let count = dsl::one_grams
            .select(diesel::dsl::count_star())
            .execute(&conn)?;
        if count > 0 {
            return Ok(());
        }

        diesel::sql_query(indoc! {"
            INSERT INTO one_grams (word, score)
            SELECT word, score
            FROM one_gram_scores
            ORDER BY score DESC, word ASC
        "})
        .execute(&conn)?;

        diesel::delete(schema::one_gram_scores::table).execute(&conn)?;

        Ok(())
    })?;

    info!("end  : finalize one grams");

    Ok(())
}

fn do_two_grams() -> Result<()> {
    unimplemented!();
}

fn do_three_grams() -> Result<()> {
    unimplemented!();
}

fn do_four_grams() -> Result<()> {
    unimplemented!();
}

fn do_five_grams() -> Result<()> {
    unimplemented!();
}

fn is_fetched_file(pool: &Pool<ConnectionManager<MysqlConnection>>, query: &Query) -> Result<bool> {
    use schema::fetched_files::dsl;

    let res = dsl::fetched_files
        .filter(dsl::n.eq_all(query.n))
        .filter(dsl::idx.eq_all(query.idx))
        .load::<models::FetchedFile>(&pool.get()?)
        .with_context(|| format!("failed to fetch fetched_files ({:?})", &query))?;

    Ok(res.len() > 0)
}

fn mark_fetched_file(conn: &MysqlConnection, query: &Query) -> Result<()> {
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

fn download(query: &Query, w: &mut impl Write) -> Result<()> {
    info!("start: download {:?}", &query);

    let url = file_url(&query);

    let response =
        reqwest::blocking::get(&url).with_context(|| format!("failed to download {}", &url))?;

    io::copy(&mut BufReader::new(response), &mut BufWriter::new(w))
        .with_context(|| format!("failed to save {:?}", query))?;

    info!("end  : download {:?}", &query);

    Ok(())
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

fn read_and_save(conn: &MysqlConnection, query: &Query, gz_data: &mut impl Read) -> Result<()> {
    info!("start: read and save {:?}", &query);

    let r = GzDecoder::new(gz_data);
    let r = BufReader::new(r);

    let mut entries = vec![];
    for line in r.lines() {
        let line = line?;
        match Entry::new(&line, query.n) {
            Ok(entry) => entries.push(entry),
            Err(_) => continue,
        };

        if entries.len() >= MAX_BUFFER_SIZE {
            save(conn, query, &entries)?;
            entries = vec![];
        }
    }
    if entries.len() > 0 {
        save(conn, query, &entries).with_context(|| format!("failed to save {:?}", &query))?;
    }

    info!("end  : read and save {:?}", &query);

    Ok(())
}

fn save(conn: &MysqlConnection, query: &Query, entries: &[Entry]) -> Result<()> {
    info!("start: save {:?}", &query);

    match query.n {
        1 => save_one_gram_scores(conn, entries)?,
        _ => {
            unimplemented!();
        }
    };

    info!("end  : save {:?}", &query);

    Ok(())
}

fn save_one_gram_scores(conn: &MysqlConnection, entries: &[Entry]) -> Result<()> {
    let values: Vec<_> = entries
        .iter()
        .map(|entry| models::NewOneGramScore {
            word: entry.ngram[0].to_owned(),
            score: entry.score,
        })
        .collect();

    diesel::insert_into(schema::one_gram_scores::table)
        .values(&values)
        .execute(conn)
        .context(format!("failed to save one_gram_scores"))?;

    Ok(())
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

#[derive(Debug)]
struct Query {
    lang: Language,
    n: i64,
    idx: i64,
}

fn gen_queries(lang: Language, n: i64) -> Vec<Query> {
    // (0..total_file_num(lang, n))
    (10..12)
        .into_iter()
        .map(|idx| Query { lang, n, idx })
        .collect()
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

        let ngram: Vec<_> = ngram.split("\t").map(|w| w.to_string()).collect();
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

    #[error("invalid counts ({ngram})")]
    InvalidWord { ngram: String },
}
