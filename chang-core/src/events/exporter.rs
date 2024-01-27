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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::migration, error::Error, utils};

    use chrono::Utc;
    use fake::faker::lorem::en::Word;
    use fake::Fake;
    use uuid::Uuid;

    #[tokio::test]
    async fn export_events() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::events(&prepare.pool).await;

        let exporter = ChangEventExporter::new(&prepare.pool);

        let records = get_records();

        exporter
            .export(records)
            .await
            .expect("failed to insert event records");

        utils::test::cleanup(prepare).await;
    }

    #[tokio::test]
    async fn pool_closed() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::events(&prepare.pool).await;

        let exporter = ChangEventExporter::new(&prepare.pool);

        exporter.db.close().await;

        let records = get_records();

        let error = exporter.export(records).await;
        utils::test::cleanup(prepare).await;

        assert!(error.is_err());

        let e = error.unwrap_err();
        assert!(matches!(
            e,
            Error::Other(x) if x == "exporter is shut down".to_string()
        ));
    }

    fn get_records() -> Vec<EventRecord> {
        let mut records: Vec<EventRecord> = Vec::new();
        for _ in 0..10 {
            records.push(EventRecord {
                id: Uuid::new_v4(),
                kind: Word().fake(),
                body: utils::test::get_object(),
                created_at: Utc::now(),
            });
        }

        records
    }
}
