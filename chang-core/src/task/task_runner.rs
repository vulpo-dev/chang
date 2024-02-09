use super::periodic_tasks::PeriodicJobs;
use super::queue::{SchedulingStrategy, TaskQueue};
use super::run_task::TaskRouter;
use super::{task_loop, FromTaskContext};

use crate::task::periodic_tasks;
use crate::task::traits::TaskHandler;
use crate::utils::context::{AnyClone, Context};

use chrono::Utc;
use futures::future;
use futures_util::future::SelectAll;
use log::{error, info};
use sqlx::PgPool;
use std::error::Error;
use std::fmt::Debug;
use std::ops::Deref;

use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio_util::sync::CancellationToken;

pub const DEFAULT_QUEUE: &str = "default";

pub struct TaskRunner<E: Into<Box<dyn Error + Send + Sync>> + 'static>
where
    E: std::fmt::Display + Debug,
{
    db: PgPool,
    routes: Arc<TaskRouter<E>>,
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
            let periodic_jobs = self.periodic_jobs.clone();

            let handle = tokio::spawn(async move {
                let thread_label = format!("{} {} queue({})", thread, label, queue.name);

                task_loop::start(
                    &thread_label,
                    &cancel_token,
                    &task_pool,
                    &queue,
                    &router,
                    &context,
                    &periodic_jobs,
                )
                .await;
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

            let now = Utc::now() - Duration::from_secs(3600);
            if let Err(error) = periodic_tasks::init(&periodic_jobs, &db, &queue.name, &now).await {
                error!("{:?}", error);
            };
        });

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Chang Tasks Shutdown requested. Waiting for pending tasks");
            token.cancel();
        });

        future::select_all(handles)
    }
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
            .insert(kind.clone().into(), schedule.into());
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

    pub fn queue(mut self, queue: TaskQueue) -> Self {
        self.inner.queue = queue;
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
