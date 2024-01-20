use chang_core::task::TaskState;
use dotenv::dotenv;
use log::{error, info, Level, LevelFilter, Metadata, Record};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::{postgres::PgPoolOptions, types::Uuid};
use std::env;
use tokio;

use chang::{
    task::{self, Context, Db, FromTaskContext, Task, TaskBuilder, TaskKind, TaskRunner},
    Task,
};

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

#[derive(Task, Deserialize, Serialize, Debug)]
struct ChildTask {
    hello: String,
    nested: bool,
}

async fn handle_child_task(ctx: Context) -> anyhow::Result<TaskState> {
    info!("Run Child Task");

    let child_task = ChildTask::from_context(&ctx)?;
    info!("{:?}", child_task);

    if child_task.nested {
        return Ok(TaskState::Completed);
    }

    info!("Insert Another ChildTask");

    let db = Db::from_context(&ctx)?;
    let task = Task::from_context(&ctx)?;

    let child = ChildTask {
        hello: "Chang".to_string(),
        nested: true,
    };

    task::try_from(child)?
        .dependend_id(&task.dependend_id.unwrap())
        .build()?
        .insert(&*db)
        .await?;

    Ok(TaskState::Completed)
}

#[derive(Task, Deserialize, Serialize, Debug)]
struct ParentTask {
    parent: String,
}

async fn handle_parent_task(ctx: Context) -> anyhow::Result<TaskState> {
    let task = ParentTask::from_context(&ctx)?;
    info!("{:?}", task);
    Ok(TaskState::Completed)
}

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

    let tasks = TaskRunner::new()
        .register(ParentTask::kind(), handle_parent_task)
        .register(ChildTask::kind(), handle_child_task)
        .concurrency(10)
        .connect(&pool)
        .start();

    let insert_pool = pool.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            match insert_tasks(&insert_pool).await {
                Ok(_) => info!("Tasks Inserted"),
                Err(err) => error!("Failed to insert tasks: {:?}", err),
            };
        }
    });

    let _ = tasks.await;

    Ok(())
}

async fn insert_tasks(db: &PgPool) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;

    let dependend_id = Uuid::new_v4();

    let child = ChildTask {
        hello: "Chang".to_string(),
        nested: false,
    };

    task::try_from(child)?
        .dependend_id(&dependend_id)
        .build()?
        .insert(&mut *tx)
        .await?;

    let parent = ParentTask {
        parent: dependend_id.to_string(),
    };

    task::try_from(parent)?
        .depends_on(&dependend_id)
        .build()?
        .insert(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}
