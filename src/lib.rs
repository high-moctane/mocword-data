#[macro_use]
extern crate diesel_migrations;

embed_migrations!();

pub mod download;
