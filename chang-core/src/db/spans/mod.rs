use serde_json;
use sqlx::PgExecutor;

use crate::{error::Result, otel::traces::transform::SpanData};

pub struct SpanService;

impl SpanService {
    pub async fn batch_insert(db: impl PgExecutor<'_>, data: SpanData) -> Result<()> {
        let spans = serde_json::to_value(&data.spans)?;
        sqlx::query_file!("src/db/spans/sql/batch_insert.sql", spans)
            .execute(db)
            .await?;

        Ok(())
    }
}
