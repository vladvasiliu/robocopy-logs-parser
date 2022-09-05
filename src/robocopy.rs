use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, TimeZone};
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::warn;

static DATE_TIME_FORMAT: &str = "%A, %B %e, %Y %r";

#[derive(Debug)]
pub struct CopyStat {
    total: u128,
    copied: u128,
    skipped: u128,
    mismatch: u128,
    failed: u128,
    extras: u128,
}

#[derive(Default, Debug)]
pub struct RobocopyResult {
    started: Option<DateTime<Local>>,
    ended: Option<DateTime<Local>>,
    source: Option<String>,
    destination: Option<String>,
    /// Files selection pattern
    files: Option<String>,
    options: Option<String>,
    /// In bytes per second
    speed: Option<u128>,
    dirs_stats: Option<CopyStat>,
    files_stats: Option<CopyStat>,
    bytes_stats: Option<CopyStat>,
}

impl RobocopyResult {
    pub fn new() -> Self {
        RobocopyResult::default()
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
                "Started" => self.started = Some(Local.datetime_from_str(value, DATE_TIME_FORMAT)?),
                "Source" => self.source = Some(value.to_string()),
                "Dest" => self.destination = Some(value.to_string()),
                "Files" => self.files = Some(value.to_string()),
                "Options" => self.options = Some(value.to_string()),
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
                "Ended" => self.ended = Some(Local.datetime_from_str(value, DATE_TIME_FORMAT)?),
                "Speed" => self.speed = Some(parse_speed(value)?),
                "Dirs" => {
                    self.dirs_stats =
                        Some(parse_stats(value).context("Failed to parse dirs stats")?)
                }
                "Files" => {
                    self.files_stats =
                        Some(parse_stats(value).context("Failed to parse files stats")?)
                }
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

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<RobocopyResult> {
    let file = File::open(path)?;

    let decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding_rs::WINDOWS_1252))
        .build(file);
    let buffered_file = BufReader::new(decoder);
    let mut stats = RobocopyResult::new();

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

/// Parses the copy statistics from the Robocopy log
fn parse_stats(value: &str) -> Result<CopyStat> {
    let fields_iter = value.split_ascii_whitespace();
    let field_vec = fields_iter.collect::<Vec<&str>>();
    let field_count = field_vec.len();
    if field_count != 6 {
        return Err(anyhow!(
            "Wrong number of fields: {} instead of 6",
            field_count,
        ));
    };
    Ok(CopyStat {
        total: field_vec[0].parse()?,
        copied: field_vec[1].parse()?,
        skipped: field_vec[2].parse()?,
        mismatch: field_vec[3].parse()?,
        failed: field_vec[4].parse()?,
        extras: field_vec[5].parse()?,
    })
}
