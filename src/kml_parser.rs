use crate::parser::GNetTrackRecord;
use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{debug, warn};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::fs::File;
use std::io::BufReader;

pub struct KmlParser {
    skip_invalid: bool,
}

impl KmlParser {
    pub fn new(skip_invalid: bool) -> Self {
        Self { skip_invalid }
    }

    pub fn parse_file(&self, file_path: &str) -> Result<Vec<GNetTrackRecord>> {
        let file = File::open(file_path)?;
        let buf_reader = BufReader::new(file);
        let mut reader = Reader::from_reader(buf_reader);
        reader.config_mut().trim_text(true);

        let mut records = Vec::new();
        let mut buf = Vec::new();
        let mut error_count = 0;

        let mut in_placemark = false;
        let mut current_placemark = PlacemarkData::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name().as_ref() {
                    b"Placemark" => {
                        in_placemark = true;
                        current_placemark = PlacemarkData::new();
                    }
                    b"Data" => {
                        if in_placemark && let Ok(Some(name_attr)) = e.try_get_attribute("name") {
                            let name_str = String::from_utf8_lossy(&name_attr.value);
                            let mut data_buf = Vec::new();
                            let value = self.read_data_value(&mut reader, &mut data_buf)?;
                            current_placemark.add_data(name_str.as_ref(), &value);
                        }
                    }
                    b"coordinates" => {
                        if in_placemark {
                            let mut coord_buf = Vec::new();
                            let coords = self.read_text_content(&mut reader, &mut coord_buf)?;
                            current_placemark.set_coordinates(&coords);
                        }
                    }
                    _ => {}
                },
                Ok(Event::End(ref e)) => {
                    if e.name().as_ref() == b"Placemark" && in_placemark {
                        match current_placemark.to_record() {
                            Ok(record) => {
                                records.push(record);
                            }
                            Err(e) => {
                                error_count += 1;
                                if self.skip_invalid {
                                    warn!("Skipping invalid placemark: {e}");
                                } else {
                                    return Err(anyhow!("Error parsing placemark: {e}"));
                                }
                            }
                        }
                        in_placemark = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    error_count += 1;
                    if self.skip_invalid {
                        warn!("XML parsing error: {e}");
                    } else {
                        return Err(anyhow!("XML parsing error: {e}"));
                    }
                }
                _ => {}
            }
            buf.clear();
        }

        if error_count > 0 {
            warn!("Encountered {error_count} errors while parsing KML file");
        }

        debug!("Parsed {} placemarks from KML file", records.len());
        Ok(records)
    }

    fn read_data_value(
        &self,
        reader: &mut Reader<BufReader<File>>,
        buf: &mut Vec<u8>,
    ) -> Result<String> {
        loop {
            buf.clear();
            match reader.read_event_into(buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"value" => {
                    buf.clear();
                    return self.read_text_content(reader, buf);
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"Data" => {
                    return Ok(String::new());
                }
                Ok(Event::Eof) => return Ok(String::new()),
                Err(e) => return Err(anyhow!("Error reading data value: {e}")),
                _ => {}
            }
        }
    }

    fn read_text_content(
        &self,
        reader: &mut Reader<BufReader<File>>,
        buf: &mut Vec<u8>,
    ) -> Result<String> {
        let mut content = String::new();
        loop {
            buf.clear();
            match reader.read_event_into(buf) {
                Ok(Event::Text(e)) => {
                    content.push_str(&e.unescape().unwrap_or_default());
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow!("Error reading text content: {e}")),
                _ => {}
            }
        }
        Ok(content)
    }
}

#[derive(Debug, Default)]
struct PlacemarkData {
    technology: Option<String>,
    rsrp: Option<String>,
    speed: Option<String>,
    altitude: Option<String>,
    time: Option<String>,
    coordinates: Option<String>,
}

impl PlacemarkData {
    fn new() -> Self {
        Self::default()
    }

    fn add_data(&mut self, name: &str, value: &str) {
        match name {
            "技術" => self.technology = Some(value.to_string()),
            "RSRP" => self.rsrp = Some(value.to_string()),
            "速度" => self.speed = Some(value.to_string()),
            "高度" => self.altitude = Some(value.to_string()),
            "時間" => self.time = Some(value.to_string()),
            _ => {
                debug!("Unknown KML data field: {name}");
            }
        }
    }

    fn set_coordinates(&mut self, coords: &str) {
        self.coordinates = Some(coords.to_string());
    }

    fn to_record(&self) -> Result<GNetTrackRecord> {
        // Parse coordinates (longitude,latitude,altitude)
        let (longitude, latitude) = if let Some(ref coords) = self.coordinates {
            let parts: Vec<&str> = coords.trim().split(',').collect();
            if parts.len() >= 2 {
                let lon = parts[0].parse::<f64>().ok();
                let lat = parts[1].parse::<f64>().ok();
                (lon, lat)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Parse timestamp
        let timestamp = if let Some(ref time_str) = self.time {
            parse_kml_timestamp(time_str)?
        } else {
            Utc::now()
        };

        // Parse speed (remove "km/h" suffix)
        let speed = if let Some(ref speed_str) = self.speed {
            speed_str
                .replace(" km/h", "")
                .replace("km/h", "")
                .trim()
                .parse::<f64>()
                .ok()
        } else {
            None
        };

        // Parse RSRP (remove "dBm" suffix)
        let level = if let Some(ref rsrp_str) = self.rsrp {
            rsrp_str
                .replace(" dBm", "")
                .replace("dBm", "")
                .trim()
                .parse::<f64>()
                .ok()
        } else {
            None
        };

        // Parse altitude from ExtendedData (remove "m" suffix)
        let _altitude_parsed = if let Some(ref alt_str) = self.altitude {
            alt_str.replace("m", "").trim().parse::<f64>().ok()
        } else {
            None
        };

        Ok(GNetTrackRecord {
            timestamp,
            longitude,
            latitude,
            speed,
            operator_name: Some("KDDI".to_string()), // Inferred from filename
            operator_code: None,
            cgi: None,
            cellname: None,
            node: None,
            cell_id: None,
            lac: None,
            network_tech: self.technology.clone(),
            network_mode: None,
            level,
            qual: None,
            snr: None,
            cqi: None,
            arfcn: None,
            dl_bitrate: None,
            ul_bitrate: None,
        })
    }
}

fn parse_kml_timestamp(time_str: &str) -> Result<DateTime<Utc>> {
    // Expected format: "2025.10.03_10.20.09"
    let cleaned = time_str.replace('_', " ");

    let formats = [
        "%Y.%m.%d %H.%M.%S",
        "%Y.%m.%d_%H.%M.%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
    ];

    for format in &formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&cleaned, format) {
            return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
    }

    Err(anyhow!("Unable to parse KML timestamp: {time_str}"))
}
