use crate::embedded_migrations;
use crate::models;
use crate::schema;
use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{prelude::*, Connection, SqliteConnection};
use flate2::read::GzDecoder;
use log::info;
use num_cpus;
use reqwest::blocking::Client;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, prelude::*, BufReader, BufWriter, SeekFrom};
use std::sync::Arc;
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

const BULK_INSERT_SIZE: usize = 10000;

pub fn run() -> Result<()> {
    initialize().context("failed to initalize")?;

    info!("start");

    let args = get_args()?;
    let conn_pool = new_conn_pool(&args).context("failed to establish conn")?;
    migrate(&conn_pool).context("failed to migrate")?;
    download_and_save_all(&args, &conn_pool).context("failed to download and save")?;
    finalize(&conn_pool).context("failed to finalize")?;

    info!("end");
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
        .unwrap_or(&format!("{}", num_cpus::get()))
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
    let cache = get_one_grams_cache(conn_pool).context("failed to get one grams cache")?;
    download_and_save_ngrams(args, conn_pool, cache)
        .context("failed to download and save ngrams")?;

    Ok(())
}

fn download_and_save_one_grams(
    args: &Args,
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<()> {
    let n = 1;

    let client = Client::builder().pool_max_idle_per_host(2).build()?;

    for query in gen_queries(args.lang, n).iter() {
        download_and_save_one_gram(conn_pool, &client, query)
            .with_context(|| format!("failed to download and save {:?}", query))?;
    }

    Ok(())
}

fn download_and_save_one_gram(
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
    client: &Client,
    query: &Query,
) -> Result<()> {
    if is_fetched_file(&conn_pool, &query)? {
        return Ok(());
    }

    info!("start: {:?}", query);

    let mut gz_file = NamedTempFile::new().context("failed to create tempfile")?;
    download(&client, &query, &mut gz_file)
        .with_context(|| format!("failed to download: {:?}", &query))?;
    gz_file
        .seek(SeekFrom::Start(0))
        .context("failed to seek file")?;
    let r = GzDecoder::new(gz_file);
    let r = BufReader::new(r);

    info!("start: save {:?}", query);

    let conn = conn_pool.get()?;
    conn.transaction::<_, anyhow::Error, _>(|| {
        let mut entries = vec![];
        for line in r.lines() {
            let line = line?;
            let entry = match Entry::new(&line, query.n) {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            entries.push(entry);

            if entries.len() >= BULK_INSERT_SIZE {
                save_one_gram(&conn, query, &entries)
                    .with_context(|| format!("failed to save {:?}", query))?;
                entries = vec![];
            }
        }
        if entries.len() > 0 {
            save_one_gram(&conn, query, &entries)
                .with_context(|| format!("failed to save {:?}", query))?;
        }

        mark_fetched_file(&conn, &query)
            .with_context(|| format!("failed to mark fetched file: {:?}", &query))?;
        Ok(())
    })?;

    info!("end  : save {:?}", query);
    info!("end  : {:?}", query);

    Ok(())
}

fn save_one_gram(conn: &SqliteConnection, query: &Query, entries: &[Entry]) -> Result<()> {
    let values: Vec<_> = entries
        .iter()
        .map(|ent| models::NewOneGram {
            word: ent.ngram[0].clone(),
            score: ent.score,
        })
        .collect();

    diesel::insert_into(schema::one_grams::table)
        .values(&values)
        .execute(conn)
        .with_context(|| format!("failed to save one gram {:?}", query))?;

    Ok(())
}

fn download_and_save_ngrams(
    args: &Args,
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
    cache: HashMap<String, i64>,
) -> Result<()> {
    let thread_pool = ThreadPool::new(args.parallel);
    let client = Client::builder().pool_max_idle_per_host(2).build()?;
    let cache = Arc::new(cache);

    for n in 1..=5 {
        for query in gen_queries(args.lang, n).into_iter() {
            let conn_pool = conn_pool.clone();
            let client = client.clone();
            let cache = Arc::clone(&cache);
            thread_pool
                .execute(move || download_and_save_ngram(conn_pool, client, query, cache).unwrap());
        }
    }

    thread_pool.join();
    if thread_pool.panic_count() > 0 {
        bail!("panic occured on download_and_save");
    }

    Ok(())
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

fn download_and_save_ngram(
    conn_pool: Pool<ConnectionManager<SqliteConnection>>,
    client: Client,
    query: Query,
    cache: Arc<HashMap<String, i64>>,
) -> Result<()> {
    if is_fetched_file(&conn_pool, &query)? {
        return Ok(());
    }

    info!("start: {:?}", &query);

    let mut gz_file = NamedTempFile::new().context("failed to create tempfile")?;
    download(&client, &query, &mut gz_file)
        .with_context(|| format!("failed to download: {:?}", &query))?;
    gz_file
        .seek(SeekFrom::Start(0))
        .context("failed to seek file")?;

    let mut tsv_file = NamedTempFile::new().context("failed to create tempfile")?;
    let mut r = GzDecoder::new(gz_file);
    save_to_tsv(&query, &mut r, &mut tsv_file, &cache)
        .with_context(|| format!("failed to save tsv: {:?}", &query))?;
    tsv_file
        .seek(SeekFrom::Start(0))
        .context("failed to seek file")?;

    let conn = conn_pool.get()?;
    conn.transaction::<_, anyhow::Error, _>(|| {
        save_tsv_to_db(&conn, &query, &mut tsv_file)?;
        mark_fetched_file(&conn, &query)?;
        Ok(())
    })?;

    info!("end  : {:?}", &query);

    Ok(())
}

fn save_to_tsv(
    query: &Query,
    r: &mut impl Read,
    w: &mut impl Write,
    cache: &Arc<HashMap<String, i64>>,
) -> Result<()> {
    info!("start: save to tsv {:?}", query);

    let r = BufReader::new(r);

    'outer: for line in r.lines() {
        let line = line?;
        let entry = match Entry::new(&line, query.n) {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let mut ids = vec![];
        for word in entry.ngram.iter() {
            let id = match cache.get(word) {
                Some(id) => *id,
                None => continue 'outer,
            };
            ids.push(id);
        }

        writeln!(
            w,
            "{}\t{}",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join("\t"),
            entry.score
        )?;
    }

    info!("end  : save to tsv {:?}", query);

    Ok(())
}

fn save_tsv_to_db(conn: &SqliteConnection, query: &Query, tsv: &mut impl Read) -> Result<()> {
    info!("start: save to db {:?}", query);

    match query.n {
        2 => save_tsv_to_db_two_gram(conn, query, tsv),
        3 => save_tsv_to_db_three_gram(conn, query, tsv),
        4 => save_tsv_to_db_four_gram(conn, query, tsv),
        5 => save_tsv_to_db_five_gram(conn, query, tsv),
        _ => panic!("invalid ngram: {}", query.n),
    }
    .with_context(|| format!("failed to save tsv {:?}", query))?;

    info!("end  : save to db {:?}", query);

    Ok(())
}

fn save_tsv_to_db_two_gram(
    conn: &SqliteConnection,
    query: &Query,
    tsv: &mut impl Read,
) -> Result<()> {
    let r = BufReader::new(tsv);

    let mut values = vec![];
    for line in r.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        let value = models::NewTwoGram {
            word1: elems.next().unwrap().parse()?,
            word2: elems.next().unwrap().parse()?,
            score: elems.next().unwrap().parse()?,
        };
        values.push(value);

        if values.len() >= BULK_INSERT_SIZE {
            diesel::insert_or_ignore_into(schema::two_grams::table)
                .values(&values)
                .execute(conn)
                .with_context(|| format!("failed to save {:?}", query))?;
            values = vec![];
        }
    }
    if values.len() > 0 {
        diesel::insert_or_ignore_into(schema::two_grams::table)
            .values(&values)
            .execute(conn)
            .with_context(|| format!("failed to save {:?}", query))?;
    }

    Ok(())
}

fn save_tsv_to_db_three_gram(
    conn: &SqliteConnection,
    query: &Query,
    tsv: &mut impl Read,
) -> Result<()> {
    let r = BufReader::new(tsv);

    let mut values = vec![];
    for line in r.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        let value = models::NewThreeGram {
            word1: elems.next().unwrap().parse()?,
            word2: elems.next().unwrap().parse()?,
            word3: elems.next().unwrap().parse()?,
            score: elems.next().unwrap().parse()?,
        };
        values.push(value);

        if values.len() >= BULK_INSERT_SIZE {
            diesel::insert_or_ignore_into(schema::three_grams::table)
                .values(&values)
                .execute(conn)
                .with_context(|| format!("failed to save {:?}", query))?;
            values = vec![];
        }
    }
    if values.len() > 0 {
        diesel::insert_or_ignore_into(schema::three_grams::table)
            .values(&values)
            .execute(conn)
            .with_context(|| format!("failed to save {:?}", query))?;
    }

    Ok(())
}

fn save_tsv_to_db_four_gram(
    conn: &SqliteConnection,
    query: &Query,
    tsv: &mut impl Read,
) -> Result<()> {
    let r = BufReader::new(tsv);

    let mut values = vec![];
    for line in r.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        let value = models::NewFourGram {
            word1: elems.next().unwrap().parse()?,
            word2: elems.next().unwrap().parse()?,
            word3: elems.next().unwrap().parse()?,
            word4: elems.next().unwrap().parse()?,
            score: elems.next().unwrap().parse()?,
        };
        values.push(value);

        if values.len() >= BULK_INSERT_SIZE {
            diesel::insert_or_ignore_into(schema::four_grams::table)
                .values(&values)
                .execute(conn)
                .with_context(|| format!("failed to save {:?}", query))?;
            values = vec![];
        }
    }
    if values.len() > 0 {
        diesel::insert_or_ignore_into(schema::four_grams::table)
            .values(&values)
            .execute(conn)
            .with_context(|| format!("failed to save {:?}", query))?;
    }

    Ok(())
}

