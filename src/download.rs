use anyhow::Result;
use env_logger;
use log::info;

pub fn run() -> Result<()> {
    env_logger::init();
    let parsed_args = parse_args();
    migrate();
    do_one_grams();
    let wordidx = get_wordidx();
    do_two_to_five_grams();
    finalize();
    Ok(())
}

fn parse_args() {}

fn migrate() {}

fn do_one_grams() {}

fn get_wordidx() {}

fn do_two_to_five_grams() {}

fn finalize() {}
