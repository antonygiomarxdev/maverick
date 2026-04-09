use std::sync::Arc;

use async_trait::async_trait;
use maverick_domain::UplinkFrame;

use crate::adapters::persistence::sqlite_utils::{
    blob_literal, optional_blob_literal, optional_i64, optional_text_literal,
};
use crate::db::Database;
use crate::ports::UplinkRepository;
use crate::Result;

pub struct SqliteUplinkRepository<D: Database> {
    db: Arc<D>,
}

impl<D: Database> SqliteUplinkRepository<D> {
    pub fn new(db: Arc<D>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<D: Database> UplinkRepository for SqliteUplinkRepository<D> {
    async fn append(&self, uplink: UplinkFrame) -> Result<()> {
        let query = format!(
            "INSERT INTO uplinks (dev_eui, gateway_eui, payload, f_port, rssi, snr, frequency_hz, spreading_factor, frame_counter, received_at, raw_frame, channel, code_rate, modulation, bandwidth_hz) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
            optional_blob_literal(uplink.dev_eui.as_ref().map(|value| &value.as_bytes_slice()[..])),
            blob_literal(uplink.gateway_eui.as_bytes_slice()),
            blob_literal(&uplink.payload),
            optional_i64(uplink.f_port.map(|value| value as i64)),
            uplink.rssi.as_i16(),
            uplink.snr.as_f32(),
            uplink.frequency.as_hz(),
            uplink.spreading_factor.0,
            optional_i64(uplink.frame_counter.map(|value| value as i64)),
            uplink.timestamp,
            blob_literal(&uplink.raw_frame),
            uplink.metadata.channel,
            optional_text_literal(uplink.metadata.code_rate.as_deref()),
            optional_text_literal(uplink.metadata.modulation.as_deref()),
            optional_i64(uplink.metadata.bandwidth.map(|value| value as i64)),
        );

        self.db.execute(&query).await?;
        Ok(())
    }

    async fn append_batch(&self, uplinks: Vec<UplinkFrame>) -> Result<()> {
        if uplinks.is_empty() {
            return Ok(());
        }

        let mut sql = String::from("BEGIN;\n");
        for uplink in uplinks {
            sql.push_str(&format!(
                "INSERT INTO uplinks (dev_eui, gateway_eui, payload, f_port, rssi, snr, frequency_hz, spreading_factor, frame_counter, received_at, raw_frame, channel, code_rate, modulation, bandwidth_hz) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {});\n",
                optional_blob_literal(uplink.dev_eui.as_ref().map(|v| &v.as_bytes_slice()[..])),
                blob_literal(uplink.gateway_eui.as_bytes_slice()),
                blob_literal(&uplink.payload),
                optional_i64(uplink.f_port.map(|v| v as i64)),
                uplink.rssi.as_i16(),
                uplink.snr.as_f32(),
                uplink.frequency.as_hz(),
                uplink.spreading_factor.0,
                optional_i64(uplink.frame_counter.map(|v| v as i64)),
                uplink.timestamp,
                blob_literal(&uplink.raw_frame),
                uplink.metadata.channel,
                optional_text_literal(uplink.metadata.code_rate.as_deref()),
                optional_text_literal(uplink.metadata.modulation.as_deref()),
                optional_i64(uplink.metadata.bandwidth.map(|v| v as i64)),
            ));
        }
        sql.push_str("COMMIT;");

        self.db.execute_batch(&sql).await?;
        Ok(())
    }
}
