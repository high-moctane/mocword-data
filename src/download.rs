use crate::embedded_migrations;
use anyhow::{Context, Result};
use clap::{App, Arg};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{prelude::*, Connection, SqliteConnection};
use log::info;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::io::{self, prelude::*};

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
    download_and_save(&args, &conn_pool).context("failed to download and save")?;
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

fn download_and_save(
    args: &Args,
    conn_pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<()> {
    download_and_save_one_grams(args, conn_pool)
        .context("failed to download and save one grams")?;
    let cache = get_one_grams_cache(args, conn_pool).context("failed to get one grams cache")?;
    for i in 2..=5 {
        download_and_save_ngrams(args, conn_pool).context("failed to download and save ngrams")?;
    }

    Ok(())
}

fn download_and_save_one_grams(args, conn_pool) -> Result<()> {
    unimplemented!();
}

fn finalize(args: &Args, conn_pool: &Pool<ConnectionManager<SqliteConnection>>) -> Result<()> {
    unimplemented!();
}
