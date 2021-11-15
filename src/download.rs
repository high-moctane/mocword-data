use std::io;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, SeekFrom};
use std::sync::{Arc, Mutex};
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
use lru::LruCache;
use num_cpus;
use reqwest::blocking::Client;
use simplelog::{Config, LevelFilter, SimpleLogger};
use tempfile;
use thiserror::Error;
use threadpool::ThreadPool;

use crate::embedded_migrations;
use crate::models;
use crate::schema;

const MAX_BUFFER_SIZE: usize = 5000;
const DEFAULT_CACHE_LENGTH: usize = 100_000_000;

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
    (1..=5).try_for_each(|n| {
        download_and_save_all(&args, &pool, n)
            .with_context(|| format!("failed to download and save all {}-gram", n))
    })?;

    Ok(())
}

fn initialize() -> Result<()> {
    SimpleLogger::init(LevelFilter::Info, Config::default())?;
    Ok(())
}

#[derive(Debug)]
struct Args {
    lang: Language,
    parallel: usize,
    dsn: String,
    cache: usize,
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
    let parallel = matches
        .value_of("parallel")
        .unwrap_or(&format!("{}", num_cpus::get()))
        .parse()?;
    let dsn = matches
        .value_of("dsn")
        .unwrap_or(&mariadb_dsn())
        .to_string();
    let cache = matches
        .value_of("cache")
        .unwrap_or(&format!("{}", DEFAULT_CACHE_LENGTH))
        .parse()?;

    Ok(Args {
        lang,
        parallel,
        dsn,
        cache,
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

fn download_and_save_all(
    args: &Args,
    pool: &Pool<ConnectionManager<MysqlConnection>>,
    n: i64,
) -> Result<()> {
    info!("start: {}-grams download and save all", n);

    let thread_pool = ThreadPool::new(args.parallel);
    let client = Client::builder().pool_max_idle_per_host(2).build()?;

    {
        let cache = Arc::new(Mutex::new(LruCache::new(args.cache)));

        for query in gen_queries(args.lang, n).into_iter() {
            let pool = pool.clone();
            let client = client.clone();
            let cache = Arc::clone(&cache);
            thread_pool.execute(move || download_and_save(pool, client, query, cache).unwrap());
        }

        thread_pool.join();
        if thread_pool.panic_count() > 0 {
            bail!("panic occured on download_and_save");
        }
    }

    finalize(pool, n).with_context(|| format!("failed to finalize {}-gram", n))?;

    info!("end  : {}-grams download and save", n);

    Ok(())
}

fn download_and_save(
    pool: Pool<ConnectionManager<MysqlConnection>>,
    client: Client,
    query: Query,
    mut cache: Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
    if is_fetched_file(&pool, &query)? {
        return Ok(());
    }

    info!("start: download and save {:?}", &query);

    let conn = pool.get()?;
    conn.transaction::<_, anyhow::Error, _>(|| {
        let mut file = tempfile::tempfile().context("failed to create tempfile")?;
        download(&client, &query, &mut file)
            .with_context(|| format!("failed to download: {:?}", &query))?;
        file.seek(SeekFrom::Start(0))
            .context("failed to seek file")?;

        read_and_save(&conn, &query, &mut file, &mut cache)
            .with_context(|| format!("failed to read and save: {:?}", &query))?;
        mark_fetched_file(&conn, &query)
            .with_context(|| format!("failed to mark fetched file: {:?}", &query))?;
        Ok(())
    })?;

    info!("end  : download and save {:?}", &query);

    Ok(())
}

fn finalize(pool: &Pool<ConnectionManager<MysqlConnection>>, n: i64) -> Result<()> {
    info!("start: finalize {}-gram", n);

    let conn = pool.get()?;
    match n {
        1 => finalize_one_gram(&conn),
        2 => finalize_two_gram(&conn),
        3 => finalize_three_gram(&conn),
        4 => finalize_four_gram(&conn),
        5 => finalize_five_gram(&conn),
        _ => panic!("invalid {}-gram", n),
    }
    .with_context(|| format!("failed to finalize {}-gram", n))?;

    info!("end  : finalize {}-gram", n);

    Ok(())
}

fn finalize_one_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        use schema::one_grams::dsl;

        let count_result = dsl::one_grams.first::<models::OneGram>(conn);
        if let Ok(_) = count_result {
            return Ok(());
        }

        diesel::sql_query(indoc! {"
            INSERT INTO one_grams
            SELECT
                DENSE_RANK() OVER (ORDER BY score DESC, word ASC) as id,
                word,
                score
            FROM one_gram_scores
        "})
        .execute(conn)?;

        diesel::delete(schema::one_gram_scores::table).execute(conn)?;

        Ok(())
    })
}

fn finalize_two_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        use schema::two_grams::dsl;

        let count_result = dsl::two_grams.first::<models::TwoGram>(conn);
        if let Ok(_) = count_result {
            return Ok(());
        }

        diesel::sql_query(indoc! {"
            INSERT INTO two_grams
            SELECT
                DENSE_RANK() OVER (ORDER BY score DESC, word ASC) as id,
                prefix_id,
                suffix_id,
                score
            FROM two_gram_scores
        "})
        .execute(conn)?;

        diesel::delete(schema::two_gram_scores::table).execute(conn)?;

        Ok(())
    })
}

fn finalize_three_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        use schema::three_grams::dsl;

