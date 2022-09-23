use anyhow::Result;
use std::fs::OpenOptions;
use tracing::{error, info, instrument};

use robocopy::RobocopyResult;
use uuid::Uuid;

use crate::config::Config;

mod config;
mod robocopy;

fn main() -> Result<()> {
    let config = Config::from_args();

    let subscriber = tracing_subscriber::fmt();

    if let Some(log_file_path) = &config.log_file {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(log_file_path)?;

        subscriber.json().with_writer(file).init();
    } else {
        subscriber.init();
    }
    work(&config);
    Ok(())
}

#[instrument(skip_all, name = "main", fields(execution_id = Uuid::new_v4().to_string()))]
fn work(config: &Config) {
    match (|| {
        let r = RobocopyResult::read_file(&config.source_file)?;
        r.write_to_file(&config.output_file, config.overwrite)?;
        Ok::<(), anyhow::Error>(())
    })() {
        Ok(()) => info!("Done"),
        Err(err) => error!(
            error.cause = err.root_cause(),
            error.source = err.root_cause().source(),
            "{}",
            err
        ),
    }
}
