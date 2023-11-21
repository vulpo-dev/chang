use async_trait::async_trait;
use core::fmt;
use opentelemetry::ExportError;
use opentelemetry_sdk::export::logs::{ExportResult, LogData};
use sqlx::PgPool;

use crate::db::logs::LogsService;

/// A [`ChangLogExporter`] that writes to PostgreSQL.
///
/// [`ChangLogExporter`]: chang::otel::logs::ChangLogExporter
/// [`Db`]: sqlx::PgPool
pub struct ChangLogExporter {
    db: PgPool,
}

impl ChangLogExporter {
    /// Create a builder to configure this exporter.
    pub fn new(pool: &PgPool) -> ChangLogExporter {
        let db = pool.clone();
        ChangLogExporter { db }
    }
}

impl fmt::Debug for ChangLogExporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChangLogsExporter")
    }
}

#[async_trait]
impl opentelemetry_sdk::export::logs::LogExporter for ChangLogExporter {
    /// Export spans to postgres
    async fn export(&mut self, batch: Vec<LogData>) -> ExportResult {
        if self.db.is_closed() == false {
            let data = crate::otel::logs::transform::LogData::from(batch);
            LogsService::batch_insert(&self.db, data)
                .await
                .map_err(Error)?;

            Ok(())
        } else {
            Err("exporter is shut down".into())
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
struct Error(#[from] crate::error::Error);

impl ExportError for Error {
    fn exporter_name(&self) -> &'static str {
        "chang"
    }
}
