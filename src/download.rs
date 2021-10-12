use anyhow::Result;

use diesel::prelude::*;

use diesel::sqlite::SqliteConnection;

use crate::models::*;
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

    let new_word = NewWord { word: "powa" };
    let new_one_gram = NewOneGram { word1_id: 1 };

    diesel::sql_query("pragma foreign_keys = on;").execute(&conn)?;

    diesel::insert_into(schema::words::table)
        .values(&new_word)
        .execute(&conn)?;
    diesel::insert_into(schema::one_grams::table)
        .values(&new_one_gram)
        .execute(&conn)?;

    println!("Hello, download!");
    Ok(())
}
