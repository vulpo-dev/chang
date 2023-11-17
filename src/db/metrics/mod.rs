use serde_json;
use sqlx::PgExecutor;

use crate::{error::Result, otel::metrics::MetricsData};

pub struct MetricsService;

impl MetricsService {
    pub async fn batch_insert(db: impl PgExecutor<'_>, data: MetricsData) -> Result<()> {
        let logs = serde_json::to_value(&data.metrics)?;
        sqlx::query_file!("src/db/metrics/sql/batch_insert.sql", logs)
            .execute(db)
            .await?;

        Ok(())
    }
}
