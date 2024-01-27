use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use url::Url;

use crate::utils::name;

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Name(#[from] name::NameError),
}

pub async fn create(database_url: &str) -> Result<(), DatabaseError> {
    let mut url = Url::parse(database_url).expect("Invalid database url");

    let database_url = url.clone();
    let database = database_url
        .path_segments()
        .and_then(|mut segments| segments.next())
        .expect("database name");

    name::is_valid(&database)?;

    url.set_path("postgres");

    let postgres_connection = url.to_string();
    let pool = connect(&postgres_connection).await?;

    let db_exists = exists(&pool, database).await?;

    if db_exists {
        return Ok(());
    }

    let query = format!("create database {}", database);
    sqlx::query(&query).execute(&pool).await?;
    Ok(())
}

pub async fn exists(pool: &PgPool, database: &str) -> Result<bool, DatabaseError> {
    let row = sqlx::query(
        "
	    select count(*)
	      from pg_catalog.pg_database
	     where datname = $1
	",
    )
    .bind(database)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<i64, &str>("count") != 0)
}

pub async fn drop(pool: &PgPool, database: &str) -> Result<(), DatabaseError> {
    let db_exists = exists(pool, database).await?;

    if !db_exists {
        return Ok(());
    }

    let query = format!("drop database if exists {};", database);
    sqlx::query(&query).execute(pool).await?;

    Ok(())
}

pub async fn connect(database_url: &str) -> Result<PgPool, DatabaseError> {
    let url = Url::parse(database_url).expect("Invalid database url");

    let options = PgConnectOptions::from_str(url.as_ref())
        .expect("valid db connection string")
        .to_owned();

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    Ok(pool)
}
