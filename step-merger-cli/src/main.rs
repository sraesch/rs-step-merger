mod options;

use std::{io::Write, time::Instant};

use anyhow::Result;
use clap::Parser;
use log::{error, info, LevelFilter};
use options::Options;
use step_merger::{step::StepData, Assembly};

/// Parses the program arguments and returns None, if no arguments were provided and Some otherwise.
fn parse_args() -> Result<Options> {
    let options = Options::parse();
    Ok(options)
}

/// Initializes the program logging
///
/// # Arguments
/// * `filter` - The log level filter, i.e., the minimum log level to be logged.
fn initialize_logging(filter: LevelFilter) {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter_level(filter)
        .init();
}

/// Runs the program.
fn run_program() -> Result<()> {
    let options = parse_args()?;
    initialize_logging(LevelFilter::from(options.log_level));

    info!("Options:");
    options.dump_to_log();
    info!("-------");

    info!("Read STEP data...");
    let step_data = StepData::from_file(options.input_file)?;
    info!("Read STEP data...DONE");

    info!("Write STEP data...");
    step_data.to_file(options.output_file)?;
    info!("Write STEP data...DONE");

    // info!("Read assembly structure...");
    // let t = Instant::now();
    // let assembly: Assembly = step_merger::Assembly::from_file(options.input_file)?;
    // info!("Read assembly structure...DONE in {} ms", t.elapsed().as_millis());

    Ok(())
}

fn main() {
    match run_program() {
        Ok(()) => {
            info!("SUCCESS");
        }
        Err(err) => {
            error!("Error: {}", err);
            error!("FAILED");

            std::process::exit(-1);
        }
    }
}
