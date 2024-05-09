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

/// CLI interface for loading a step and exporting its entries to a Neo4j database.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Options {
    /// The log level
    #[arg(short, value_enum, long, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    /// The input step file
    #[arg(short, long)]
    pub input_file: PathBuf,

    /// The input step file
    #[arg(short, long)]
    pub user: String,

    /// The input step file
    #[arg(short, long)]
    pub password: String,

    /// The uri for the Neo4j database
    #[arg(short, long, default_value = "neo4j://127.0.0.1:7687")]
    pub neo4j_uri: String,
}

impl Options {
    /// Dumps the options to the log.
    pub fn dump_to_log(&self) {
        info!("log_level: {:?}", self.log_level);
        info!("input_file: {:?}", self.input_file);
        info!("Neo4J URI: {:?}", self.neo4j_uri);
    }
}
