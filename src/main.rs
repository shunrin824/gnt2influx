mod config;
mod influx_client;
mod kml_parser;
mod parser;

use anyhow::Result;
use clap::{Arg, Command};
use log::{LevelFilter, debug, error, info};
use std::path::Path;

use crate::config::Config;
use crate::influx_client::InfluxClient;
use crate::kml_parser::KmlParser;
use crate::parser::LogParser;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("gnt2influx")
        .version("0.1.0")
        .author("Your Name")
        .about("Converts G-NetTrack Lite log files to InfluxDB format and uploads them")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Path to G-NetTrack log file")
                .required_unless_present("test-connection"),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Path to configuration file")
                .default_value("config.toml"),
        )
        .arg(
            Arg::new("test-connection")
                .long("test-connection")
                .help("Test InfluxDB connection without uploading data")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Parse the log file but don't upload to InfluxDB")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize logging
    let log_level = if matches.get_flag("verbose") {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    env_logger::Builder::new().filter_level(log_level).init();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = if Path::new(config_path).exists() {
        Config::from_file(config_path)?
    } else {
        info!("Configuration file not found, using default settings");
        Config::default()
    };

    // Override log level from config if not set via CLI
    if !matches.get_flag("verbose") {
        let level = match config.logging.level.to_lowercase().as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        };
        env_logger::Builder::new()
            .filter_level(level)
            .try_init()
            .ok();
    }

    // Create InfluxDB client
    let influx_client = InfluxClient::new(&config.influxdb)?;

    // Test connection if requested
    if matches.get_flag("test-connection") {
        info!("Testing InfluxDB connection...");
        influx_client.test_connection().await?;
        info!("Connection test successful!");
        return Ok(());
    }

    // Get input file
    let input_file = match matches.get_one::<String>("input") {
        Some(file) => file,
        None => {
            error!("Input file is required when not testing connection");
            std::process::exit(1);
        }
    };

    if !Path::new(input_file).exists() {
        error!("Input file does not exist: {input_file}");
        std::process::exit(1);
    }

    info!("Processing log file: {input_file}");

    // Parse the log file - detect format by extension
    let records = if input_file.to_lowercase().ends_with(".kml") {
        let kml_parser = KmlParser::new(config.processing.skip_invalid);
        kml_parser.parse_file(input_file)?
    } else {
        let parser = LogParser::new(config.processing.batch_size, config.processing.skip_invalid);
        parser.parse_file(input_file)?
    };

    info!("Successfully parsed {} records", records.len());

    if records.is_empty() {
        info!("No records to process");
        return Ok(());
    }

    // Debug: print first few records to understand the data structure
    if matches.get_flag("verbose") {
        for (i, record) in records.iter().take(3).enumerate() {
            debug!("Record {}: {:?}", i + 1, record);
        }
    }

    // Dry run - just parse and exit
    if matches.get_flag("dry-run") {
        info!(
            "Dry run completed. {} records would be uploaded.",
            records.len()
        );

        // Show what InfluxDB queries would look like for first few records
        if matches.get_flag("verbose") {
            info!("Sample InfluxDB line protocol format (dry run):");
            let influx_client = InfluxClient::new(&config.influxdb)?;
            // Take first 3 records for debugging
            let sample_records: Vec<_> = records.iter().take(3).cloned().collect();
            match influx_client.format_records_for_influx(&sample_records) {
                Ok(formatted_lines) => {
                    for (i, line) in formatted_lines.iter().enumerate() {
                        info!("InfluxDB line {}: {}", i + 1, line);
                    }
                }
                Err(e) => debug!("Failed to format records for InfluxDB: {e}"),
            }
        }

        return Ok(());
    }

    // Test connection and create database
    info!("Testing InfluxDB connection...");
    match influx_client.test_connection().await {
        Ok(_) => info!("InfluxDB connection successful"),
        Err(e) => {
            error!("InfluxDB connection failed: {e}");
            info!(
                "Please ensure InfluxDB is running on {}",
                config.influxdb.url
            );
            info!(
                "You can start InfluxDB with Docker: docker run -d --name influxdb -p 8086:8086 -e INFLUXDB_DB=gnettrack influxdb:1.8"
            );
            return Err(e);
        }
    }

    info!("Creating database if it doesn't exist...");
    influx_client.create_database_if_not_exists().await?;

    // Upload records to InfluxDB
    info!("Uploading {} records to InfluxDB...", records.len());
    match influx_client
        .write_records_batch(&records, config.processing.batch_size)
        .await
    {
        Ok(_) => {
            info!(
                "Successfully uploaded {} records to InfluxDB!",
                records.len()
            );
            info!(
                "Data is now available in database '{}' on {}",
                config.influxdb.database, config.influxdb.url
            );
            info!("You can query the data with: SELECT * FROM network_measurements LIMIT 10");
        }
        Err(e) => {
            error!("Failed to upload records to InfluxDB: {e}");
            return Err(e);
        }
    }

    info!("Successfully completed processing!");
    Ok(())
}
