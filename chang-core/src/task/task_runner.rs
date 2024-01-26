use super::queue::{SchedulingStrategy, TaskQueue};
use super::FromTaskContext;
use crate::db::tasks::TaskKind;

use crate::db::tasks::{Task, TaskService, TaskState};
use crate::task::ChangSchedulePeriodicTask;
use crate::utils::context::{AnyClone, Context};

use chrono::{Timelike, Utc};
use futures::future;
use futures_util::future::SelectAll;
use futures_util::Future;
use log::{error, info};
use sqlx::PgPool;
use std::error::Error;
use std::ops::Deref;
use std::time::Duration;
use std::{
    collections::{hash_map, HashMap},
    sync::Arc,
};
use std::{fmt::Debug, pin::Pin};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct PeriodicJobs(HashMap<String, String>);

impl PeriodicJobs {
    pub fn entries(&self) -> hash_map::Iter<'_, String, String> {
        self.0.iter()
    }
}

impl FromTaskContext for PeriodicJobs {
    type Error = PeriodicJobsError;

    fn from_context(ctx: &Context) -> Result<Self, Self::Error> {
        ctx.get::<PeriodicJobs>()
            .ok_or(PeriodicJobsError::PeriodicJobsNotFound)
            .cloned()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PeriodicJobsError {
    #[error("PeriodicJobs not found in Context")]
    PeriodicJobsNotFound,
}

pub const DEFAULT_QUEUE: &str = "default";

pub struct TaskRunner<E: Into<Box<dyn Error + Send + Sync>> + 'static>
where
    E: std::fmt::Display + Debug,
{
    db: PgPool,
    routes: Arc<HashMap<String, Box<dyn TaskHandler<Context, E> + Send + Sync>>>,
    periodic_jobs: Arc<HashMap<String, String>>,
    context: Arc<Context>,
    queue: Arc<TaskQueue>,
    concurrency: i64,
    label: Arc<String>,
}

impl<E: Into<Box<dyn Error + Send + Sync>> + 'static + std::marker::Send> TaskRunner<E>
where
    E: std::fmt::Display + Debug,
{
    pub fn builder() -> TasksBuilder<E> {
        let default_queue = TaskQueue::builder()
            .name(DEFAULT_QUEUE)
            .strategy(SchedulingStrategy::FCFS)
            .build();

        let inner = TasksBuilderInner {
            routes: HashMap::new(),
            context: Context::new(),
            queue: default_queue,
            concurrency: 10,
            label: String::from("chang-tasks"),
            periodic_jobs: HashMap::new(),
        };

        TasksBuilder { inner }
    }

    pub fn start(&self) -> SelectAll<tokio::task::JoinHandle<()>> {
        let concurrency = self.concurrency;

        let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];
        let token = CancellationToken::new();

        for thread in 0..concurrency {
            let context = self.context.clone();
            let router = self.routes.clone();
            let task_pool = self.db.clone();
            let queue = self.queue.clone();
            let label = self.label.clone();
            let cancel_token = token.clone();

            let handle = tokio::spawn(async move {
                let thread_label = format!("{} - {}", label, thread);

                loop {
                    if cancel_token.is_cancelled() || task_pool.is_closed() {
                        break;
                    }

                    if task_pool.is_closed() {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    let get_tasks = match queue.strategy {
                        SchedulingStrategy::Priority => {
                            TaskService::get_priority_tasks(&task_pool, &queue.name, 1).await
                        }
                        SchedulingStrategy::FCFS => {
                            TaskService::get_tasks(&task_pool, &queue.name, 1).await
                        }
                    };

                    let tasks = match get_tasks {
                        Ok(tasks) => tasks,
                        Err(err) => {
                            error!(
                                "[{}] task error: failed to fetch tasks {:?}",
                                thread_label, err
                            );
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            continue;
                        }
                    };

                    if tasks.is_empty() {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    let mut futures: Vec<_> = vec![];

                    for task in tasks.into_iter() {
                        let fut = run_task::<E>(&task_pool, task, &router, &context, &thread_label);
                        futures.push(Box::pin(fut));
                    }

                    let _ = future::select_all(futures).await;
                }
            });

            handles.push(handle);
        }

        let periodic_jobs = self.periodic_jobs.clone();
        let queue = self.queue.clone();
        let db = self.db.clone();
        tokio::spawn(async move {
            // try insert chang_schedule_periodic_tasks task

            if periodic_jobs.is_empty() {
                return;
            }

            let kind = ChangSchedulePeriodicTask::kind();

            let current_task = match TaskService::get_task_by_kind(&db, &kind, &queue.name).await {
                Err(err) => {
                    error!("failed to fetch current task({}): {:?}", kind, err);
                    return;
                }

                Ok(ct) => ct,
            };

            if current_task.is_some() {
                return;
            }

            let now = Utc::now();
            let seconds = now.minute() * 60;
            let start_of_hour = now - Duration::from_secs(seconds.into());

            let task = Task::builder()
                .kind(&kind)
                .args(serde_json::Value::Null)
                .scheduled_at(&start_of_hour)
                .priority(std::i16::MAX)
                .queue(&queue.name)
                .build();

            let task = match task {
                Err(err) => {
                    error!("failed to build task({}): {:?}", kind, err);
                    return;
                }

                Ok(task) => task,
            };

            if let Err(err) = task.insert(&db).await {
                error!("failed to insert task({}): {:?}", kind, err);
            }
        });

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Chang Tasks Shutdown requested. Waiting for pending tasks");
            token.cancel();
        });

        future::select_all(handles)
    }
}

async fn run_task<E: Into<Box<dyn Error + Send + Sync>>>(
    task_pool: &PgPool,
    task: Task,
    router: &HashMap<String, Box<dyn TaskHandler<Context, E> + Send + Sync>>,
    context: &Context,
    label: &str,
) where
    E: std::fmt::Display + Debug,
{
    info!("[{}] run task({:?})", label, task.id);

    let Some(handler) = router.get(&task.kind) else {
        error!(
            "[{}] task error: handler not found for task {} with kind {}",
            label, task.id, task.kind
        );
        return;
    };

    let task_id = task.id;

    let mut ctx = Context::from(context);
    ctx.put(task);
    ctx.put(task_pool.clone());

    match handler.call(ctx).await {
        Err(err) => {
            let error = format!("{:?}", err);
            error!("[{}] task({}) failed to run: {:?}", label, task_id, error);
            if let Err(err) = TaskService::failed(task_pool, &task_id, &error).await {
                error!(
                    "[{}] Failed to set task state {:?} task {:?}",
                    label,
                    TaskState::Discarded,
                    err
                );
            };
        }
        Ok(state) => {
            let res = match state {
                TaskState::Completed => TaskService::complete(task_pool, &task_id).await,
                _ => TaskService::complete(task_pool, &task_id).await,
            };

            if let Err(err) = res {
                error!(
                    "[{}] Failed to set task state {:?} task {:?}",
                    label, state, err
                );
            };
        }
    };
}

pub struct TasksBuilderInner<E: Into<Box<dyn Error + Send + Sync>> + 'static>
where
    E: std::fmt::Display + Debug,
{
    routes: HashMap<String, Box<dyn TaskHandler<Context, E> + Send + Sync>>,
    context: Context,
    queue: TaskQueue,
    concurrency: i64,
    label: String,
    periodic_jobs: HashMap<String, String>,
}
pub struct TasksBuilder<E: Into<Box<dyn Error + Send + Sync>> + 'static>
where
    E: std::fmt::Display + Debug,
{
    inner: TasksBuilderInner<E>,
}

impl<E: Into<Box<dyn Error + Send + Sync>> + 'static> TasksBuilder<E>
where
    E: std::fmt::Display + Debug,
{
    pub fn register<K, H, Arg>(mut self, k: K, h: H) -> Self
    where
        K: Into<String>,
        H: TaskHandler<Arg, E> + Sync + 'static + Send,
    {
        let wrapper = move |ctx| Box::pin(h.call(ctx));
        self.inner.routes.insert(k.into(), Box::new(wrapper));
        self
    }

    pub fn register_periodic<S, K, H, Arg>(mut self, schedule: S, kind: K, handler: H) -> Self
    where
        S: Into<String>,
        K: Into<String> + Clone,
        H: TaskHandler<Arg, E> + Sync + 'static + Send,
    {
        self.inner
            .periodic_jobs
            .insert(schedule.into(), kind.clone().into());
        self.register(kind, handler)
    }

    pub fn add_context<Val>(mut self, value: Val) -> Self
    where
        Val: AnyClone + Send + Sync + Clone,
    {
        self.set_context(value);
        self
    }

    pub fn set_context<Val>(&mut self, value: Val)
    where
        Val: AnyClone + Send + Sync + Clone,
    {
        self.inner.context.put(value);
    }

    pub fn concurrency(mut self, concurrency: i64) -> Self {
        self.inner.concurrency = concurrency;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.inner.label = label.into();
        self
    }

    pub fn connect(mut self, db: &PgPool) -> TaskRunner<E> {
        self.set_context(db.clone());
        self.set_context(PeriodicJobs(self.inner.periodic_jobs.clone()));

        TaskRunner {
            routes: Arc::new(self.inner.routes),
            db: db.clone(),
            context: Arc::new(self.inner.context),
            queue: Arc::new(self.inner.queue),
            concurrency: self.inner.concurrency,
            label: Arc::new(self.inner.label),
            periodic_jobs: Arc::new(self.inner.periodic_jobs),
        }
    }
}

pub trait TaskHandler<Ctx, E> {
    fn call(&self, ctx: Context) -> Pin<Box<dyn Future<Output = Result<TaskState, E>> + Send>>;
}

impl<F: Sync + 'static, Ret, E> TaskHandler<Context, E> for F
where
    F: Fn(Context) -> Ret + Sync + 'static,
    Ret: Future<Output = Result<TaskState, E>> + Send + 'static,
    E: Into<Box<dyn Error + Send + Sync>>,
{
    fn call(&self, ctx: Context) -> Pin<Box<dyn Future<Output = Result<TaskState, E>> + Send>> {
        Box::pin(self(ctx))
    }
}

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
