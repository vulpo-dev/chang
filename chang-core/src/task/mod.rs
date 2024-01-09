mod context;

pub use context::{AnyClone, Context, CurrentTask};
use futures::future;

use crate::db::tasks::{NewTask, Task as DbTask, TaskService};
use chrono::{DateTime, Utc};
use futures_util::Future;
use serde::Serialize;
use sqlx::{PgExecutor, PgPool};
use std::error::Error;
use std::ops::Deref;
use std::{collections::HashMap, sync::Arc};
use std::{fmt::Debug, pin::Pin};
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct Task {
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

impl Task {
    pub fn builder() -> TaskBuilder {
        TaskBuilder::default()
    }

    pub async fn insert(self, db: impl PgExecutor<'_>) -> sqlx::Result<Uuid> {
        let new_task = NewTask {
            scheduled_at: self.scheduled_at,
            max_attempts: self.max_attempts,
            attempted_by: self.attempted_by,
            tags: self.tags,
            kind: self.kind,
            args: self.args,
            priority: self.priority,
            queue: self.queue,
            depends_on: self.depends_on,
            dependend_id: self.dependend_id,
        };

        Box::pin(TaskService::insert(db, new_task)).await
    }
}

pub async fn enqueu_task(db: impl PgExecutor<'_>, task: Task) -> sqlx::Result<Uuid> {
    let new_task = NewTask {
        scheduled_at: task.scheduled_at,
        max_attempts: task.max_attempts,
        attempted_by: task.attempted_by,
        tags: task.tags,
        kind: task.kind,
        args: task.args,
        priority: task.priority,
        queue: task.queue,
        depends_on: task.depends_on,
        dependend_id: task.dependend_id,
    };

    TaskService::insert(db, new_task).await
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
            priority: Some(1),
            queue: None,
            depends_on: None,
            dependend_id: None,
        };

        TaskBuilder { inner }
    }
}

impl TaskBuilder {
    pub fn task<T: TaskKind + Serialize>(mut self, task: T) -> Result<Self, serde_json::Error> {
        let kind = T::kind();
        let args = serde_json::to_value(task)?;

        self.inner.kind = Some(kind);
        self.inner.args = Some(args);

        Ok(self)
    }

    pub fn depends_on(mut self, dependend_id: &Uuid) -> Self {
        self.inner.depends_on = Some(dependend_id.clone());
        self
    }

    pub fn dependend_id(mut self, dependend_id: &Uuid) -> Self {
        self.inner.dependend_id = Some(dependend_id.clone());
        self
    }

    pub fn scheduled_at(mut self, scheduled_at: &DateTime<Utc>) -> Self {
        self.inner.scheduled_at = Some(scheduled_at.clone());
        self
    }

