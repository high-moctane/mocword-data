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

// year, match_count, volume_count
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

    #[error("invalid ngram: {0:?}")]
    InvalidNgram(Ngram),

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

fn save_ngram(conn: &SqliteConnection, ngram: &Ngram) -> Result<Box<dyn models::Ngram>> {
    let word_records = save_words(conn, ngram)?;

    Ok(match ngram.len() {
        1 => save_one_gram(conn, &word_records)?,
        2 => save_two_gram(conn, &word_records)?,
        3 => save_three_gram(conn, &word_records)?,
        4 => save_four_gram(conn, &word_records)?,
        5 => save_five_gram(conn, &word_records)?,
        _ => Err(DownloadError::InvalidNgram(ngram.clone()))?,
    })
}

fn save_one_gram(
    conn: &SqliteConnection,
    word_records: &Vec<models::Word>,
) -> Result<Box<models::OneGram>> {
    use schema::one_grams::dsl;

    let one_gram = models::NewOneGram {
        word1_id: word_records[0].id,
    };

    diesel::insert_or_ignore_into(dsl::one_grams)
        .values(&one_gram)
        .execute(conn)?;

    Ok(Box::new(
        dsl::one_grams
            .filter(dsl::word1_id.eq_all(one_gram.word1_id))
            .limit(1)
            .load::<models::OneGram>(conn)?[0],
    ))
}

fn save_two_gram(
    conn: &SqliteConnection,
    word_records: &Vec<models::Word>,
) -> Result<Box<models::TwoGram>> {
    use schema::two_grams::dsl;

    let two_gram = models::NewTwoGram {
        word1_id: word_records[0].id,
        word2_id: word_records[1].id,
    };

    diesel::insert_or_ignore_into(dsl::two_grams)
        .values(&two_gram)
        .execute(conn)?;

    Ok(Box::new(
        dsl::two_grams
            .filter(dsl::word1_id.eq_all(two_gram.word1_id))
            .filter(dsl::word2_id.eq_all(two_gram.word2_id))
            .limit(1)
            .load::<models::TwoGram>(conn)?[0],
    ))
}

fn save_three_gram(
    conn: &SqliteConnection,
    word_records: &Vec<models::Word>,
) -> Result<Box<models::ThreeGram>> {
    use schema::three_grams::dsl;

    let three_gram = models::NewThreeGram {
        word1_id: word_records[0].id,
        word2_id: word_records[1].id,
        word3_id: word_records[2].id,
    };

    diesel::insert_or_ignore_into(dsl::three_grams)
        .values(&three_gram)
        .execute(conn)?;

    Ok(Box::new(
        dsl::three_grams
            .filter(dsl::word1_id.eq_all(three_gram.word1_id))
            .filter(dsl::word2_id.eq_all(three_gram.word2_id))
            .filter(dsl::word3_id.eq_all(three_gram.word3_id))
            .limit(1)
            .load::<models::ThreeGram>(conn)?[0],
    ))
}

fn save_four_gram(
    conn: &SqliteConnection,
    word_records: &Vec<models::Word>,
) -> Result<Box<models::FourGram>> {
    use schema::four_grams::dsl;

    let four_gram = models::NewFourGram {
        word1_id: word_records[0].id,
        word2_id: word_records[1].id,
        word3_id: word_records[2].id,
        word4_id: word_records[3].id,
    };

    diesel::insert_or_ignore_into(dsl::four_grams)
        .values(&four_gram)
        .execute(conn)?;

    Ok(Box::new(
        dsl::four_grams
            .filter(dsl::word1_id.eq_all(four_gram.word1_id))
            .filter(dsl::word2_id.eq_all(four_gram.word2_id))
            .filter(dsl::word3_id.eq_all(four_gram.word3_id))
            .filter(dsl::word4_id.eq_all(four_gram.word4_id))
            .limit(1)
            .load::<models::FourGram>(conn)?[0],
    ))
}

