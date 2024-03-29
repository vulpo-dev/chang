use core::fmt;
use futures_util::future::BoxFuture;
use opentelemetry::{trace::TraceError, ExportError};
use opentelemetry_sdk::export::{self, trace::ExportResult};
use sqlx::PgPool;

use crate::{db::spans::SpanService, otel::traces::transform::SpanData};

pub struct ChangSpanExporter {
    db: PgPool,
}

impl fmt::Debug for ChangSpanExporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChangSpanExporter")
    }
}

impl ChangSpanExporter {
    pub fn new(pool: &PgPool) -> ChangSpanExporter {
        let db = pool.clone();
        ChangSpanExporter { db }
    }
}

impl opentelemetry_sdk::export::trace::SpanExporter for ChangSpanExporter {
    fn export(&mut self, batch: Vec<export::trace::SpanData>) -> BoxFuture<'static, ExportResult> {
        if !self.db.is_closed() {
            let data = SpanData::from(batch);
            let db = self.db.clone();

            Box::pin(async move {
                SpanService::batch_insert(&db, data).await.map_err(|err| {
                    let err = Error::from(err);
                    TraceError::from(err)
                })
            })
        } else {
            Box::pin(std::future::ready(Err("exporter is shut down".into())))
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
struct Error(#[from] crate::error::Error);

impl ExportError for Error {
    fn exporter_name(&self) -> &'static str {
        "postgres"
    }
}
