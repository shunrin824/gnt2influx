use crate::config::InfluxDbConfig;
use crate::parser::GNetTrackRecord;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use futures::stream;
use influxdb::{Client as InfluxDB1Client, ReadQuery, Timestamp, WriteQuery};
use influxdb2::{Client as InfluxDB2Client, models::DataPoint};
use log::{debug, error, info};

pub enum InfluxClient {
    V1 {
        client: InfluxDB1Client,
        database: String,
    },
    V2 {
        client: InfluxDB2Client,
        #[allow(dead_code)]
        org: String,
        bucket: String,
    },
}

impl InfluxClient {
    pub fn new(config: &InfluxDbConfig) -> Result<Self> {
        // Check if we should use InfluxDB 2.x (token and org are provided)
        if let Some(token) = &config.token {
            if !token.is_empty() {
                if let Some(org) = &config.org {
                    if !org.is_empty() {
                        // InfluxDB 2.x
                        let client = InfluxDB2Client::new(&config.url, org, token);
                        return Ok(Self::V2 {
                            client,
                            org: org.clone(),
                            bucket: config.database.clone(), // Use database as bucket name
                        });
                    }
                }
            }
        }

        // InfluxDB 1.x fallback
        let client = if !config.username.is_empty() {
            InfluxDB1Client::new(&config.url, &config.database)
                .with_auth(&config.username, &config.password)
        } else {
            InfluxDB1Client::new(&config.url, &config.database)
        };

        Ok(Self::V1 {
            client,
            database: config.database.clone(),
        })
    }

    pub async fn test_connection(&self) -> Result<()> {
        match self {
            Self::V1 { client, .. } => {
                let query = ReadQuery::new("SHOW DATABASES");
                match client.query(query).await {
                    Ok(_) => {
                        info!("Successfully connected to InfluxDB 1.x");
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to connect to InfluxDB 1.x: {e}");
                        Err(anyhow!("Connection test failed: {e}"))
                    }
                }
            }
            Self::V2 { client, .. } => match client.health().await {
                Ok(_) => {
                    info!("Successfully connected to InfluxDB 2.x");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to connect to InfluxDB 2.x: {e}");
                    Err(anyhow!("Connection test failed: {e}"))
                }
            },
        }
    }

    pub async fn create_database_if_not_exists(&self) -> Result<()> {
        match self {
            Self::V1 { client, database } => {
                let query = ReadQuery::new(format!("CREATE DATABASE \"{database}\""));
                match client.query(query).await {
                    Ok(_) => {
                        info!("Database '{database}' created or already exists");
                        Ok(())
                    }
                    Err(e) => {
                        // Database might already exist, which is okay
                        debug!("Database creation result: {e}");
                        Ok(())
                    }
                }
            }
            Self::V2 { bucket, .. } => {
                // InfluxDB 2.x buckets are created via setup or API
                // For now, assume bucket exists or will be created externally
                info!("Using InfluxDB 2.x bucket: {bucket}");
                Ok(())
            }
        }
    }

    pub fn format_records_for_influx(&self, records: &[GNetTrackRecord]) -> Result<Vec<String>> {
        let mut formatted_queries = Vec::new();

        for record in records {
            let timestamp = record.timestamp.timestamp_nanos_opt().unwrap_or(0);

            let mut line = String::from("network_measurements,measurement_type=gnettrack");

            // Add tags
            if let Some(ref operator_name) = record.operator_name {
                line.push_str(&format!(",operator_name={operator_name}"));
            }
            if let Some(ref network_tech) = record.network_tech {
                line.push_str(&format!(",network_tech={network_tech}"));
            }

            line.push(' ');

            // Add fields
            let mut fields = Vec::new();
            if let Some(longitude) = record.longitude {
                fields.push(format!("longitude={longitude}"));
            }
            if let Some(latitude) = record.latitude {
                fields.push(format!("latitude={latitude}"));
            }
            if let Some(speed) = record.speed {
                fields.push(format!("speed={speed}"));
            }
            if let Some(level) = record.level {
                fields.push(format!("level={level}"));
            }

            line.push_str(&fields.join(","));
            line.push_str(&format!(" {timestamp}"));

            formatted_queries.push(line);
        }

        Ok(formatted_queries)
    }

