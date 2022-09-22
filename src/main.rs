use tracing::{error, info, instrument};

use robocopy::RobocopyResult;
use uuid::Uuid;

use crate::config::Config;

mod config;
mod robocopy;

fn main() {
    let subscriber = tracing_subscriber::fmt();
    if matches!(std::env::var("ROBOCOPY_LOG"), Ok(f) if f.eq_ignore_ascii_case("json")) {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
    let config = Config::from_args();
    work(&config);
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
