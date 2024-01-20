use chang::events::{self, exporter::ChangEventExporter, ChangEventCollector, Event};

use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::env;
use tokio::time::{sleep, Duration};

#[derive(Deserialize, Serialize)]
struct Nested {
    inner: i32,
}

#[derive(Deserialize, Serialize)]
struct Yak {
    name: String,
    other: String,
    nested: Nested,
}

impl Event for Yak {
    fn kind() -> String {
        String::from("yak")
    }

    fn from_event(value: &serde_json::Value) -> serde_json::Result<Self> {
        serde_json::from_value(value.clone())
    }
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

    let exporter = ChangEventExporter::new(&pool);
    let _collector = ChangEventCollector::builder()
        .with_exporter(exporter)
        .with_interval(Duration::from_secs(1))
        .start();

    let yak = Yak {
        name: String::from("test"),
        other: String::from("other"),
        nested: Nested { inner: 1 },
    };

    events::publish(yak);

    sleep(Duration::from_secs(2)).await;
}
