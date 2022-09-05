use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, TimeZone};
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::warn;

static DATE_TIME_FORMAT: &str = "%A, %B %e, %Y %r";

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let file_name = env::var("FILE_NAME").context("Failed to read file name")?;
    let r = read_file(file_name);
    println!("{:#?}", r);
    Ok(())
}

#[derive(Default, Debug)]
struct RobocopyStats {
    started: DateTime<Local>,
    ended: DateTime<Local>,
    source: String,
    destination: String,
    /// Files selection pattern
    files: String,
    options: String,
    /// In bytes per second
    speed: u128,
}

impl RobocopyStats {
    pub fn new() -> Self {
        RobocopyStats::default()
    }

    /// Parse a header or footer key and value
    ///
    /// Expects the keys and values to be trimmed
    ///
    /// Possible fields:
    /// * Started
    /// * Source
    /// * Dest
    /// * Files
    /// * Options
    pub fn parse_header(&mut self, key: &str, value: &str) {
        let mut r = move || {
            match key {
                "Started" => self.started = Local.datetime_from_str(value, DATE_TIME_FORMAT)?,
                "Source" => self.source = value.to_string(),
                "Dest" => self.destination = value.to_string(),
                "Files" => self.files = value.to_string(),
                "Options" => self.options = value.to_string(),
                _ => return Err(anyhow!("Unknown header key: {}", key)),
            };
            Ok(())
        };

        if let Err(err) = r() {
            warn!("Failed to parse header key `{}`: {}", key, err);
        }
    }

    /// Parse a footer key and value
    ///
    /// Expects the keys and values to be trimmed
    ///
    /// Possible fields:
    /// * Ended
    pub fn parse_footer(&mut self, key: &str, value: &str) {
        let mut r = move || {
            match key {
                "Ended" => self.ended = Local.datetime_from_str(value, DATE_TIME_FORMAT)?,
                "Speed" => self.speed = parse_speed(value)?,
                _ => return Err(anyhow!("Unknown footer key: {}", key)),
            };
            Ok(())
        };

        if let Err(err) = r() {
            warn!("Failed to parse footer key `{}`: {}", key, err);
        }
    }
}

/// Convert a header or footer line in Key: Value
/// Keys are the characters until the first `:`, Values are the rest of the line
/// Both keys and values are returned trimmed
fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    line.split_once(':').map(|(k, v)| (k.trim(), v.trim()))
}

fn read_file<P: AsRef<Path>>(path: P) -> Result<RobocopyStats> {
    let file = File::open(path)?;

    let decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding_rs::WINDOWS_1252))
        .build(file);
    let buffered_file = BufReader::new(decoder);
    let mut stats = RobocopyStats::new();

    // There are four sections, each coming after a first line of only dashes:
    // 1. ROBOCOPY title
    // 2. Initial info and config
    // 3. Files list
    // 4. End statistics
    //
    // We only care about numbers 2 and 4
    let mut section = 0;

    for (line_no, line) in buffered_file.lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                warn!("Failed to read line {}: {}", line_no, err);
                continue;
            }
        };

        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line.trim_start_matches('-').is_empty() {
            section += 1;
            continue;
        }

        if section == 2 {
            if let Some((k, v)) = split_key_value(line) {
                stats.parse_header(k, v);
            }
        } else if section == 4 {
            if let Some((k, v)) = split_key_value(line) {
                stats.parse_footer(k, v);
            }
        }
    }

    Ok(stats)
}

fn parse_speed(value: &str) -> Result<u128> {
    let (value, unit) = value
        .split_once(' ')
        .ok_or_else(|| anyhow!("Unrecognized speed value: {}", value))?;
    if unit.eq_ignore_ascii_case("Bytes/sec.") {
        value.parse().context("Failed to parse speed value")
    } else {
        Err(anyhow!("Unexpected speed unit: {}", unit))
    }
}
