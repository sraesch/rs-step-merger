use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use log::{info, LevelFilter};

/// Workaround for parsing the different log level
#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
        }
    }
}

/// CLI interface for merging step files into a single monolithic step file.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Options {
    /// The log level
    #[arg(short, value_enum, long, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    /// The input file to with the assembly structure
    #[arg(short, long)]
    pub input_file: PathBuf,

    /// The output file to write the merged step file to
    #[arg(short, long)]
    pub output_file: PathBuf,

    /// Avoid loading references
    #[arg(short, long)]
    pub avoid_references: bool,
}

impl Options {
    /// Dumps the options to the log.
    pub fn dump_to_log(&self) {
        info!("log_level: {:?}", self.log_level);
        info!("input_file: {:?}", self.input_file);
        info!("output_file: {:?}", self.output_file);
        info!("loading references: {:?}", !self.avoid_references);
    }
}
