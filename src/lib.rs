#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

embed_migrations!();

pub mod download;
pub mod models;
pub mod schema;
