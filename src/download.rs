use anyhow::Result;

use diesel::prelude::*;

use diesel::sqlite::SqliteConnection;

use crate::models::*;
use crate::schema;

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