        let count_result = dsl::three_grams.first::<models::ThreeGram>(conn);
        if let Ok(_) = count_result {
            return Ok(());
        }

        diesel::sql_query(indoc! {"
            INSERT INTO three_grams
            SELECT
                DENSE_RANK() OVER (ORDER BY score DESC, word ASC) as id,
                prefix_id,
                suffix_id,
                score
            FROM three_gram_scores
        "})
        .execute(conn)?;

        diesel::delete(schema::three_gram_scores::table).execute(conn)?;

        Ok(())
    })
}

fn finalize_four_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        use schema::four_grams::dsl;

        let count_result = dsl::four_grams.first::<models::FourGram>(conn);
        if let Ok(_) = count_result {
            return Ok(());
        }

        diesel::sql_query(indoc! {"
            INSERT INTO four_grams
            SELECT
                DENSE_RANK() OVER (ORDER BY score DESC, word ASC) as id,
                prefix_id,
                suffix_id,
                score
            FROM four_gram_scores
        "})
        .execute(conn)?;

        diesel::delete(schema::four_gram_scores::table).execute(conn)?;

        Ok(())
    })
}

fn finalize_five_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        use schema::five_grams::dsl;

        let count_result = dsl::five_grams.first::<models::FiveGram>(conn);
        if let Ok(_) = count_result {
            return Ok(());
        }

        diesel::sql_query(indoc! {"
            INSERT INTO five_grams
            SELECT
                DENSE_RANK() OVER (ORDER BY score DESC, word ASC) as id,
                prefix_id,
                suffix_id,
                score
            FROM five_gram_scores
        "})
        .execute(conn)?;

        diesel::delete(schema::five_gram_scores::table).execute(conn)?;

        Ok(())
    })
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

fn read_and_save(
    conn: &MysqlConnection,
    query: &Query,
    gz_data: &mut impl Read,
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
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
            save(conn, query, &entries, cache)?;
            entries = vec![];
        }
    }
    if entries.len() > 0 {
        save(conn, query, &entries, cache)
            .with_context(|| format!("failed to save {:?}", &query))?;
    }

    info!("end  : read and save {:?}", &query);

    Ok(())
}

