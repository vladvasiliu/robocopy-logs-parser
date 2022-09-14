use anyhow::{Context, Result};
use std::env;

mod robocopy;

use robocopy::RobocopyResult;

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let file_name = env::var("FILE_NAME").context("Failed to read file name")?;
    let r = RobocopyResult::read_file(file_name)?;
    r.write_to_file()?;
    Ok(())
}
