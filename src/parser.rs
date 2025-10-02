use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDateTime, Utc};
use csv::ReaderBuilder;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GNetTrackRecord {
    pub timestamp: DateTime<Utc>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub speed: Option<f64>,
    pub operator_name: Option<String>,
    pub operator_code: Option<String>,
    pub cgi: Option<String>,
    pub cellname: Option<String>,
    pub node: Option<String>,
    pub cell_id: Option<String>,
    pub lac: Option<String>,
    pub network_tech: Option<String>,
    pub network_mode: Option<String>,
    pub level: Option<f64>,
    pub qual: Option<f64>,
    pub snr: Option<f64>,
    pub cqi: Option<f64>,
    pub arfcn: Option<String>,
    pub dl_bitrate: Option<f64>,
    pub ul_bitrate: Option<f64>,
}

impl GNetTrackRecord {
    pub fn from_csv_record(
        record: &csv::StringRecord,
        headers: &csv::StringRecord,
    ) -> Result<Self> {
        let mut timestamp = Utc::now();
        let mut longitude = None;
        let mut latitude = None;
        let mut speed = None;
        let mut operator_name = None;
        let mut operator_code = None;
        let mut cgi = None;
        let mut cellname = None;
        let mut node = None;
        let mut cell_id = None;
        let mut lac = None;
        let mut network_tech = None;
        let mut network_mode = None;
        let mut level = None;
        let mut qual = None;
        let mut snr = None;
        let mut cqi = None;
        let mut arfcn = None;
        let mut dl_bitrate = None;
        let mut ul_bitrate = None;

        for (i, value) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                let header_lower = header.to_lowercase();

                match header_lower.as_str() {
                    "timestamp" | "time" => {
                        timestamp = parse_timestamp(value)?;
                    }
                    "longitude" | "lon" => {
                        longitude = parse_float_optional(value);
                    }
                    "latitude" | "lat" => {
                        latitude = parse_float_optional(value);
                    }
                    "speed" => {
                        speed = parse_float_optional(value);
                    }
                    "operator" | "operator_name" => {
                        operator_name = Some(value.to_string());
                    }
                    "mcc-mnc" | "operator_code" => {
                        operator_code = Some(value.to_string());
                    }
                    "cgi" => {
                        cgi = Some(value.to_string());
                    }
                    "cellname" => {
                        cellname = Some(value.to_string());
                    }
                    "node" | "rnc" | "enodeb" => {
                        node = Some(value.to_string());
                    }
                    "cellid" | "cell_id" => {
                        cell_id = Some(value.to_string());
                    }
                    "lac" => {
                        lac = Some(value.to_string());
                    }
                    "networktech" | "network_tech" | "tech" => {
                        network_tech = Some(value.to_string());
                    }
                    "networkmode" | "network_mode" | "mode" => {
                        network_mode = Some(value.to_string());
                    }
                    "level" | "rsrp" | "rscp" | "rxlevel" => {
                        level = parse_float_optional(value);
                    }
                    "qual" | "rsrq" | "ecno" | "rxqual" => {
                        qual = parse_float_optional(value);
                    }
                    "snr" => {
                        snr = parse_float_optional(value);
                    }
                    "cqi" => {
                        cqi = parse_float_optional(value);
                    }
                    "arfcn" => {
                        arfcn = Some(value.to_string());
                    }
                    "dl_bitrate" | "downlink_bitrate" => {
                        dl_bitrate = parse_float_optional(value);
                    }
                    "ul_bitrate" | "uplink_bitrate" => {
                        ul_bitrate = parse_float_optional(value);
                    }
                    _ => {
                        // Ignore unknown columns
                        debug!("Unknown column: {header}");
                    }
                }
            }
        }

        Ok(GNetTrackRecord {
            timestamp,
            longitude,
            latitude,
            speed,
            operator_name,
            operator_code,
            cgi,
            cellname,
            node,
            cell_id,
            lac,
            network_tech,
            network_mode,
            level,
            qual,
            snr,
            cqi,
            arfcn,
            dl_bitrate,
            ul_bitrate,
        })
    }
}

pub struct LogParser {
    skip_invalid: bool,
}

impl LogParser {
    pub fn new(_batch_size: usize, skip_invalid: bool) -> Self {
        Self { skip_invalid }
    }

    pub fn parse_file(&self, file_path: &str) -> Result<Vec<GNetTrackRecord>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        // Try to determine the delimiter (tab or comma)
        let delimiter = self.detect_delimiter(file_path)?;

        let mut csv_reader = ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(true)
            .from_reader(reader);

        let headers = csv_reader.headers()?.clone();
        let mut records = Vec::new();
        let mut error_count = 0;

        for (line_num, result) in csv_reader.records().enumerate() {
            match result {
                Ok(record) => match GNetTrackRecord::from_csv_record(&record, &headers) {
                    Ok(parsed_record) => {
                        records.push(parsed_record);
                    }
                    Err(e) => {
                        error_count += 1;
                        if self.skip_invalid {
                            warn!("Skipping invalid record at line {}: {}", line_num + 2, e);
                        } else {
                            return Err(anyhow!(
                                "Error parsing record at line {}: {}",
                                line_num + 2,
                                e
                            ));
                        }
                    }
                },
                Err(e) => {
                    error_count += 1;
                    if self.skip_invalid {
                        warn!("Skipping malformed line {}: {}", line_num + 2, e);
                    } else {
                        return Err(anyhow!("Error reading line {}: {}", line_num + 2, e));
                    }
                }
            }
        }

        if error_count > 0 {
            warn!("Encountered {error_count} errors while parsing file");
        }

        Ok(records)
    }

    fn detect_delimiter(&self, file_path: &str) -> Result<u8> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        use std::io::BufRead;
        reader.read_line(&mut line)?;

        if line.contains('\t') {
            Ok(b'\t')
        } else {
            Ok(b',')
        }
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    if value.is_empty() {
        return Ok(Utc::now());
    }

    // Try multiple timestamp formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%d.%m.%Y %H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.3f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S%.3fZ",
    ];

    for format in &formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
    }

    // Try parsing as Unix timestamp
    if let Ok(timestamp) = value.parse::<i64>()
        && let Some(dt) = DateTime::from_timestamp(timestamp, 0)
    {
        return Ok(dt);
    }

    Err(anyhow!("Unable to parse timestamp: {value}"))
}

fn parse_float_optional(value: &str) -> Option<f64> {
    if value.is_empty() || value == "N/A" || value == "null" {
        None
    } else {
        value.parse().ok()
    }
}
