use log::debug;
use sqlx::Execute;
use sqlx::{PgExecutor, Postgres, QueryBuilder};

use crate::utils::{self, name::NameError};

pub struct PartitionHash {
    pub modulus: u8,
    pub remainder: u8,
}

impl From<(u8, u8)> for PartitionHash {
    fn from(value: (u8, u8)) -> Self {
        PartitionHash {
            modulus: value.0,
            remainder: value.1,
        }
    }
}

pub struct PartitionRange {
    pub from: String,
    pub to: String,
}

impl From<(String, String)> for PartitionRange {
    fn from(value: (String, String)) -> Self {
        PartitionRange {
            from: value.0,
            to: value.1,
        }
    }
}

impl From<(&str, &str)> for PartitionRange {
    fn from(value: (&str, &str)) -> Self {
        PartitionRange {
            from: value.0.to_string(),
            to: value.1.to_string(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PartitionError {
    #[error(transparent)]
    TableName(#[from] NameError),

    #[error("partion name is empty")]
    PartitionNameMissing,

    #[error("partion source name is empty")]
    FromTableNameMissing,

    #[error("invalid arguments")]
    InvalidArguments,

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Default)]
pub struct PartitionInner {
    name: Option<String>,
    from: Option<String>,
    range: Option<PartitionRange>,
    hash: Option<PartitionHash>,
    list: Option<Vec<String>>,
    tablespace: Option<String>,
}

#[derive(Default)]
pub struct Partition {
    inner: PartitionInner,
}

impl Partition {
    pub fn new() -> Partition {
        Partition::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.set_name(Some(name));
        self
    }

    pub fn set_name(&mut self, name: Option<&str>) {
        self.inner.name = name.map(|val| val.into());
    }

    pub fn from(mut self, from: &str) -> Self {
        self.set_from(Some(from));
        self
    }

    pub fn set_from(&mut self, from: Option<&str>) {
        self.inner.from = from.map(|val| val.into());
    }

    pub fn range(mut self, range: impl Into<PartitionRange>) -> Self {
        self.set_range(Some(range));
        self
    }

    pub fn set_range(&mut self, range: Option<impl Into<PartitionRange>>) {
        self.inner.range = range.map(|val| val.into());
    }

    pub fn hash(mut self, hash: impl Into<PartitionHash>) -> Self {
        self.set_hash(Some(hash));
        self
    }

    pub fn set_hash(&mut self, hash: Option<impl Into<PartitionHash>>) {
        self.inner.hash = hash.map(|val| val.into());
    }

    pub async fn create(self, db: impl PgExecutor<'_>) -> Result<(), PartitionError> {
        let name = self
            .inner
            .name
            .ok_or(PartitionError::PartitionNameMissing)?;

        utils::name::is_valid(&name)?;

        let from = self
            .inner
            .from
            .ok_or(PartitionError::FromTableNameMissing)?;

        utils::name::is_valid(&from)?;

        let args = [
            self.inner.range.is_none(),
            self.inner.hash.is_none(),
            self.inner.list.is_none(),
        ];

        let all_none = args.iter().all(|value| *value);
        let count_some = args.iter().filter(|value| !**value).count();

        if all_none || count_some > 1 {
            return Err(PartitionError::InvalidArguments);
        }

        let base_query =
            format!("create table if not exists {name} partition of {from} for values ",);

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(base_query);

        if let Some(range) = self.inner.range {
            let range_query = format!("from ('{}') to ('{}')", range.from, range.to);
            query_builder.push(range_query);
        }

        if let Some(list) = self.inner.list {
            let items = list
                .iter()
                .map(|val| val.replace('\'', "\\'"))
                .map(|val| format!("'{}'", val))
                .collect::<Vec<String>>()
                .join(",");

            let list_query = format!("in ({})", items);
            query_builder.push(list_query);
        }

        if let Some(hash) = self.inner.hash {
            let hash_query = format!(
                "with (modulus {}, remainder {})",
                hash.modulus, hash.remainder
            );
            query_builder.push(hash_query);
        }

        if let Some(tablespace) = self.inner.tablespace {
            utils::name::is_valid(&tablespace)?;
            let tablespace_query = format!("tablespace {tablespace}");
            query_builder.push(tablespace_query);
        }

        let query = query_builder.build();
        let sql = query.sql();
        debug!("{:?}", sql);
        sqlx::query(sql).execute(db).await?;

        Ok(())
    }

    pub async fn detach(self, db: impl PgExecutor<'_>) -> Result<(), PartitionError> {
        let name = self
            .inner
            .name
            .ok_or(PartitionError::PartitionNameMissing)?;

        utils::name::is_valid(&name)?;

        let from = self
            .inner
            .from
            .ok_or(PartitionError::FromTableNameMissing)?;

        utils::name::is_valid(&from)?;

        let query = format!("alter table {from} detach partition {name};");
        debug!("{:?}", query);
        sqlx::query(&query).execute(db).await?;

        Ok(())
    }

    pub async fn detach_concurrently(self, db: impl PgExecutor<'_>) -> Result<(), PartitionError> {
        let name = self
            .inner
            .name
            .ok_or(PartitionError::PartitionNameMissing)?;

        utils::name::is_valid(&name)?;

        let from = self
            .inner
            .from
            .ok_or(PartitionError::FromTableNameMissing)?;

        utils::name::is_valid(&from)?;

        let query = format!("alter table {from} detach partition {name} concurrently;");
        debug!("{:?}", query);
        sqlx::query(&query).execute(db).await?;

        Ok(())
    }

    pub async fn drop(self, db: impl PgExecutor<'_>) -> Result<(), PartitionError> {
        let name = self
            .inner
            .name
            .ok_or(PartitionError::PartitionNameMissing)?;

        utils::name::is_valid(&name)?;

        let query = format!("drop table {name};");
        debug!("{:?}", query);
        sqlx::query(&query).execute(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils;

    use sqlx::postgres::Postgres;

    #[tokio::test]
    async fn can_create_partition() {
        let prepare = utils::test::prepare().await;

        sqlx::query::<Postgres>(
            "create table if not exists partition_example(
                  id serial
                , created_at timestamptz not null default now()
            ) partition by range(created_at)",
        )
        .execute(&prepare.pool)
        .await
        .unwrap();

        Partition::new()
            .name("partition_example_fuu")
            .from("partition_example")
            .range(("2006-02-01", "2006-03-01"))
            .create(&prepare.pool)
            .await
            .unwrap();

        sqlx::query::<Postgres>(
            "create unique index partition_example_fuu_idx on partition_example_fuu (id);",
        )
        .execute(&prepare.pool)
        .await
        .unwrap();

        Partition::new()
            .name("partition_example_fuu")
            .from("partition_example")
            .detach(&prepare.pool)
            .await
            .unwrap();

        Partition::new()
            .name("partition_example_fuu")
            .drop(&prepare.pool)
            .await
            .unwrap();

        utils::test::cleanup(prepare).await;
    }
}
