use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

pub fn try_from(
    task: impl TryInto<TaskBuilder, Error = serde_json::Error>,
) -> Result<TaskBuilder, serde_json::Error> {
    task.try_into()
}

#[derive(PartialEq, Debug, Clone, Serialize)]
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

impl NewTask {
    pub async fn insert(self, db: impl PgExecutor<'_>) -> sqlx::Result<Uuid> {
        Box::pin(TaskService::insert(db, self)).await
    }
}

#[typeshare]
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

pub trait TaskKind {
    fn kind() -> String
    where
        Self: Sized;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: Uuid,
    pub state: TaskState,
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

impl Task {
    pub fn builder() -> TaskBuilder {
        TaskBuilder::default()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskBuildError {
    #[error("kind missing")]
    KindMissing,

    #[error("args missing")]
    ArgsMissing,
}

struct TaskBuilderInner {
    scheduled_at: Option<DateTime<Utc>>,
    max_attempts: Option<i16>,
    attempted_by: Vec<String>,
    tags: Vec<String>,
    kind: Option<String>,
    args: Option<serde_json::Value>,
    priority: Option<i16>,
    queue: Option<String>,
    depends_on: Option<Uuid>,
    dependend_id: Option<Uuid>,
}

pub struct TaskBuilder {
    inner: TaskBuilderInner,
}

impl Default for TaskBuilder {
    fn default() -> Self {
        let inner = TaskBuilderInner {
            scheduled_at: None,
            max_attempts: Some(3),
            attempted_by: vec![],
            tags: vec![],
            kind: None,
            args: None,
            priority: Some(100),
            queue: None,
            depends_on: None,
            dependend_id: None,
        };

        TaskBuilder { inner }
    }
}

impl TaskBuilder {
    pub fn task<T: TaskKind + Serialize>(mut self, task: T) -> Result<Self, serde_json::Error> {
        self.set_task(task)?;
        Ok(self)
    }

    pub fn set_task<T: TaskKind + Serialize>(&mut self, task: T) -> Result<(), serde_json::Error> {
        let kind = T::kind();
        let args = serde_json::to_value(task)?;

        self.inner.kind = Some(kind);
        self.inner.args = Some(args);

        Ok(())
    }

    pub fn kind(mut self, kind: &str) -> Self {
        self.inner.kind = Some(kind.into());
        self
    }

    pub fn args(mut self, args: serde_json::Value) -> Self {
        self.inner.args = Some(args);
        self
    }

    pub fn depends_on(mut self, dependend_id: &Uuid) -> Self {
        self.set_depends_on(dependend_id);
        self
    }

    pub fn set_depends_on(&mut self, dependend_id: &Uuid) {
        self.inner.depends_on = Some(*dependend_id);
    }

    pub fn queue(mut self, queue: &str) -> Self {
        self.set_queue(queue);
        self
    }

    pub fn set_queue(&mut self, queue: &str) {
        self.inner.queue = Some((*queue).to_string());
    }

    pub fn priority(mut self, priority: i16) -> Self {
        self.set_priority(priority);
        self
    }

    pub fn set_priority(&mut self, priority: i16) {
        self.inner.priority = Some(priority);
    }

    pub fn dependend_id(mut self, dependend_id: &Uuid) -> Self {
        self.set_dependend_id(dependend_id);
        self
    }

    pub fn set_dependend_id(&mut self, dependend_id: &Uuid) {
        self.inner.dependend_id = Some(*dependend_id);
    }

    pub fn scheduled_at(mut self, scheduled_at: &DateTime<Utc>) -> Self {
        self.set_scheduled_at(scheduled_at);
        self
    }

    pub fn set_scheduled_at(&mut self, scheduled_at: &DateTime<Utc>) {
        self.inner.scheduled_at = Some(*scheduled_at);
    }

    pub fn build(self) -> Result<NewTask, TaskBuildError> {
        let inner = self.inner;
        let kind = inner.kind.ok_or(TaskBuildError::KindMissing)?;
        let args = inner.args.ok_or(TaskBuildError::ArgsMissing)?;

        let task = NewTask {
            scheduled_at: inner.scheduled_at,
            max_attempts: inner.max_attempts.unwrap_or(3),
            priority: inner.priority.unwrap_or(3),
            attempted_by: inner.attempted_by,
            tags: inner.tags,
            queue: inner.queue,
            kind,
            args,
            depends_on: inner.depends_on,
            dependend_id: inner.dependend_id,
        };

        Ok(task)
    }
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

    pub async fn batch_insert(
        db: impl PgExecutor<'_>,
        tasks: &[NewTask],
    ) -> crate::error::Result<Vec<Uuid>> {
        let data = serde_json::to_value(&tasks)?;
        let rows = sqlx::query_file!("src/db/tasks/sql/batch_insert.sql", data)
            .fetch_all(db)
            .await?;

        let ids = rows.into_iter().map(|row| row.id).collect::<Vec<Uuid>>();
        Ok(ids)
    }

    pub async fn get_tasks_by_kind(
        db: impl PgExecutor<'_>,
        kind: &str,
        queue: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<Task>> {
        let rows = sqlx::query_file_as!(
            Task,
            "src/db/tasks/sql/get_tasks_by_kind.sql",
            kind,
            queue,
            limit
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get_tasks(
        db: impl PgExecutor<'_>,
        queue: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<Task>> {
        let rows = sqlx::query_file_as!(Task, "src/db/tasks/sql/get_tasks.sql", queue, limit)
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    pub async fn get_priority_tasks(
        db: impl PgExecutor<'_>,
        queue: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<Task>> {
        let rows = sqlx::query_file_as!(
            Task,
            "src/db/tasks/sql/get_priority_tasks.sql",
            queue,
            limit
        )
        .fetch_all(db)
        .await?;

        Ok(rows)
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

    pub async fn get_task(db: impl PgExecutor<'_>, task_id: &Uuid) -> sqlx::Result<Option<Task>> {
        sqlx::query_file_as!(Task, "src/db/tasks/sql/get_task.sql", task_id)
            .fetch_optional(db)
            .await
    }

    pub async fn get_all(db: impl PgExecutor<'_>, ids: &Vec<Uuid>) -> sqlx::Result<Vec<Task>> {
        sqlx::query_file_as!(Task, "src/db/tasks/sql/get_all.sql", ids)
            .fetch_all(db)
            .await
    }

    pub async fn failed(db: impl PgExecutor<'_>, task_id: &Uuid, error: &str) -> sqlx::Result<()> {
        sqlx::query_file!("src/db/tasks/sql/failed.sql", task_id, error)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn set_state(
        db: impl PgExecutor<'_>,
        task_id: &Uuid,
        state: &TaskState,
    ) -> sqlx::Result<()> {
        sqlx::query_file!(
            "src/db/tasks/sql/set_state.sql",
            task_id,
            state as &TaskState
        )
        .execute(db)
        .await?;

        Ok(())
    }
}
