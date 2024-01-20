use async_trait::async_trait;
use sqlx::PgPool;

use crate::db::events::EventsService;
use crate::error::{Error, Result};
use crate::events::transform::EventRecord;

use super::transform::EventData;

#[async_trait]
pub trait EventExporter: Send + Sync {
    async fn export(&self, batch: Vec<EventRecord>) -> Result<()>;
}

pub struct ChangEventExporter {
    db: PgPool,
}

impl ChangEventExporter {
    pub fn new(pool: &PgPool) -> ChangEventExporter {
        let db = pool.clone();
        ChangEventExporter { db }
    }
}

#[async_trait]
impl EventExporter for ChangEventExporter {
    async fn export(&self, batch: Vec<EventRecord>) -> Result<()> {
        if !self.db.is_closed() {
            let data = EventData::from(batch);
            EventsService::batch_insert(&self.db, data).await?;
            Ok(())
        } else {
            Err(Error::Other("exporter is shut down".to_string()))
        }
    }
}
