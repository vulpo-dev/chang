use std::env;

use dotenv::dotenv;
use opentelemetry::{
    trace::{Span, Tracer, TracerProvider as _},
    KeyValue,
};
use opentelemetry_sdk::{runtime, trace::TracerProvider};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Valid DB connection");

    let exporter = chang::otel::traces::ChangSpanExporter::new(&pool);
    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .build();

    let tracer = tracer_provider.tracer("stdout-test");
    let mut span = tracer.start("test_span");
    span.set_attribute(KeyValue::new("test_key", "test_value"));
    span.end();

    Ok(())
}
