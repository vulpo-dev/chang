use dotenv::dotenv;

use fake::{
    faker::lorem::en::{Word, Words},
    Fake,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use uuid::Uuid;

use crate::{database, utils::connection::ConnectionString};

pub struct Prepare {
    pub pool: PgPool,
    pub name: String,
    pub connection_string: ConnectionString,
}

pub async fn prepare() -> Prepare {
    dotenv().ok();

    let id = Uuid::new_v4().to_string().replace('-', "");
    let name = format!("test_{id}");

    let mut connection_string = ConnectionString::try_from(
        env::var("DATABASE_URL").expect("DATABASE_URL environment variable"),
    )
    .expect("valid database url");

    connection_string.set_database(Some(name.clone()));

    let database_url = connection_string.to_string();

    database::create(&database_url)
        .await
        .expect("failed to create database");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Valid DB connection");

    Prepare {
        pool,
        name,
        connection_string,
    }
}

pub async fn cleanup(mut prepare: Prepare) {
    prepare.pool.close().await;

    prepare
        .connection_string
        .set_database(Some("postgres".into()));

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&prepare.connection_string.to_string())
        .await
        .expect("Valid DB connection");

    database::drop(&pool, &prepare.name)
        .await
        .expect("failed to drop database");
}

pub fn get_object() -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for _ in 0..10 {
        let key = Word().fake();
        let s: String = Words(5..10).fake::<Vec<String>>().join(" ");
        let value = serde_json::Value::String(s);
        map.insert(key, value);
    }

    serde_json::Value::Object(map)
}

pub mod events {
    use crate::events::transform::EventRecord;
    use crate::utils::test;
    use fake::{faker::lorem::en::Word, Fake};

    pub fn get_records() -> Vec<EventRecord> {
        let mut records: Vec<EventRecord> = Vec::new();
        for _ in 0..10 {
            let kind = Word().fake();
            let body = test::get_object();
            records.push(EventRecord::new(kind, body));
        }

        records
    }
}