    pub async fn write_records(&self, records: &[GNetTrackRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        match self {
            Self::V1 { client, database } => self.write_records_v1(client, database, records).await,
            Self::V2 { client, bucket, .. } => self.write_records_v2(client, bucket, records).await,
        }
    }

    async fn write_records_v1(
        &self,
        client: &InfluxDB1Client,
        database: &str,
        records: &[GNetTrackRecord],
    ) -> Result<()> {
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

            debug!("InfluxDB 1.x write query: {write_query:?}");
            write_queries.push(write_query);
        }

        info!(
            "Attempting to write {} records to InfluxDB 1.x...",
            records.len()
        );
        debug!("Writing to measurement 'network_measurements' in database '{database}'");

        match client.query(write_queries).await {
            Ok(_) => {
                info!(
                    "Successfully wrote {} records to InfluxDB 1.x",
                    records.len()
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to write records to InfluxDB 1.x: {e}");
                Err(anyhow!("Write operation failed: {e}"))
            }
        }
    }

    async fn write_records_v2(
        &self,
        client: &InfluxDB2Client,
        bucket: &str,
        records: &[GNetTrackRecord],
    ) -> Result<()> {
        let mut data_points = Vec::new();

        for record in records {
            let timestamp: DateTime<Utc> = record.timestamp;

            let mut data_point = DataPoint::builder("network_measurements")
                .timestamp(timestamp.timestamp_nanos_opt().unwrap_or(0))
                .tag("measurement_type", "gnettrack");

            // Add tags (indexed fields)
            if let Some(ref operator_name) = record.operator_name {
                data_point = data_point.tag("operator_name", operator_name);
            }
            if let Some(ref operator_code) = record.operator_code {
                data_point = data_point.tag("operator_code", operator_code);
            }
            if let Some(ref cell_id) = record.cell_id {
                data_point = data_point.tag("cell_id", cell_id);
            }
            if let Some(ref network_tech) = record.network_tech {
                data_point = data_point.tag("network_tech", network_tech);
            }
            if let Some(ref network_mode) = record.network_mode {
                data_point = data_point.tag("network_mode", network_mode);
            }
            if let Some(ref lac) = record.lac {
                data_point = data_point.tag("lac", lac);
            }

            // Add numeric fields
            if let Some(longitude) = record.longitude {
                data_point = data_point.field("longitude", longitude);
            }
            if let Some(latitude) = record.latitude {
                data_point = data_point.field("latitude", latitude);
            }
            if let Some(speed) = record.speed {
                data_point = data_point.field("speed", speed);
            }
            if let Some(level) = record.level {
                data_point = data_point.field("level", level);
            }
            if let Some(qual) = record.qual {
                data_point = data_point.field("qual", qual);
            }
            if let Some(snr) = record.snr {
                data_point = data_point.field("snr", snr);
            }
            if let Some(cqi) = record.cqi {
                data_point = data_point.field("cqi", cqi);
            }
            if let Some(dl_bitrate) = record.dl_bitrate {
                data_point = data_point.field("dl_bitrate", dl_bitrate);
            }
            if let Some(ul_bitrate) = record.ul_bitrate {
                data_point = data_point.field("ul_bitrate", ul_bitrate);
            }

            // Add string fields
            if let Some(ref cgi) = record.cgi {
                data_point = data_point.field("cgi", cgi.as_str());
            }
            if let Some(ref cellname) = record.cellname {
                data_point = data_point.field("cellname", cellname.as_str());
            }
            if let Some(ref node) = record.node {
                data_point = data_point.field("node", node.as_str());
            }
            if let Some(ref arfcn) = record.arfcn {
                data_point = data_point.field("arfcn", arfcn.as_str());
            }

            let built_point = data_point.build()?;
            debug!("InfluxDB 2.x data point: {built_point:?}");
            data_points.push(built_point);
        }

        info!(
            "Attempting to write {} records to InfluxDB 2.x...",
            records.len()
        );
        debug!("Writing to measurement 'network_measurements' in bucket '{bucket}'");

        match client.write(bucket, stream::iter(data_points)).await {
            Ok(_) => {
                info!(
                    "Successfully wrote {} records to InfluxDB 2.x",
                    records.len()
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to write records to InfluxDB 2.x: {e}");
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
