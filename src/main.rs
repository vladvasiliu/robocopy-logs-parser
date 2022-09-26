use anyhow::Result;
use chrono::Utc;
use std::fs::OpenOptions;
use tracing::{error, info, instrument};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use robocopy::RobocopyResult;
use uuid::Uuid;

use crate::config::Config;

mod config;
mod robocopy;

fn main() -> Result<()> {
    let config = Config::from_args();

    let log_stdout = tracing_subscriber::fmt::layer();

    let subscriber = tracing_subscriber::Registry::default().with(log_stdout);

    let log_json = if let Some(log_file_path) = &config.log_file {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(log_file_path)?;

        Some(tracing_subscriber::fmt::layer().json().with_writer(file))
    } else {
        None
    };
    subscriber.with(log_json).init();
    work(&config);
    Ok(())
}

#[instrument(skip_all, name = "main", fields(execution_id = Uuid::new_v4().to_string()))]
fn work(config: &Config) {
    let start_time = Utc::now();
    let result = (|| {
        let r = RobocopyResult::read_file(&config.source_file)?;
        r.write_to_file(&config.output_file, config.overwrite)?;
        Ok::<(), anyhow::Error>(())
    })();

    let end_time = Utc::now();
    let duration = (end_time - start_time).num_seconds();
    let success = match result {
        Ok(()) => true,
        Err(err) => {
            error!(
                error.cause = err.root_cause(),
                error.source = err.root_cause().source(),
                "{}",
                err
            );
            false
        }
    };
    info!(duration, success, "Done");
}
