use serde_json;
use sqlx::PgExecutor;

use crate::{error::Result, otel::logs::transform::LogData};

pub struct LogsService;

impl LogsService {
    pub async fn batch_insert(db: impl PgExecutor<'_>, data: LogData) -> Result<()> {
        let logs = serde_json::to_value(&data.logs)?;
        sqlx::query_file!("src/db/logs/sql/batch_insert.sql", logs)
            .execute(db)
            .await?;

        Ok(())
    }
}
