use chang_core::partition::Partition;
use dotenv::dotenv;
use log::{info, Level, LevelFilter, Metadata, Record};

use sqlx::postgres::PgPoolOptions;
use sqlx::Postgres;
use std::env;
use tokio;

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info));

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable");
    info!("DATABASE_URL: {:?}", database_url);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Valid DB connection");

    sqlx::query::<Postgres>(
        "create table if not exists chang.partition_example(
          id uuid not null default uuid_generate_v4()
        , created_at timestamptz not null default now()
    ) partition by range(created_at)",
    )
    .execute(&pool)
    .await?;

    Partition::new()
        .name("chang.partition_example_fuu")
        .from("chang.partition_example")
        .range(("2006-02-01", "2006-03-01"))
        .create(&pool)
        .await?;

    sqlx::query::<Postgres>(
        "create unique index partition_example_fuu_idx on chang.partition_example_fuu (id);",
    )
    .execute(&pool)
    .await?;

    Partition::new()
        .name("chang.partition_example_fuu")
        .from("chang.partition_example")
        .detach(&pool)
        .await?;

    Partition::new()
        .name("chang.partition_example_fuu")
        .drop(&pool)
        .await?;

    sqlx::query::<Postgres>("drop table chang.partition_example")
        .execute(&pool)
        .await?;

    Ok(())
}
