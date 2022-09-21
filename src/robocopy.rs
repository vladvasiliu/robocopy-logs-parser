use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, TimeZone};
use encoding_rs_io::DecodeReaderBytesBuilder;
use serde::Serialize;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::{instrument, warn};

static DATE_TIME_FORMAT: &str = "%A, %B %e, %Y %r";

#[derive(Debug, Serialize)]
pub struct CopyStat {
    total: u128,
    copied: u128,
    skipped: u128,
    mismatch: u128,
    failed: u128,
    extras: u128,
}

#[derive(Debug, Default, Serialize)]
pub struct Stats {
    dirs: Option<CopyStat>,
    files: Option<CopyStat>,
    bytes: Option<CopyStat>,
}

#[derive(Default, Debug, Serialize)]
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
    stats: Stats,
}

impl RobocopyResult {
    /// Read and parse the file into a usable struct
    #[instrument]
    pub fn read_file<P: AsRef<Path> + Debug>(path: P) -> Result<Self> {
        let file = File::open(path)?;

        let decoder = DecodeReaderBytesBuilder::new()
            .encoding(Some(encoding_rs::UTF_16LE))
            .build(file);
        let buffered_file = BufReader::new(decoder);
        let mut robocopy_result = Self::default();

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
                    robocopy_result.parse_header(k, v);
                }
            } else if section == 4 {
                if let Some((k, v)) = split_key_value(line) {
                    robocopy_result.parse_footer(k, v);
                }
            }
        }

        Ok(robocopy_result)
    }

    /// Parse a header key and value
    ///
    /// Expects the keys and values to be trimmed
    ///
    /// Possible fields:
    /// * Started
    /// * Source
    /// * Dest
    /// * Files
    /// * Options
    #[instrument(skip(self))]
    pub fn parse_header(&mut self, key: &str, value: &str) {
        if let Err(err) = (|| {
            match key {
                "Started" => self.started = Some(Local.datetime_from_str(value, DATE_TIME_FORMAT)?),
                "Source" => self.source = Some(value.to_string()),
                "Dest" => self.destination = Some(value.to_string()),
                "Files" => self.files = Some(value.to_string()),
                "Options" => self.options = Some(value.to_string()),
                _ => return Err(anyhow!("Unknown header key: {}", key)),
            };
            Ok(())
        })() {
            warn!("Failed to parse header key `{}`: {}", key, err);
        };
    }

    /// Parse a footer key and value
    ///
    /// Expects the keys and values to be trimmed
    ///
    /// Possible fields:
    /// * Ended
    /// * Speed (bytes only)
    /// * Dirs
    /// * Files
    #[instrument(skip(self))]
    pub fn parse_footer(&mut self, key: &str, value: &str) {
        if let Err(err) = (|| {
            match key {
                "Ended" => self.ended = Some(Local.datetime_from_str(value, DATE_TIME_FORMAT)?),
                "Speed" => self.speed = Some(parse_speed(value)?),
                "Dirs" => {
                    self.stats.dirs =
                        Some(parse_stats(value).context("Failed to parse dirs stats")?)
                }
                "Files" => {
                    self.stats.files =
                        Some(parse_stats(value).context("Failed to parse files stats")?)
                }
                "Bytes" => {
                    self.stats.bytes =
                        Some(parse_stats(value).context("Failed to parse bytes stats")?)
                }
                _ => return Err(anyhow!("Unknown footer key: {}", key)),
            };
            Ok(())
        })() {
            warn!("Failed to parse footer key `{}`: {}", key, err);
        };
    }

    #[instrument(skip(self))]
    pub fn write_to_file<P: AsRef<Path> + Debug>(&self, output: P, overwrite: bool) -> Result<()> {
        let mut options = File::options();
        options.write(true);
        if overwrite {
            options.create(true).truncate(true);
        } else {
            options.create_new(true);
        }
        let file = options.open(output).context("Failed to open output file")?;
        serde_json::to_writer(&file, &self).context("Failed to write output file")
    }
}

/// Convert a header or footer line in Key: Value
/// Keys are the characters until the first `:`, Values are the rest of the line
/// Both keys and values are returned trimmed
fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    line.split_once(':').map(|(k, v)| (k.trim(), v.trim()))
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
            "Unexpected number of fields: {} instead of 6",
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
