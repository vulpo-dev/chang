use super::queue::{SchedulingStrategy, TaskQueue};
use super::FromTaskContext;
use crate::db::tasks::{Task, TaskService, TaskState};
use crate::task::NewTask;
use crate::utils::context::{AnyClone, Context};

use chrono::{Timelike, Utc};
use futures::future;
use futures_util::future::SelectAll;
use futures_util::Future;
use log::{error, info};
use sqlx::PgPool;
use std::error::Error;
use std::ops::Deref;
use std::str::FromStr;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use std::{fmt::Debug, pin::Pin};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct PeriodicJob(HashMap<String, String>);

impl FromTaskContext for PeriodicJob {
    type Error = PeriodicJobError;

    fn from_context(ctx: &Context) -> Result<Self, Self::Error> {
        ctx.get::<PeriodicJob>()
            .ok_or(PeriodicJobError::PeriodicJobNotFound)
            .cloned()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PeriodicJobError {
    #[error("PeriodicJob not found in Context")]
    PeriodicJobNotFound,
}

pub const DEFAULT_QUEUE: &'static str = "default";

pub struct TaskRunner<E: Into<Box<dyn Error + Send + Sync>> + 'static>
where
    E: std::fmt::Display + Debug,
{
    db: PgPool,
    routes: Arc<RwLock<HashMap<String, Box<dyn TaskHandler<Context, E> + Send + Sync>>>>,
    periodic_jobs: Arc<RwLock<HashMap<String, String>>>,
    context: Arc<RwLock<Context>>,
    queue: Arc<TaskQueue>,
    concurrency: i64,
    label: Arc<String>,
}

impl<E: Into<Box<dyn Error + Send + Sync>> + 'static + std::marker::Send> TaskRunner<E>
where
    E: std::fmt::Display + Debug,
{
    pub fn new() -> TasksBuilder<E> {
        let default_queue = TaskQueue::new()
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

                    if tasks.len() == 0 {
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
        tokio::spawn(async move {
            let _periodic_jobs = periodic_jobs;
            // todo: upsert periodic jobs
            // todo: delete unused jobs
        });

        let pool = self.db.clone();
        let queue = self.queue.clone();
        let cancel_token = token.clone();
        tokio::spawn(async move {
            loop {
                let mut tx = pool.begin().await.unwrap();
                let periodic_jobs =
                    match TaskService::get_periodic_tasks(&mut *tx, &queue.name).await {
                        Err(err) => {
                            error!("Failed to get periodic tasks {:?}", err);
                            tx.rollback().await.unwrap();
                            let schedule_in = 60 - Utc::now().minute();

                            let sleep =
                                tokio::time::sleep(Duration::from_secs((schedule_in * 60).into()));

                            tokio::select! {
                                _ = sleep => {
                                    continue;
                                }

                                _ = cancel_token.cancelled() => {
                                    break;
                                }
                            }
                        }

                        Ok(value) => value,
                    };

                let hour = Duration::from_secs(60 * 60);
                let start_time = Utc::now() + hour;
                let schedule_in = 60 - start_time.minute();
                let start_next_hour = start_time + Duration::from_secs((schedule_in * 60).into());
                let end_next_hour = start_next_hour + Duration::from_secs(60 * 60);

                let tasks = periodic_jobs
                    .into_iter()
                    .filter_map(|job| {
                        cron::Schedule::from_str(&job.schedule)
                            .ok()
                            .map(|schedule| {
                                schedule
                                    .after(&start_next_hour)
                                    .take_while(|schedule_at| schedule_at < &end_next_hour)
                                    .map(|schedule_at| {
                                        Task::builder()
                                            .kind(&job.kind)
                                            .args(serde_json::value::Value::Null)
                                            .scheduled_at(&schedule_at)
                                            .build()
                                            .unwrap()
                                    })
                                    .collect::<Vec<NewTask>>()
                            })
                    })
                    .flatten()
                    .collect::<Vec<NewTask>>();

                TaskService::batch_insert(&mut *tx, tasks).await.unwrap();

                tx.commit().await.unwrap();

                let schedule_in = 60 - Utc::now().minute();
                let sleep = tokio::time::sleep(Duration::from_secs((schedule_in * 60).into()));

                tokio::select! {
                    _ = sleep => {
                        continue;
                    }

                    _ = cancel_token.cancelled() => {
                        break;
                    }
                }
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
    router: &RwLock<HashMap<String, Box<dyn TaskHandler<Context, E> + Send + Sync>>>,
    context: &RwLock<Context>,
    label: &str,
) -> ()
where
    E: std::fmt::Display + Debug,
{
    info!("[{}] run task({:?})", label, task.id);

    let routes = router.read().await;
    let Some(handler) = routes.get(&task.kind) else {
        error!(
            "[{}] task error: handler not found for task {} with kind {}",
            label, task.id, task.kind
        );
        return ();
    };

    let task_id = task.id.clone();
    let context = context.read().await;

    let mut ctx = Context::from(context.deref());
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

    ()
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
        self.set_context(PeriodicJob(self.inner.periodic_jobs.clone()));

        TaskRunner {
            routes: Arc::new(self.inner.routes.into()),
            db: db.clone(),
            context: Arc::new(self.inner.context.into()),
            queue: Arc::new(self.inner.queue),
            concurrency: self.inner.concurrency,
            label: Arc::new(self.inner.label),
            periodic_jobs: Arc::new(self.inner.periodic_jobs.into()),
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
