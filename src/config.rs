use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub influxdb: InfluxDbConfig,
    pub logging: LoggingConfig,
    pub processing: ProcessingConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InfluxDbConfig {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub org: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProcessingConfig {
    pub batch_size: usize,
    pub skip_invalid: bool,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default() -> Self {
        Config {
            influxdb: InfluxDbConfig {
                url: "http://localhost:8086".to_string(),
                database: "gnettrack".to_string(),
                username: String::new(),
                password: String::new(),
                org: None,
                token: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
            processing: ProcessingConfig {
                batch_size: 1000,
                skip_invalid: true,
            },
        }
    }
}