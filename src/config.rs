use clap::builder::PathBufValueParser;
use clap::{arg, command, ArgAction};
use std::path::PathBuf;

pub struct Config {
    pub source_file: PathBuf,
    pub output_file: PathBuf,
    pub log_file: Option<PathBuf>,
    pub overwrite: bool,
}

impl Config {
    pub fn from_args() -> Self {
        let matches = command!()
            .arg(
                arg!(--source <SOURCE> "Robocopy log file to process")
                    .value_parser(PathBufValueParser::new())
                    .required(true),
            )
            .arg(
                arg!(--output <OUTPUT> "Processed output file")
                    .value_parser(PathBufValueParser::new())
                    .required(true),
            )
            .arg(
                arg!(--log <LOG> "Where to output this program's logs")
                    .value_parser(PathBufValueParser::new())
                    .required(false),
            )
            .arg(
                arg!(--overwrite "Overwrite of output file if present")
                    .takes_value(false)
                    .action(ArgAction::SetTrue),
            )
            .get_matches();

        Config {
            source_file: matches.get_one::<PathBuf>("source").unwrap().clone(),
            output_file: matches.get_one::<PathBuf>("output").unwrap().clone(),
            log_file: matches.get_one::<PathBuf>("log").cloned(),
            overwrite: matches.get_flag("overwrite"),
        }
    }
}
