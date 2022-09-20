use anyhow::Result;

mod config;
mod robocopy;

use crate::config::Config;
use robocopy::RobocopyResult;

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let config = Config::from_args();
    let r = RobocopyResult::read_file(config.source_file)?;
    r.write_to_file(config.output_file)?;
    Ok(())
}
