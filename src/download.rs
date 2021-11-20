use std::fmt;
use std::io;
use std::io::{prelude::*, BufReader, BufWriter, SeekFrom};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{prelude::*, Connection, MysqlConnection};
use flate2::read::GzDecoder;
use indoc::indoc;
use log::{error, info};
use lru::LruCache;
use reqwest::blocking::Client;
use simplelog::{Config, LevelFilter, SimpleLogger};
use tempfile::NamedTempFile;
use thiserror::Error;
use threadpool::ThreadPool;

use crate::embedded_migrations;
use crate::models;
use crate::schema;

const DEFAULT_CACHE_LENGTH: usize = 1_000_000;

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
        .unwrap_or(&format!("{}", 6))
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
    format!("mysql://root:rootpw@localhost/mocword")
}

fn new_conn_pool(args: &Args) -> Result<Pool<ConnectionManager<MysqlConnection>>> {
    loop {
        let manager = ConnectionManager::<MysqlConnection>::new(&args.dsn);
        let pool = Pool::builder()
            .max_size(args.parallel as u32)
            .build(manager);

        let pool = match pool {
            Ok(pool) => pool,
            Err(e) => {
                error!("failed to establish pool: {}", e);
                thread::sleep(Duration::from_secs(10));
                continue;
            }
        };

        if let Err(e) = diesel::sql_query("select 1").execute(&pool.get()?) {
            error!("failed to establish pool: {}", e);
            thread::sleep(Duration::from_secs(10));
            continue;
        }

        return Ok(pool);
    }
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
        let mut gz_file = NamedTempFile::new().context("failed to create tempfile")?;
        download(&client, &query, &mut gz_file)
            .with_context(|| format!("failed to download: {:?}", &query))?;
        gz_file
            .seek(SeekFrom::Start(0))
            .context("failed to seek file")?;

        let mut tsv_file = NamedTempFile::new().context("failed to create tempfile")?;
        save_to_tsv(&conn, &query, &mut gz_file, &mut tsv_file, &mut cache)
            .with_context(|| format!("failed to save tsv: {:?}", &query))?;
        tsv_file
            .seek(SeekFrom::Start(0))
            .context("failed to seek file")?;

        save_to_db(&conn, &query, tsv_file.path().to_str().unwrap())
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
        diesel::sql_query("create index idx_one_grams_word on one_grams(word(255))")
            .execute(conn)?;
        diesel::sql_query("create index idx_one_grams_score on one_grams(score)").execute(conn)?;

        Ok(())
    })
}

fn finalize_two_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        diesel::sql_query(
            "create unique index idx_two_grams_ngram on two_grams(prefix_id, suffix_id)",
        )
        .execute(conn)?;
        diesel::sql_query("create index idx_two_grams_score on two_grams(score)").execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table two_grams
                add constraint two_grams_ibfk_prefix_id
                foreign key (prefix_id) references one_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table two_grams
                add constraint two_grams_ibfk_suffix_id
                foreign key (suffix_id) references one_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;

        Ok(())
    })
}

fn finalize_three_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        diesel::sql_query(
            "create unique index idx_three_grams_ngram on three_grams(prefix_id, suffix_id)",
        )
        .execute(conn)?;
        diesel::sql_query("create index idx_three_grams_score on three_grams(score)")
            .execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table three_grams
                add constraint three_grams_ibfk_prefix_id
                foreign key (prefix_id) references two_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table three_grams
                add constraint three_grams_ibfk_suffix_id
                foreign key (suffix_id) references one_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;

        Ok(())
    })
}

fn finalize_four_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        diesel::sql_query(
            "create unique index idx_four_grams_ngram on four_grams(prefix_id, suffix_id)",
        )
        .execute(conn)?;
        diesel::sql_query("create index idx_four_grams_score on four_grams(score)")
            .execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table four_grams
                add constraint four_grams_ibfk_prefix_id
                foreign key (prefix_id) references three_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table four_grams
                add constraint four_grams_ibfk_suffix_id
                foreign key (suffix_id) references one_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;

        Ok(())
    })
}

