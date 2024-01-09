use sqlx::PgPool;
use std::env;

use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, types::Uuid};
use std::ops::Deref;
use tokio::{self, join};

use chang::{
    db::tasks::Task as DbTask,
    task::{
        self, Context, DbTaskError, FromTaskContext, TaskBuildError, TaskContextError, TaskKind,
        Tasks,
    },
    Task,
};

#[derive(Debug, Clone)]
pub struct Db(pub PgPool);

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("PgPool not found in Context")]
    PoolNotFound,
}

impl FromTaskContext for Db {
    type Error = DbError;

    fn from_context(ctx: &Context) -> Result<Self, Self::Error> {
        let pool = ctx.get::<PgPool>().ok_or(DbError::PoolNotFound)?;
        Ok(Db(pool.to_owned()))
    }
}

impl Deref for Db {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(thiserror::Error, Debug)]
enum TaskError {
    #[error(transparent)]
    TaskContextError(#[from] TaskContextError),

    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    DbCtx(#[from] DbError),

    #[error(transparent)]
    Task(#[from] DbTaskError),

    #[error(transparent)]
    TaskBuildError(#[from] TaskBuildError),

    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
}

#[derive(Task, Deserialize, Serialize, Debug)]
struct ChildTask {
    hello: String,
    nested: bool,
}

async fn handle_child_task(ctx: Context) -> anyhow::Result<()> {
    let child_task = ChildTask::from_context(&ctx)?;
    println!("{:?}", child_task);

    if child_task.nested {
        return Ok(());
    }

    println!("Insert Another ChildTask");

    let task = DbTask::from_context(&ctx)?;
    let db = Db::from_context(&ctx)?;

    let child = ChildTask {
        hello: "Chang".to_string(),
        nested: true,
    };

    task::Task::builder()
        .task(child)?
        .dependend_id(&task.dependend_id.unwrap())
        .build()?
        .insert(&*db)
        .await?;

    Ok(())
}

#[derive(Task, Deserialize, Serialize, Debug)]
struct ParentTask {
    parent: String,
}

async fn handle_parent_task(ctx: Context) -> anyhow::Result<()> {
    let task = ParentTask::from_context(&ctx)?;
    println!("{:?}", task);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Valid DB connection");

    let db = Db(pool.clone());
    let tasks = Tasks::router()
        .register(ParentTask::kind(), handle_parent_task)
        .register(ChildTask::kind(), handle_child_task)
        .add_context(db)
        .connect(pool.clone())
        .start();

    let insert_pool = pool.clone();
    let task_fut = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let dependend_id = Uuid::new_v4();

            let child = ChildTask {
                hello: "Chang".to_string(),
                nested: false,
            };

            task::Task::builder()
                .task(child)
                .unwrap()
                .dependend_id(&dependend_id)
                .build()
                .unwrap()
                .insert(&insert_pool)
                .await
                .unwrap();

            let parent = ParentTask {
                parent: dependend_id.to_string(),
            };

            task::Task::builder()
                .task(parent)
                .unwrap()
                .depends_on(&dependend_id)
                .build()
                .unwrap()
                .insert(&insert_pool)
                .await
                .unwrap();
        }
    });

    let _ = join!(task_fut, tasks);

    Ok(())
}