fn save(
    conn: &MysqlConnection,
    query: &Query,
    entries: &[Entry],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
    match query.n {
        1 => save_one_gram_scores(conn, entries),
        2 => save_two_gram_scores(conn, entries, cache),
        3 => save_three_gram_scores(conn, entries, cache),
        4 => save_four_gram_scores(conn, entries, cache),
        5 => save_five_gram_scores(conn, entries, cache),
        _ => panic!("invalid query: {:?}", query),
    }
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

fn save_two_gram_scores(
    conn: &MysqlConnection,
    entries: &[Entry],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
    let mut values = vec![];
    for entry in entries.iter() {
        let prefix_id = match get_id(conn, &entry.ngram[..1], cache)? {
            Some(id) => id,
            None => continue,
        };
        let suffix_id = match get_id(conn, &entry.ngram[1..], cache)? {
            Some(id) => id,
            None => continue,
        };

        let value = models::NewTwoGramScore {
            prefix_id,
            suffix_id,
            score: entry.score,
        };
        values.push(value);
    }

    diesel::insert_into(schema::two_gram_scores::table)
        .values(&values)
        .execute(conn)
        .context(format!("failed to save two_gram_scores"))?;

    Ok(())
}

fn save_three_gram_scores(
    conn: &MysqlConnection,
    entries: &[Entry],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
    let mut values = vec![];
    for entry in entries.iter() {
        let prefix_id = match get_id(conn, &entry.ngram[..2], cache)? {
            Some(id) => id,
            None => continue,
        };
        let suffix_id = match get_id(conn, &entry.ngram[2..], cache)? {
            Some(id) => id,
            None => continue,
        };

        let value = models::NewThreeGramScore {
            prefix_id,
            suffix_id,
            score: entry.score,
        };
        values.push(value);
    }

    diesel::insert_into(schema::three_gram_scores::table)
        .values(&values)
        .execute(conn)
        .context(format!("failed to save three_gram_scores"))?;

    Ok(())
}

fn save_four_gram_scores(
    conn: &MysqlConnection,
    entries: &[Entry],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
    let mut values = vec![];
    for entry in entries.iter() {
        let prefix_id = match get_id(conn, &entry.ngram[..3], cache)? {
            Some(id) => id,
            None => continue,
        };
        let suffix_id = match get_id(conn, &entry.ngram[3..], cache)? {
            Some(id) => id,
            None => continue,
        };

        let value = models::NewFourGramScore {
            prefix_id,
            suffix_id,
            score: entry.score,
        };
        values.push(value);
    }

    diesel::insert_into(schema::four_gram_scores::table)
        .values(&values)
        .execute(conn)
        .context(format!("failed to save four_gram_scores"))?;

    Ok(())
}

fn save_five_gram_scores(
    conn: &MysqlConnection,
    entries: &[Entry],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
    let mut values = vec![];
    for entry in entries.iter() {
        let prefix_id = match get_id(conn, &entry.ngram[..4], cache)? {
            Some(id) => id,
            None => continue,
        };
        let suffix_id = match get_id(conn, &entry.ngram[4..], cache)? {
            Some(id) => id,
            None => continue,
        };

        let value = models::NewFiveGramScore {
            prefix_id,
            suffix_id,
            score: entry.score,
        };
        values.push(value);
    }

    diesel::insert_into(schema::five_gram_scores::table)
        .values(&values)
        .execute(conn)
        .context(format!("failed to save five_gram_scores"))?;

    Ok(())
}

fn get_id(
    conn: &MysqlConnection,
    words: &[String],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<Option<i64>> {
    let line = words.join(" ");

    {
        if let Some(res) = cache.lock().unwrap().get(&line) {
            return Ok(*res);
        }
    }

    let opt_id = match words.len() {
        1 => get_one_gram_id(conn, &words[0]),
        2 => get_two_gram_id(conn, words, cache),
        3 => get_three_gram_id(conn, words, cache),
        4 => get_four_gram_id(conn, words, cache),
        5 => get_five_gram_id(conn, words, cache),
        _ => panic!("invalid words {:?}", words),
    }
    .with_context(|| format!("failed to get id {}", &line))?;

    {
        cache.lock().unwrap().put(line, opt_id);
    }

    Ok(opt_id)
}

fn get_one_gram_id(conn: &MysqlConnection, word: &str) -> Result<Option<i64>> {
    use schema::one_grams::dsl;

    let res = dsl::one_grams
        .filter(dsl::word.eq_all(word))
        .first::<models::OneGram>(conn)
        .with_context(|| format!("failed to get id {}", word));

    match res {
        Ok(val) => Ok(Some(val.id)),
        Err(e) => match is_not_found_error(&e) {
            true => Ok(None),
            false => Err(e),
        },
    }
}

fn get_two_gram_id(
    conn: &MysqlConnection,
    words: &[String],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<Option<i64>> {
    use schema::two_grams::dsl;

    let prefix_id = match get_id(conn, &words[..1], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };
    let suffix_id = match get_id(conn, &words[1..], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };

    let res = dsl::two_grams
        .filter(dsl::prefix_id.eq_all(prefix_id))
        .filter(dsl::suffix_id.eq_all(suffix_id))
        .first::<models::TwoGram>(conn)
        .with_context(|| format!("failed to get id {:?}", words));

    match res {
        Ok(val) => Ok(Some(val.id)),
        Err(e) => match is_not_found_error(&e) {
            true => Ok(None),
            false => Err(e),
        },
    }
}

fn get_three_gram_id(
    conn: &MysqlConnection,
    words: &[String],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<Option<i64>> {
    use schema::three_grams::dsl;

    let prefix_id = match get_id(conn, &words[..2], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };
    let suffix_id = match get_id(conn, &words[2..], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };

    let res = dsl::three_grams
        .filter(dsl::prefix_id.eq_all(prefix_id))
        .filter(dsl::suffix_id.eq_all(suffix_id))
        .first::<models::ThreeGram>(conn)
        .with_context(|| format!("failed to get id {:?}", words));

    match res {
        Ok(val) => Ok(Some(val.id)),
        Err(e) => match is_not_found_error(&e) {
            true => Ok(None),
            false => Err(e),
        },
    }
}

fn get_four_gram_id(
    conn: &MysqlConnection,
    words: &[String],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<Option<i64>> {
    use schema::four_grams::dsl;

    let prefix_id = match get_id(conn, &words[..3], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };
    let suffix_id = match get_id(conn, &words[3..], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };

    let res = dsl::four_grams
        .filter(dsl::prefix_id.eq_all(prefix_id))
        .filter(dsl::suffix_id.eq_all(suffix_id))
        .first::<models::FourGram>(conn)
        .with_context(|| format!("failed to get id {:?}", words));

    match res {
        Ok(val) => Ok(Some(val.id)),
        Err(e) => match is_not_found_error(&e) {
            true => Ok(None),
            false => Err(e),
        },
    }
}

fn get_five_gram_id(
    conn: &MysqlConnection,
    words: &[String],
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<Option<i64>> {
    use schema::five_grams::dsl;

    let prefix_id = match get_id(conn, &words[..4], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };
    let suffix_id = match get_id(conn, &words[4..], cache)? {
        Some(id) => id,
        None => return Ok(None),
    };

    let res = dsl::five_grams
        .filter(dsl::prefix_id.eq_all(prefix_id))
        .filter(dsl::suffix_id.eq_all(suffix_id))
        .first::<models::FiveGram>(conn)
        .with_context(|| format!("failed to get id {:?}", words));

    match res {
        Ok(val) => Ok(Some(val.id)),
        Err(e) => match is_not_found_error(&e) {
            true => Ok(None),
            false => Err(e),
        },
    }
}

fn is_not_found_error(err: &anyhow::Error) -> bool {
    match err.downcast_ref::<diesel::result::Error>() {
        Some(diesel::result::Error::NotFound) => true,
        _ => false,
    }
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
    (0..total_file_num(lang, n))
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
