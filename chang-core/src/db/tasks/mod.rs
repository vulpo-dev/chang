use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

pub struct NewTask {
    pub scheduled_at: Option<DateTime<Utc>>,
    pub max_attempts: i16,
    pub attempted_by: Vec<String>,
    pub tags: Vec<String>,
    pub kind: String,
    pub args: serde_json::Value,
    pub priority: i16,
    pub queue: Option<String>,
    pub depends_on: Option<Uuid>,
    pub dependend_id: Option<Uuid>,
}

#[derive(sqlx::Type, PartialEq, Debug, Clone, Deserialize, Serialize)]
#[sqlx(type_name = "chang.task_state")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Available,
    Cancelled,
    Completed,
    Discarded,
    Retryable,
    Running,
    Scheduled,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: Uuid,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub attempt: i16,
    pub max_attempts: i16,
    pub attempted_by: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub kind: String,
    pub args: serde_json::Value,
    pub priority: i16,
    pub queue: Option<String>,
    pub depends_on: Option<Uuid>,
    pub dependend_id: Option<Uuid>,
}

pub struct TaskService;

impl TaskService {
    pub async fn insert(db: impl PgExecutor<'_>, task: NewTask) -> sqlx::Result<Uuid> {
        let row = sqlx::query_file!(
            "src/db/tasks/sql/insert.sql",
            task.max_attempts,
            task.scheduled_at,
            task.priority,
            task.args,
            &task.attempted_by,
            task.kind,
            task.queue,
            &task.tags,
            task.depends_on,
            task.dependend_id
        )
        .fetch_one(db)
        .await?;

        Ok(row.id)
    }

    pub async fn get_tasks(db: impl PgExecutor<'_>) -> sqlx::Result<Vec<Uuid>> {
        let rows = sqlx::query_file!("src/db/tasks/sql/get_tasks.sql")
            .fetch_all(db)
            .await?;

        let ids = rows.into_iter().map(|row| row.id).collect::<Vec<Uuid>>();
        Ok(ids)
    }

    pub async fn get_sheduled_task(
        db: impl PgExecutor<'_>,
        task_id: &Uuid,
    ) -> sqlx::Result<Option<Task>> {
        sqlx::query_file_as!(Task, "src/db/tasks/sql/get_sheduled_task.sql", task_id)
            .fetch_optional(db)
            .await
    }

    pub async fn complete(db: impl PgExecutor<'_>, task_id: &Uuid) -> sqlx::Result<()> {
        sqlx::query_file!("src/db/tasks/sql/complete.sql", task_id)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn failed(db: impl PgExecutor<'_>, task_id: &Uuid, error: &str) -> sqlx::Result<()> {
        sqlx::query_file!("src/db/tasks/sql/failed.sql", task_id, error)
            .execute(db)
            .await?;

        Ok(())
    }
}
