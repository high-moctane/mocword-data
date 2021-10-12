use anyhow::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::models;
use crate::schema;

#[derive(Debug)]
enum Language {
    English,
    AmericanEnglish,
    BritishEnglish,
    EnglishFiction,
    Chinese,
    French,
    German,
    Hebrew,
    Italian,
    Russian,
    Spanish,
}

impl Language {
    fn parse(lang_name: &str) -> Language {
        match &*lang_name.to_lowercase() {
            "eng" => Language::English,
            _ => unimplemented!("not implemented language"),
        }
    }

    fn url_name(&self) -> String {
        match self {
            Language::English => "eng".to_string(),
            _ => unimplemented!("not implemented language"),
        }
    }
}

fn total_file_num(lang: &Language, n: i8) -> i16 {
    match lang {
        Language::English => match n {
            1 => 24,
            2 => 589,
            3 => 6881,
            4 => 6668,
            5 => 19423,
            _ => panic!("invalid ngram number: {}", n),
        },
        _ => unimplemented!("not implemented language: {:?}", lang),
    }
}

fn gz_url(lang: &Language, n: i8, idx: i16) -> String {
    let total = total_file_num(lang, n);

    format!(
        "http://storage.googleapis.com/books/ngrams/books/20200217/{}/{}-{}-of-{}.gz",
        lang.url_name(),
        n,
        idx,
        total
    )
}

pub fn run() -> Result<()> {
    let conn = SqliteConnection::establish("build/download.sqlite")?;

    println!("Hello, download!");
    Ok(())
}

type Ngram = Vec<String>;
struct Entry(i16, i64, i64);

fn save_line(conn: &SqliteConnection, line: &str) -> Result<()> {
    unimplemented!();

    let (ngram, entries) = parse_line(line)?;
    let ngram_record = save_ngram(conn, &ngram)?;
    save_entries(conn, &ngram_record, &entries)?;
}

fn parse_line(line: &str) -> Result<(Ngram, Vec<Entry>)> {
    unimplemented!();
}

fn save_ngram(conn: &SqliteConnection, ngram: &Ngram) -> Result<models::Ngram> {
    let word_records = save_words(conn, ngram)?;
    unimplemented!();
}

fn save_words<'a>(conn: &'a SqliteConnection, ngram: &'a Ngram) -> Result<Vec<models::Word<'a>>> {
    unimplemented!();
}

fn save_entries(
    conn: &SqliteConnection,
    ngram_record: &models::Ngram,
    entries: &Vec<Entry>,
) -> Result<()> {
    unimplemented!();
}
