use std::io;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use crossbeam::channel;
use diesel::Connection;
use diesel::{r2d2::Pool, MysqlConnection};
use exponential_backoff::Backoff;
use log::{debug, error, info, warn};
use simplelog::{Config, LevelFilter, SimpleLogger};
use thiserror::Error;
use threadpool::ThreadPool;

use crate::embedded_migrations;

pub fn run() -> Result<()> {
    initialize().context("failed to initialize")?;
    let args = get_args().context("failed to get args")?;
    let conn = new_conn(&args).context("failed to establish conn")?;
    migrate(&conn).context("failed to migrate")?;
    do_one_grams().context("failed to process one grams")?;
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

fn new_conn(args: &Args) -> Result<MysqlConnection> {
    let backoff = Backoff::new(8, Duration::from_millis(100), Duration::from_secs(10));

    for duration in &backoff {
        match MysqlConnection::establish(&mariadb_dsn()) {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                error!("failed to establish: {}", e);
                thread::sleep(duration);
            }
        };
    }

    Err(NetworkError::DBConnectionError())?
}

fn migrate(conn: &MysqlConnection) -> Result<()> {
    info!("start: migrate");
    embedded_migrations::run_with_output(conn, &mut io::stdout())?;
    info!("end  : migrate");
    Ok(())
}

fn do_one_grams(args: &Args, conn: &MysqlConnection) -> Result<()> {
    let n = 1;

    let conn = Arc::new(*conn);

    let pool = ThreadPool::new(args.parallel);
    for query in gen_queries(args.lang, n).into_iter() {
        let conn = Arc::clone(&conn);
        pool.execute(move || do_one_gram(conn, query));
    }

    Ok(())
}

fn do_one_gram(conn: Arc<MysqlConnection>, query: Query) {
    unimplemented!();
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

enum Language {
    English,
}

struct Query {}

fn gen_queries(lang: Language, n: i64) -> Vec<Query> {
    unimplemented!();
}
