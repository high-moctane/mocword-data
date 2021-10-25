use anyhow::Result;
use env_logger;
use log::info;

pub fn run() -> Result<()> {
    env_logger::init();

    info!("hello download");

    Ok(())
}
