use crate::config::InfluxDbConfig;
use crate::parser::GNetTrackRecord;
use anyhow::{Result, anyhow};
use influxdb::{Client, ReadQuery, Timestamp, WriteQuery};
use log::{debug, error, info};

pub struct InfluxClient {
    client: Client,
    database: String,
}

impl InfluxClient {
    pub fn new(config: &InfluxDbConfig) -> Result<Self> {
        let client = if let Some(token) = &config.token {
            if !token.is_empty() {
                // InfluxDB 2.x with token
                Client::new(&config.url, &config.database).with_token(token)
            } else {
                // InfluxDB 1.x with username/password
                if !config.username.is_empty() {
                    Client::new(&config.url, &config.database)
                        .with_auth(&config.username, &config.password)
                } else {
                    Client::new(&config.url, &config.database)
                }
            }
        } else {
            // InfluxDB 1.x with username/password
            if !config.username.is_empty() {
                Client::new(&config.url, &config.database)
                    .with_auth(&config.username, &config.password)
            } else {
                Client::new(&config.url, &config.database)
            }
        };

        Ok(Self {
            client,
            database: config.database.clone(),
        })
    }

    pub async fn test_connection(&self) -> Result<()> {
        let query = ReadQuery::new("SHOW DATABASES");

        match self.client.query(query).await {
            Ok(_) => {
                info!("Successfully connected to InfluxDB");
                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to InfluxDB: {e}");
                Err(anyhow!("Connection test failed: {e}"))
            }
        }
    }

    pub async fn create_database_if_not_exists(&self) -> Result<()> {
        let query = ReadQuery::new(format!("CREATE DATABASE \"{}\"", self.database));

        match self.client.query(query).await {
            Ok(_) => {
                info!("Database '{}' created or already exists", self.database);
                Ok(())
            }
            Err(e) => {
                // Database might already exist, which is okay
                debug!("Database creation result: {e}");
                Ok(())
            }
        }
    }

    pub async fn write_records(&self, records: &[GNetTrackRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let mut write_queries = Vec::new();

        for record in records {
            let timestamp =
                Timestamp::Nanoseconds(record.timestamp.timestamp_nanos_opt().unwrap_or(0) as u128);

            let mut write_query = WriteQuery::new(timestamp, "network_measurements")
                .add_tag("measurement_type", "gnettrack");

            // Add tags (indexed fields)
            if let Some(ref operator_name) = record.operator_name {
                write_query = write_query.add_tag("operator_name", operator_name.as_str());
            }
            if let Some(ref operator_code) = record.operator_code {
                write_query = write_query.add_tag("operator_code", operator_code.as_str());
            }
            if let Some(ref cell_id) = record.cell_id {
                write_query = write_query.add_tag("cell_id", cell_id.as_str());
            }
            if let Some(ref network_tech) = record.network_tech {
                write_query = write_query.add_tag("network_tech", network_tech.as_str());
            }
            if let Some(ref network_mode) = record.network_mode {
                write_query = write_query.add_tag("network_mode", network_mode.as_str());
            }
            if let Some(ref lac) = record.lac {
                write_query = write_query.add_tag("lac", lac.as_str());
            }

            // Add numeric fields
            if let Some(longitude) = record.longitude {
                write_query = write_query.add_field("longitude", longitude);
            }
            if let Some(latitude) = record.latitude {
                write_query = write_query.add_field("latitude", latitude);
            }
            if let Some(speed) = record.speed {
                write_query = write_query.add_field("speed", speed);
            }
            if let Some(level) = record.level {
                write_query = write_query.add_field("level", level);
            }
            if let Some(qual) = record.qual {
                write_query = write_query.add_field("qual", qual);
            }
            if let Some(snr) = record.snr {
                write_query = write_query.add_field("snr", snr);
            }
            if let Some(cqi) = record.cqi {
                write_query = write_query.add_field("cqi", cqi);
            }
            if let Some(dl_bitrate) = record.dl_bitrate {
                write_query = write_query.add_field("dl_bitrate", dl_bitrate);
            }
            if let Some(ul_bitrate) = record.ul_bitrate {
                write_query = write_query.add_field("ul_bitrate", ul_bitrate);
            }

            // Add string fields
            if let Some(ref cgi) = record.cgi {
                write_query = write_query.add_field("cgi", cgi.clone());
            }
            if let Some(ref cellname) = record.cellname {
                write_query = write_query.add_field("cellname", cellname.clone());
            }
            if let Some(ref node) = record.node {
                write_query = write_query.add_field("node", node.clone());
            }
            if let Some(ref arfcn) = record.arfcn {
                write_query = write_query.add_field("arfcn", arfcn.clone());
            }

            write_queries.push(write_query);
        }

        match self.client.query(write_queries).await {
            Ok(_) => {
                info!("Successfully wrote {} records to InfluxDB", records.len());
                Ok(())
            }
            Err(e) => {
                error!("Failed to write records to InfluxDB: {e}");
                Err(anyhow!("Write operation failed: {e}"))
            }
        }
    }

    pub async fn write_records_batch(
        &self,
        records: &[GNetTrackRecord],
        batch_size: usize,
    ) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        info!(
            "Writing {} records in batches of {}",
            records.len(),
            batch_size
        );

        for (i, chunk) in records.chunks(batch_size).enumerate() {
            debug!("Writing batch {} with {} records", i + 1, chunk.len());
            self.write_records(chunk).await?;
        }

        info!(
            "Successfully wrote all {} records to InfluxDB",
            records.len()
        );
        Ok(())
    }
}
