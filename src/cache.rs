use anyhow::Result;
use diesel::mysql::MysqlConnection;
use diesel::{self, prelude::*};
use radix_trie::Trie;

use crate::models;
use crate::schema;

pub struct OneGramCache {
    cache: Trie<String, Option<i32>>,
}

impl OneGramCache {
    fn get(&self, conn: &MysqlConnection, key: &str) -> Result<Option<i32>> {
        use schema::one_grams::dsl;

        if let Some(opt_id) = self.cache.get(key) {
            return Ok(*opt_id);
        }

        let res = dsl::one_grams
            .filter(dsl::word.eq_all(key))
            .load::<models::OneGram>(conn)?;

        match res.len() {
            0 => Ok(None),
            1 => Ok(Some(res[0].id)),
            _ => panic!("duplicated key: {}", key),
        }
    }
}
