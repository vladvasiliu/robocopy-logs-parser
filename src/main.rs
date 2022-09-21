use anyhow::Result;
use tracing::{error, info};

mod config;
mod robocopy;

use crate::config::Config;
use robocopy::RobocopyResult;

fn main() {
    tracing_subscriber::fmt().init();
    let config = Config::from_args();

    match work(&config) {
        Ok(()) => info!("Done"),
        Err(err) => error!(
            error.cause = err.root_cause(),
            error.source = err.root_cause().source(),
            error.backtrace = err.backtrace().to_string(),
            "{}",
            err
        ),
    }
}

fn work(config: &Config) -> Result<()> {
    let r = RobocopyResult::read_file(&config.source_file)?;
    r.write_to_file(&config.output_file, config.overwrite)?;
    Ok(())
}
