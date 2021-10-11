extern crate mocword;

use crate::mocword::download;

fn main() {
    download::run().expect("mocword_download failed");
}
