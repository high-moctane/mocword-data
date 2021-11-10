use std::io;
use std::io::prelude::*;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use crossbeam::channel;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{prelude::*, Connection, MysqlConnection};
use exponential_backoff::Backoff;
use log::{debug, error, info, warn};
use reqwest;
use simplelog::{Config, LevelFilter, SimpleLogger};
use thiserror::Error;
use threadpool::ThreadPool;

use crate::embedded_migrations;
use crate::models;
use crate::schema;

pub fn run() -> Result<()> {
    initialize().context("failed to initialize")?;
    let args = get_args().context("failed to get args")?;
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

struct Args {
    lang: Language,
    parallel: usize,
}

fn get_args() -> Result<Args> {
    unimplemented!();
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
        let manager = ConnectionManager::<MysqlConnection>::new(&mariadb_dsn());
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
    let n = 1;

    let thread_pool = ThreadPool::new(args.parallel);

    for query in gen_queries(args.lang, n).into_iter() {
        let pool = pool.clone();
        thread_pool.execute(move || do_one_gram(pool, query).unwrap());
    }

    Ok(())
}

fn do_one_gram(pool: Pool<ConnectionManager<MysqlConnection>>, query: Query) -> Result<()> {
    if is_fetched_file(&pool, &query)? {
        return Ok(());
    }

    let gz_data =
        download(&pool, &query).with_context(|| format!("failed to download: {:?}", &query))?;

    mark_fetched_file(&pool, &query)?;

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

fn mark_fetched_file(pool: &Pool<ConnectionManager<MysqlConnection>>, query: &Query) -> Result<()> {
    let value = models::NewFetchedFile {
        n: query.n,
        idx: query.idx,
    };

    diesel::insert_into(schema::fetched_files::table)
        .values(&value)
        .execute(&pool.get()?)
        .with_context(|| format!("failed to insert fetched_files: {:?}", query))?;

    Ok(())
}

fn download(pool: &Pool<ConnectionManager<MysqlConnection>>, query: &Query) -> Result<Vec<u8>> {
    let url = file_url(&query);
    let mut body = vec![];

    reqwest::blocking::get(&url)
        .with_context(|| format!("failed to download {}", &url))?
        .read_to_end(&mut body)
        .with_context(|| format!("failed to download {}", &url))?;

    Ok(body)
}

fn file_url(query: &Query) -> String {
    format!(
        "http://storage.googleapis.com/books/ngrams/books/20200217/{}/{}-{:05}-of-{:05}.gz",
        query.lang.url_name(),
        query.n,
        query.idx,
        total_file_num(&query)
    )
}

fn total_file_num(query: &Query) -> i64 {
    match query.lang {
        Language::English => match query.n {
            1 => 24,
            2 => 589,
            3 => 6881,
            4 => 6668,
            5 => 19423,
            _ => panic!("invalid ngram number: {:?}", query),
        },
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

#[derive(Debug)]
struct Query {
    lang: Language,
    n: i64,
    idx: i64,
}

fn gen_queries(lang: Language, n: i64) -> Vec<Query> {
    unimplemented!();
}
