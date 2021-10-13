use anyhow::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use thiserror::Error;

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

#[derive(Debug, PartialEq, Eq)]
struct Entry(i16, i64, i64);

fn save_line(conn: &SqliteConnection, line: &str) -> Result<()> {
    unimplemented!();

    let (ngram, entries) = parse_line(line)?;
    let ngram_record = save_ngram(conn, &ngram)?;
    save_entries(conn, &ngram_record, &entries)?;
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("invalid line: {0}")]
    InvalidLine(String),

    #[error("invalid entry: {0}")]
    InvalidEntry(String),
}

fn parse_line(line: &str) -> Result<(Ngram, Vec<Entry>)> {
    let ngram_entries: Vec<&str> = line.split("\t").collect();
    if ngram_entries.len() != 2 {
        return Err(DownloadError::InvalidLine(line.to_string()))?;
    }

    let ngram = parse_ngram(ngram_entries[0]);
    let entries = parse_entries(ngram_entries[1])?;

    Ok((ngram, entries))
}

fn parse_ngram(ngram_vec: &str) -> Vec<String> {
    ngram_vec.split(" ").map(|w| w.to_string()).collect()
}

fn parse_entries(entries_line: &str) -> Result<Vec<Entry>> {
    let mut res = Vec::new();
    for s in entries_line.split(" ") {
        res.push(parse_entry(s)?);
    }
    Ok(res)
}

fn parse_entry(entry_str: &str) -> Result<Entry> {
    let elems: Vec<&str> = entry_str.split(",").collect();
    if elems.len() != 3 {
        return Err(DownloadError::InvalidEntry(entry_str.to_string()))?;
    }

    Ok(Entry(
        elems[0].parse()?,
        elems[1].parse()?,
        elems[2].parse()?,
    ))
}

fn save_ngram(conn: &SqliteConnection, ngram: &Ngram) -> Result<models::Ngram> {
    let word_records = save_words(conn, ngram)?;
    unimplemented!();
}

fn save_words(conn: &SqliteConnection, ngram: &Ngram) -> Result<Vec<models::Word>> {
    use schema::words::dsl;

    let new_words: Vec<models::NewWord> = ngram
        .iter()
        .map(|w| models::NewWord {
            word: w.to_string(),
        })
        .collect();

    diesel::insert_or_ignore_into(dsl::words)
        .values(&new_words)
        .execute(conn)?;

    let query = schema::words::dsl::words;
    Ok(match ngram.len() {
        1 => query
            .filter(dsl::word.eq_all(&ngram[0]))
            .load::<models::Word>(conn)?,
        2 => query
            .filter(dsl::word.eq_all(&ngram[0]))
            .or_filter(dsl::word.eq_all(&ngram[1]))
            .load::<models::Word>(conn)?,
        3 => query
            .filter(dsl::word.eq_all(&ngram[0]))
            .or_filter(dsl::word.eq_all(&ngram[1]))
            .or_filter(dsl::word.eq_all(&ngram[2]))
            .load::<models::Word>(conn)?,
        4 => query
            .filter(dsl::word.eq_all(&ngram[0]))
            .or_filter(dsl::word.eq_all(&ngram[1]))
            .or_filter(dsl::word.eq_all(&ngram[2]))
            .or_filter(dsl::word.eq_all(&ngram[3]))
            .load::<models::Word>(conn)?,
        5 => query
            .filter(dsl::word.eq_all(&ngram[0]))
            .or_filter(dsl::word.eq_all(&ngram[1]))
            .or_filter(dsl::word.eq_all(&ngram[2]))
            .or_filter(dsl::word.eq_all(&ngram[3]))
            .or_filter(dsl::word.eq_all(&ngram[4]))
            .load::<models::Word>(conn)?,
        _ => panic!("invalid ngram: {:?}", &ngram),
    })
}

fn save_entries(
    conn: &SqliteConnection,
    ngram_record: &models::Ngram,
    entries: &Vec<Entry>,
) -> Result<()> {
    unimplemented!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        // OK
        let input = "hello world\t2012,195943,849381 2013,598483,57483 2014,483584,4731";
        let want_ngram: Vec<String> = vec!["hello".to_string(), "world".to_string()];
        let want_entries = vec![
            Entry(2012, 195943, 849381),
            Entry(2013, 598483, 57483),
            Entry(2014, 483584, 4731),
        ];

        let (got_ngram, got_entries) = parse_line(&input).unwrap();
        assert_eq!(want_ngram.len(), got_ngram.len());
        for i in 0..want_ngram.len() {
            assert_eq!(want_ngram[i], got_ngram[i].to_string());
        }
        assert_eq!(&want_entries[..], &got_entries[..]);

        // NG
        assert!(parse_line("hello world 1773,2").is_err());
        assert!(parse_line("hello world 1773,2,5 143").is_err());
    }
}