fn finalize_five_gram(conn: &MysqlConnection) -> Result<()> {
    conn.transaction::<_, anyhow::Error, _>(|| {
        diesel::sql_query(
            "create unique index idx_five_grams_ngram on five_grams(prefix_id, suffix_id)",
        )
        .execute(conn)?;
        diesel::sql_query("create index idx_five_grams_score on five_grams(score)")
            .execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table five_grams
                add constraint five_grams_ibfk_prefix_id
                foreign key (prefix_id) references four_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;
        diesel::sql_query(indoc! {"
            alter table five_grams
                add constraint five_grams_ibfk_suffix_id
                foreign key (suffix_id) references one_grams(id)
                on delete cascade
                on update cascade
        "})
        .execute(conn)?;

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

fn save_to_tsv(
    conn: &MysqlConnection,
    query: &Query,
    gz_data: &mut impl Read,
    tsv_data: &mut impl Write,
    cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
) -> Result<()> {
    info!("start: save to tsv {:?}", &query);

    let r = GzDecoder::new(gz_data);
    let r = BufReader::new(r);
    let mut w = BufWriter::new(tsv_data);

    match query.n {
        1 => {
            for line in r.lines() {
                let entry = match Entry::new(&line?, query.n) {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                writeln!(w, "{}", entry)
                    .with_context(|| format!("failed to save to csv: {:?}", query))?;
            }
        }
        _ => {
            for line in r.lines() {
                let entry = match Entry::new(&line?, query.n) {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let indexed_entry = IndexedEntry::new(conn, &entry, cache)
                    .with_context(|| format!("failed to save to csv: {:?}", query))?;
                match indexed_entry {
                    Some(ent) => {
                        writeln!(w, "{}", ent)
                            .with_context(|| format!("failed to save to csv: {:?}", query))?;
                    }
                    None => continue,
                }
            }
        }
    }

    info!("end  : save to tsv {:?}", &query);

    Ok(())
}

fn save_to_db(conn: &MysqlConnection, query: &Query, tsv_data_filename: &str) -> Result<()> {
    info!("start: save to db {:?}", query);

    let table_name = match query.n {
        1 => "one_grams",
        2 => "two_grams",
        3 => "three_grams",
        4 => "four_grams",
        5 => "five_grams",
        _ => panic!("invalid query: {:?}", query),
    };

    let fields = match query.n {
        1 => "(word, score)",
        _ => "(prefix_id, suffix_id, score)",
    };

    diesel::sql_query(format!(
        "load data infile '{}' ignore into table {} fields terminated by '\\t' {}",
        tsv_data_filename, table_name, fields,
    ))
    .execute(conn)
    .with_context(|| format!("failed to save to db: {:?}", query))?;

    info!("end  : save to db {:?}", query);

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

struct IndexedEntry {
    prefix_id: i64,
    suffix_id: i64,
    score: i64,
}

impl IndexedEntry {
    fn new(
        conn: &MysqlConnection,
        from: &Entry,
        cache: &mut Arc<Mutex<LruCache<String, Option<i64>>>>,
    ) -> Result<Option<IndexedEntry>> {
        let prefix = &from.ngram[..(from.ngram.len() - 1)];
        let suffix = &from.ngram[(from.ngram.len() - 1)..];

        let prefix_id = match get_id(conn, prefix, cache) {
            Ok(Some(id)) => id,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };
        let suffix_id = match get_id(conn, suffix, cache) {
            Ok(Some(id)) => id,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

        Ok(Some(IndexedEntry {
            prefix_id,
            suffix_id,
            score: from.score,
        }))
    }
}

impl fmt::Display for IndexedEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t{}\t{}", self.prefix_id, self.suffix_id, self.score)
    }
}
