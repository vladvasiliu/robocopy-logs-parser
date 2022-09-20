use clap::builder::PathBufValueParser;
use clap::{arg, command};
use std::path::PathBuf;

pub struct Config {
    pub source_file: PathBuf,
    pub output_file: PathBuf,
    pub log_file: PathBuf,
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
                    .required(true),
            )
            .get_matches();

        Config {
            source_file: matches.get_one::<PathBuf>("source").unwrap().clone(),
            output_file: matches.get_one::<PathBuf>("output").unwrap().clone(),
            log_file: matches.get_one::<PathBuf>("log").unwrap().clone(),
        }
    }
}
