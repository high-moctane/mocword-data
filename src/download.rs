use std::path::Path;

use anyhow::Result;
use clap::{App, Arg, SubCommand};
use env_logger;
use log::{debug, info};

#[derive(Debug)]
struct Args {
    lang: String,
    dir: String,
}

pub fn run() -> Result<()> {
    env_logger::init();
    let parsed_args = parse_args();
    do_one_grams();
    let wordidx = get_wordidx();
    do_two_to_five_grams();
    finalize();
    Ok(())
}

fn parse_args() -> Args {
    let languages = vec!["eng"];

    let matches = App::new("Mocword Download")
        .author("high-moctane <high.moctane@gmail.com>")
        .about("Download and build mocword ngram data")
        .arg(
            Arg::with_name("language")
                .short("l")
                .long("lang")
                .value_name("LANG")
                .help(format!("Sets a language (default: eng): {}", languages.join(", ")).as_str())
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dir")
                .long("dir")
                .short("d")
                .value_name("DIR")
                .help("Destination directory path")
                .takes_value(true),
        )
        .get_matches();

    let lang = matches
        .value_of("language")
        .unwrap_or_else(|| "eng")
        .to_string();
    let dir = matches.value_of("dir").unwrap_or_else(|| ".").to_string();

    Args { lang, dir }
}

fn do_one_grams() {}

fn get_wordidx() {}

fn do_two_to_five_grams() {}

fn finalize() {}
