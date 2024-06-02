use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::Execute;
use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use std::str::FromStr;
use url::Url;

use crate::utils::{self, name};

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Name(#[from] name::NameError),

    #[error("database/invalid_name")]
    InvalidName,
}

#[derive(Default)]
pub struct DatabaseInner {
    name: Option<String>,
    owner: Option<String>,
    template: Option<String>,
}

#[derive(Default)]
pub struct Database {
    inner: DatabaseInner,
}

impl Database {
    pub fn new() -> Database {
        Database::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.set_name(Some(name));
        self
    }

    pub fn set_name(&mut self, name: Option<&str>) {
        self.inner.name = name.map(|val| val.into());
    }

    pub fn owner(mut self, owner: &str) -> Self {
        self.set_owner(Some(owner));
        self
    }

    pub fn set_owner(&mut self, owner: Option<&str>) {
        self.inner.owner = owner.map(|val| val.into());
    }

    pub fn template(mut self, template: &str) -> Self {
        self.set_template(Some(template));
        self
    }

    pub fn set_template(&mut self, template: Option<&str>) {
        self.inner.template = template.map(|val| val.into());
    }

    pub async fn create(self, db: impl PgExecutor<'_>) -> Result<(), DatabaseError> {
        let name = self.inner.name.ok_or(DatabaseError::InvalidName)?;

        utils::name::is_valid(&name)?;

        let base_query = format!("create database {}", name);
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(base_query);

        if let Some(owner) = self.inner.owner {
            utils::name::is_valid(&owner)?;
            let query = format!("\nowner = {owner}");
            query_builder.push(query);
        }

        if let Some(template) = self.inner.template {
            utils::name::is_valid(&template)?;
            let query = format!("\ntemplate = {template}");
            query_builder.push(query);
        }

        let query = query_builder.build();
        let sql = query.sql();

        sqlx::query(sql).execute(db).await?;

        Ok(())
    }
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

    Database::new().name(database).create(&pool).await?;
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
