use std::io;

use anyhow::{Context, Result};
use backoff::{retry, Error, ExponentialBackoff};
use diesel::Connection;
use diesel::MysqlConnection;
use log::{debug, error, info, warn};
use simplelog::{Config, LevelFilter, SimpleLogger};

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

struct Args {}

fn get_args() -> Result<Args> {
    return Ok(Args {});
    unimplemented!();
}

fn mariadb_dsn() -> String {
    format!("mysql://moctane:pw@mariadb:3306/mocword")
}

fn new_conn(args: &Args) -> Result<MysqlConnection> {
    let op = || match MysqlConnection::establish(&mariadb_dsn()) {
        Ok(conn) => Ok(conn),
        Err(e) => {
            error!("failed to establish: {}", e);
            Err(Error::Transient)
        }
    };

    Ok(retry(&mut ExponentialBackoff::default(), op)?)
}

fn migrate(conn: &MysqlConnection) -> Result<()> {
    info!("start: migrate");
    embedded_migrations::run_with_output(conn, &mut io::stdout())?;
    info!("end  : migrate");
    Ok(())
}

fn do_one_grams() -> Result<()> {
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