    pub fn build(self) -> Result<Task, TaskBuildError> {
        let inner = self.inner;
        let kind = inner.kind.ok_or(TaskBuildError::KindMissing)?;
        let args = inner.args.ok_or(TaskBuildError::ArgsMissing)?;

        let task = Task {
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

pub trait FromTaskContext {
    type Error: std::error::Error;
    fn from_context(ctx: &Context) -> Result<Self, Self::Error>
    where
        Self: Sized,
        Self::Error: std::error::Error;
}

#[derive(thiserror::Error, Debug)]
pub enum DbTaskError {
    #[error("task not found")]
    NotFound,
}

impl FromTaskContext for DbTask {
    type Error = DbTaskError;

    fn from_context(ctx: &Context) -> Result<Self, Self::Error> {
        let task = ctx.get::<DbTask>().ok_or(DbTaskError::NotFound)?;
        Ok(task.clone())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CurrentTaskError {
    #[error("current task not found")]
    NotFound,

    #[error(transparent)]
    Task(#[from] DbTaskError),
}

#[derive(thiserror::Error, Debug)]
pub enum TaskContextError {
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),

    #[error(transparent)]
    CurrentTask(#[from] CurrentTaskError),
}

impl FromTaskContext for CurrentTask {
    type Error = CurrentTaskError;

    fn from_context(ctx: &Context) -> Result<Self, Self::Error> {
        let current_task = DbTask::from_context(ctx)?;
        Ok(CurrentTask(current_task.args.clone()))
    }
}

pub trait TaskKind {
    fn kind() -> String
    where
        Self: Sized;
}

pub struct Tasks<E: Into<Box<dyn Error + Send + Sync>> + 'static>
where
    E: std::fmt::Display + Debug,
{
    db: PgPool,
    routes: Arc<RwLock<HashMap<String, Box<dyn TaskRunner<Context, E> + Send + Sync>>>>,
    context: Arc<RwLock<Context>>,
}

impl<E: Into<Box<dyn Error + Send + Sync>> + 'static + std::marker::Send> Tasks<E>
where
    E: std::fmt::Display + Debug,
{
    pub fn router() -> TasksBuilder<E> {
        TasksBuilder {
            routes: HashMap::new(),
            context: Context::new(),
        }
    }

    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let context = self.context.clone();
        let router = self.routes.clone();
        let task_pool = self.db.clone();

        tokio::spawn(async move {
            loop {
                let tasks = match TaskService::get_tasks(&task_pool).await {
                    Ok(tasks) => tasks,
                    Err(err) => {
                        println!("task error: failed to fetch tasks {:?}", err);
                        continue;
                    }
                };

                println!("fetched {} tasks", tasks.len());

                if tasks.len() == 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }

                let mut futures: Vec<_> = vec![];

                for task_id in tasks.into_iter() {
                    let fut = run_task::<E>(&task_pool, task_id, &router, &context);
                    futures.push(Box::pin(fut));
                }

                let _ = future::select_all(futures).await;
            }
        })
    }
}

async fn run_task<E: Into<Box<dyn Error + Send + Sync>>>(
    task_pool: &PgPool,
    task_id: Uuid,
    router: &RwLock<HashMap<String, Box<dyn TaskRunner<Context, E> + Send + Sync>>>,
    context: &RwLock<Context>,
) -> ()
where
    E: std::fmt::Display + Debug,
{
    let task = match TaskService::get_sheduled_task(task_pool, &task_id).await {
        Ok(task) => task,
        Err(err) => {
            println!("task error: failed to fetch task({}) {:?}", task_id, err);
            return ();
        }
    };

    let Some(task) = task else {
        println!("task error: not found: {}", task_id);
        return ();
    };

    let routes = router.read().await;
    let Some(handler) = routes.get(&task.kind) else {
        println!("task error: handler not found: {}", task_id);
        return ();
    };

    let context = context.read().await;

    let mut ctx = Context::from(context.deref());
    ctx.put(task);
    ctx.put(task_pool.clone());

    match handler.call(ctx).await {
        Err(err) => {
            println!("task({}) failed: {:?}", task_id, err);
            let error = format!("{:?}", err);
            TaskService::failed(task_pool, &task_id, &error)
                .await
                .unwrap();
        }
        Ok(_) => {
            if let Err(err) = TaskService::complete(task_pool, &task_id).await {
                println!("Failed to complete task {:?}", err);
            };
        }
    };

    ()
}

pub struct TasksBuilder<E: Into<Box<dyn Error + Send + Sync>> + 'static>
where
    E: std::fmt::Display + Debug,
{
    routes: HashMap<String, Box<dyn TaskRunner<Context, E> + Send + Sync>>,
    context: Context,
}

impl<E: Into<Box<dyn Error + Send + Sync>> + 'static> TasksBuilder<E>
where
    E: std::fmt::Display + Debug,
{
    pub fn register<K, H, TFut>(mut self, k: K, h: H) -> Self
    where
        K: Into<String>,
        H: Fn(Context) -> TFut + Send + Sync + 'static,
        TFut: Future<Output = Result<(), E>> + Send + 'static,
    {
        let wrapper = move |req| Box::pin(h(req));
        self.routes.insert(k.into(), Box::new(wrapper));
        self
    }

    pub fn add_context<Val>(mut self, value: Val) -> Self
    where
        Val: AnyClone + Send + Sync + Clone,
    {
        self.context.put(value);
        self
    }

    pub fn connect(self, db: PgPool) -> Tasks<E> {
        Tasks {
            routes: Arc::new(self.routes.into()),
            db,
            context: Arc::new(self.context.into()),
        }
    }
}

trait TaskRunner<Ctx, E> {
    fn call(&self, ctx: Ctx) -> Pin<Box<dyn Future<Output = Result<(), E>> + Send>>;
}

impl<F: Sync + 'static, Ret, Ctx, E> TaskRunner<Ctx, E> for F
where
    F: Fn(Ctx) -> Ret + Sync + 'static,
    Ret: Future<Output = Result<(), E>> + Send + 'static,
    E: Into<Box<dyn Error + Send + Sync>>,
{
    fn call(&self, req: Ctx) -> Pin<Box<dyn Future<Output = Result<(), E>> + Send>> {
        Box::pin(self(req))
    }
}
