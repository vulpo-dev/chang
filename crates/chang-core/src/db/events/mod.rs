use serde_json;
use sqlx::PgExecutor;

use crate::{error::Result, events::transform::EventData};

pub struct EventsService;

impl EventsService {
    pub async fn batch_insert(db: impl PgExecutor<'_>, data: EventData) -> Result<()> {
        let events = serde_json::to_value(&data.events)?;
        sqlx::query_file!("src/db/events/sql/batch_insert.sql", events)
            .execute(db)
            .await?;

        Ok(())
    }
}
