use async_trait::async_trait;
use core::fmt;
use opentelemetry::metrics::Result;
use opentelemetry::ExportError;
use opentelemetry_sdk::metrics::{
    data,
    exporter::PushMetricsExporter,
    reader::{
        AggregationSelector, DefaultAggregationSelector, DefaultTemporalitySelector,
        TemporalitySelector,
    },
    Aggregation, InstrumentKind,
};
use sqlx::PgPool;

use crate::db::metrics::MetricsService;

pub struct ChangMetricsExporter {
    db: PgPool,
    temporality_selector: Box<dyn TemporalitySelector>,
    aggregation_selector: Box<dyn AggregationSelector>,
}

impl ChangMetricsExporter {
    /// Create a builder to configure this exporter.
    pub fn builder(db: &PgPool) -> ChangMetricsExporterBuilder {
        ChangMetricsExporterBuilder {
            db: db.clone(),
            temporality_selector: Box::new(DefaultTemporalitySelector::new()),
            aggregation_selector: Box::new(DefaultAggregationSelector::new()),
        }
    }

    pub fn new(db: &PgPool) -> ChangMetricsExporter {
        ChangMetricsExporter {
            db: db.clone(),
            temporality_selector: Box::new(DefaultTemporalitySelector::new()),
            aggregation_selector: Box::new(DefaultAggregationSelector::new()),
        }
    }
}

impl fmt::Debug for ChangMetricsExporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MetricsExporter")
    }
}

impl TemporalitySelector for ChangMetricsExporter {
    fn temporality(&self, kind: InstrumentKind) -> data::Temporality {
        self.temporality_selector.temporality(kind)
    }
}

impl AggregationSelector for ChangMetricsExporter {
    fn aggregation(&self, kind: InstrumentKind) -> Aggregation {
        self.aggregation_selector.aggregation(kind)
    }
}

#[async_trait]
impl PushMetricsExporter for ChangMetricsExporter {
    async fn export(&self, metrics: &mut data::ResourceMetrics) -> Result<()> {
        if !self.db.is_closed() {
            let data = crate::otel::metrics::MetricsData::from(metrics);
            MetricsService::batch_insert(&self.db, data)
                .await
                .map_err(Error)?;
            Ok(())
        } else {
            Err(opentelemetry::metrics::MetricsError::Other(
                "exporter is shut down".into(),
            ))
        }
    }

    async fn force_flush(&self) -> Result<()> {
        // exporter holds no state, nothing to flush
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Configuration for the stdout metrics exporter
pub struct ChangMetricsExporterBuilder {
    db: PgPool,
    temporality_selector: Box<dyn TemporalitySelector>,
    aggregation_selector: Box<dyn AggregationSelector>,
}

impl ChangMetricsExporterBuilder {
    /// Set the temporality exporter for the exporter
    pub fn with_temporality_selector(
        mut self,
        selector: impl TemporalitySelector + 'static,
    ) -> Self {
        self.temporality_selector = Box::new(selector);
        self
    }

    /// Set the aggregation exporter for the exporter
    pub fn with_aggregation_selector(
        mut self,
        selector: impl AggregationSelector + 'static,
    ) -> Self {
        self.aggregation_selector = Box::new(selector);
        self
    }

    /// Create a metrics exporter with the current configuration
    pub fn build(self) -> ChangMetricsExporter {
        ChangMetricsExporter {
            db: self.db,
            temporality_selector: self.temporality_selector,
            aggregation_selector: self.aggregation_selector,
        }
    }
}

impl fmt::Debug for ChangMetricsExporterBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MetricsExporterBuilder")
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
