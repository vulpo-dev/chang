use chang::otel::logs::{ChangLogBridge, ChangLogExporter};

use dotenv::dotenv;
use log::{error, info, Level};
use opentelemetry::KeyValue;
use opentelemetry_sdk::logs::{Config, LoggerProvider};
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::Resource;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[derive(Serialize)]
struct Nested {
    inner: i32,
}

#[derive(Serialize)]
struct Yak {
    name: String,
    other: String,
    nested: Nested,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Valid DB connection");

    let exporter = ChangLogExporter::new(&pool);

    let logger_provider = LoggerProvider::builder()
        .with_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "chang-logs-example",
            )])),
        )
        .with_batch_exporter(exporter, runtime::Tokio)
        .build();

    // Setup Log Appender for the log crate.
    let otel_log_appender = ChangLogBridge::new(&logger_provider);
    log::set_boxed_logger(Box::new(otel_log_appender)).unwrap();
    log::set_max_level(Level::Info.to_level_filter());

    // Emit logs using macros from the log crate.
    // These logs gets piped through OpenTelemetry bridge and gets exported to postgres.
    error!(target: "my-target", "hello from {}. My price is {}", "apple", 2.99);

    let yak = Yak {
        name: String::from("test"),
        other: String::from("other"),
        nested: Nested { inner: 1 },
    };
    info!(target: "yak_events", a_key:serde = yak, other:serde = yak; "Commencing yak shaving");
}