fn save_five_gram(
    conn: &SqliteConnection,
    word_records: &Vec<models::Word>,
) -> Result<Box<models::FiveGram>> {
    use schema::five_grams::dsl;

    let five_gram = models::NewFiveGram {
        word1_id: word_records[0].id,
        word2_id: word_records[1].id,
        word3_id: word_records[2].id,
        word4_id: word_records[3].id,
        word5_id: word_records[4].id,
    };

    diesel::insert_or_ignore_into(dsl::five_grams)
        .values(&five_gram)
        .execute(conn)?;

    Ok(Box::new(
        dsl::five_grams
            .filter(dsl::word1_id.eq_all(five_gram.word1_id))
            .filter(dsl::word2_id.eq_all(five_gram.word2_id))
            .filter(dsl::word3_id.eq_all(five_gram.word3_id))
            .filter(dsl::word4_id.eq_all(five_gram.word4_id))
            .filter(dsl::word5_id.eq_all(five_gram.word5_id))
            .limit(1)
            .load::<models::FiveGram>(conn)?[0],
    ))
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
    ngram_record: &Box<dyn models::Ngram>,
    entries: &Vec<Entry>,
) -> Result<()> {
    Ok(match ngram_record.n() {
        1 => save_one_gram_entries(conn, ngram_record.get_id(), entries)?,
        2 => save_two_gram_entries(conn, ngram_record.get_id(), entries)?,
        3 => save_three_gram_entries(conn, ngram_record.get_id(), entries)?,
        4 => save_four_gram_entries(conn, ngram_record.get_id(), entries)?,
        5 => save_five_gram_entries(conn, ngram_record.get_id(), entries)?,
        _ => Err(DownloadError::InvalidEntry(format!("{:?}", entries)))?,
    })
}

fn save_one_gram_entries(
    conn: &SqliteConnection,
    ngram_id: i64,
    entries: &Vec<Entry>,
) -> Result<()> {
    use schema::one_gram_entries::dsl;

    let mut entry_records = vec![];
    for ent in entries {
        entry_records.push(models::NewOneGramEntry {
            one_gram_id: ngram_id,
            year: ent.0,
            match_count: ent.1,
            volume_count: ent.2,
        })
    }

    diesel::insert_or_ignore_into(dsl::one_gram_entries)
        .values(&entry_records)
        .execute(conn)?;

    Ok(())
}

fn save_two_gram_entries(
    conn: &SqliteConnection,
    ngram_id: i64,
    entries: &Vec<Entry>,
) -> Result<()> {
    use schema::two_gram_entries::dsl;

    let mut entry_records = vec![];
    for ent in entries {
        entry_records.push(models::NewTwoGramEntry {
            two_gram_id: ngram_id,
            year: ent.0,
            match_count: ent.1,
            volume_count: ent.2,
        })
    }

    diesel::insert_or_ignore_into(dsl::two_gram_entries)
        .values(&entry_records)
        .execute(conn)?;

    Ok(())
}

fn save_three_gram_entries(
    conn: &SqliteConnection,
    ngram_id: i64,
    entries: &Vec<Entry>,
) -> Result<()> {
    use schema::three_gram_entries::dsl;

    let mut entry_records = vec![];
    for ent in entries {
        entry_records.push(models::NewThreeGramEntry {
            three_gram_id: ngram_id,
            year: ent.0,
            match_count: ent.1,
            volume_count: ent.2,
        })
    }

    diesel::insert_or_ignore_into(dsl::three_gram_entries)
        .values(&entry_records)
        .execute(conn)?;

    Ok(())
}

fn save_four_gram_entries(
    conn: &SqliteConnection,
    ngram_id: i64,
    entries: &Vec<Entry>,
) -> Result<()> {
    use schema::four_gram_entries::dsl;

    let mut entry_records = vec![];
    for ent in entries {
        entry_records.push(models::NewFourGramEntry {
            four_gram_id: ngram_id,
            year: ent.0,
            match_count: ent.1,
            volume_count: ent.2,
        })
    }

    diesel::insert_or_ignore_into(dsl::four_gram_entries)
        .values(&entry_records)
        .execute(conn)?;

    Ok(())
}

fn save_five_gram_entries(
    conn: &SqliteConnection,
    ngram_id: i64,
    entries: &Vec<Entry>,
) -> Result<()> {
    use schema::five_gram_entries::dsl;

    let mut entry_records = vec![];
    for ent in entries {
        entry_records.push(models::NewFiveGramEntry {
            five_gram_id: ngram_id,
            year: ent.0,
            match_count: ent.1,
            volume_count: ent.2,
        })
    }

    diesel::insert_or_ignore_into(dsl::five_gram_entries)
        .values(&entry_records)
        .execute(conn)?;

    Ok(())
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
