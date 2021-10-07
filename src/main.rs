#[macro_use]
extern crate diesel;

use diesel::prelude::*;

pub mod models;
pub mod schema;

fn main() {
    println!("Hello, mocword!");
}