fn save_tsv_to_db_five_gram(
    conn: &SqliteConnection,
    query: &Query,
    tsv: &mut impl Read,
) -> Result<()> {
    let r = BufReader::new(tsv);

    let mut values = vec![];
    for line in r.lines() {
        let line = line?;
        let mut elems = line.split("\t");
        let value = models::NewFiveGram {
            word1: elems.next().unwrap().parse()?,
            word2: elems.next().unwrap().parse()?,
            word3: elems.next().unwrap().parse()?,
            word4: elems.next().unwrap().parse()?,
            word5: elems.next().unwrap().parse()?,
            score: elems.next().unwrap().parse()?,
        };
        values.push(value);

        if values.len() >= BULK_INSERT_SIZE {
            diesel::insert_or_ignore_into(schema::five_grams::table)
                .values(&values)
                .execute(conn)
                .with_context(|| format!("failed to save {:?}", query))?;
            values = vec![];
        }
    }
    if values.len() > 0 {
        diesel::insert_or_ignore_into(schema::five_grams::table)
            .values(&values)
            .execute(conn)
            .with_context(|| format!("failed to save {:?}", query))?;
    }

    Ok(())
}

fn get_one_grams_cache(
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<HashMap<String, i64>> {
    use schema::one_grams::dsl;

    info!("start: get one grams cache");

    let mut res = HashMap::new();

    let mut start = 0;
    let width = BULK_INSERT_SIZE as i64;

    loop {
        let one_grams = dsl::one_grams
            .filter(dsl::id.ge(start))
            .filter(dsl::id.lt(start + width))
            .load::<models::OneGram>(&conn_pool.get()?)
            .context("failed to get one grams")?;

        start += width;

        if one_grams.len() == 0 {
            break;
        }

        for one_gram in one_grams.into_iter() {
            res.insert(one_gram.word, one_gram.id);
        }
    }

    info!("end  : get one grams cache");

    Ok(res)
}

fn finalize(conn_pool: &Pool<ConnectionManager<SqliteConnection>>) -> Result<()> {
    info!("start: create idx_one_grams_score");
    diesel::sql_query("create index idx_one_grams_score on one_grams(score)")
        .execute(&conn_pool.get()?)
        .context("failed to create idx_one_grams_score")?;
    info!("end  : create idx_one_grams_score");

    info!("start: create idx_two_grams_score");
    diesel::sql_query("create index idx_two_grams_score on two_grams(score)")
        .execute(&conn_pool.get()?)
        .context("failed to create idx_two_grams_score")?;
    info!("end  : create idx_two_grams_score");

    info!("start: create idx_three_grams_score");
    diesel::sql_query("create index idx_three_grams_score on three_grams(score)")
        .execute(&conn_pool.get()?)
        .context("failed to create idx_three_grams_score")?;
    info!("end  : create idx_three_grams_score");

    info!("start: create idx_four_grams_score");
    diesel::sql_query("create index idx_four_grams_score on four_grams(score)")
        .execute(&conn_pool.get()?)
        .context("failed to create idx_four_grams_score")?;
    info!("end  : create idx_four_grams_score");

    info!("start: create idx_five_grams_score");
    diesel::sql_query("create index idx_five_grams_score on five_grams(score)")
        .execute(&conn_pool.get()?)
        .context("failed to create idx_five_grams_score")?;
    info!("end  : create idx_five_grams_score");

    Ok(())
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
