use crate::db::tasks::{Task, TaskService, TaskState};
use crate::task::periodic_tasks::PeriodicJobs;
use crate::task::TaskHandler;
use crate::utils::context::Context;

use chrono::Utc;
use log::{error, info};
use sqlx::PgPool;
use std::fmt::Debug;
use std::{collections::HashMap, error::Error};

pub type TaskRouter<E> = HashMap<String, Box<dyn TaskHandler<Context, E> + Send + Sync>>;

pub async fn run_task<E: Into<Box<dyn Error + Send + Sync>>>(
    task_pool: &PgPool,
    task: Task,
    router: &TaskRouter<E>,
    context: &Context,
    label: &str,
    periodic_jobs: &PeriodicJobs,
) where
    E: std::fmt::Display + Debug,
{
    info!("[{}] run task({}) with id({:?})", label, task.kind, task.id);

    let Some(handler) = router.get(&task.kind) else {
        let error = format!(
            "handler not found for task {} with kind {}",
            task.id, task.kind
        );
        error!("[{}] task error: {}", label, error);

        if let Err(err) = TaskService::failed(task_pool, &task.id, &error).await {
            error!(
                "[{}] Failed to set task state {:?} task {:?}",
                label,
                TaskState::Retryable,
                err
            );
        };
        return;
    };

    let task_id = task.id;
    let task_kind = task.kind.clone();

    let mut ctx = Context::from(context);
    ctx.put(task);
    ctx.put(task_pool.clone());
    ctx.put(periodic_jobs.clone());

    let start = Utc::now();
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
            let end = Utc::now();
            let total = end - start;
            info!(
                "[{}] task({}) with id({:?}) completed in {}ms",
                label,
                task_kind,
                task_id,
                total.num_milliseconds()
            );
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

#[cfg(test)]
mod test {
    use anyhow::anyhow;
    use serde::Serialize;

    use super::*;
    use crate::db::migration;
    use crate::task::periodic_tasks::PeriodicJobs;
    use crate::task::{Db, FromTaskContext, Task, TaskKind};
    use crate::utils;

    #[tokio::test]
    async fn can_run_task() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::tasks(&prepare.pool).await;

        let mut router: TaskRouter<_> = HashMap::new();
        let periodic_jobs = PeriodicJobs(HashMap::new());

        router.insert(
            SimpleTask::kind(),
            Box::new(|ctx: Context| async move {
                let _task = Task::from_context(&ctx).ok().unwrap();
                let _db = Db::from_context(&ctx).ok().unwrap();
                Ok(TaskState::Completed)
            }),
        );

        let task = insert_task(&prepare.pool).await.unwrap();
        let context = &Context::new();

        run_task::<anyhow::Error>(
            &prepare.pool,
            task.clone(),
            &router,
            &context,
            &prepare.name,
            &periodic_jobs,
        )
        .await;

        let task = TaskService::get_task(&prepare.pool, &task.id)
            .await
            .unwrap();

        let state = task.map(|t| t.state);

        assert_eq!(Some(TaskState::Completed), state);

        utils::test::cleanup(prepare).await;
    }

    #[tokio::test]
    async fn discards_tasks_after_too_many_attempts() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::tasks(&prepare.pool).await;

        let mut router: TaskRouter<_> = HashMap::new();
        let periodic_jobs = PeriodicJobs(HashMap::new());

        router.insert(
            SimpleTask::kind(),
            Box::new(|_ctx: Context| async { Err(anyhow!("Test Failed")) }),
        );

        let task = insert_task(&prepare.pool).await.unwrap();
        let context = &Context::new();

        sqlx::query!(
            "
            update chang.tasks
               set attempt = 3
            where id = $1 
        ",
            task.id
        )
        .execute(&prepare.pool)
        .await
        .unwrap();

        run_task::<anyhow::Error>(
            &prepare.pool,
            task.clone(),
            &router,
            &context,
            &prepare.name,
            &periodic_jobs,
        )
        .await;

        let task = TaskService::get_task(&prepare.pool, &task.id)
            .await
            .unwrap();

        let state = task.map(|t| t.state);

        assert_eq!(Some(TaskState::Discarded), state);

        utils::test::cleanup(prepare).await;
    }

    #[tokio::test]
    async fn failes_when_handler_returns_error() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::tasks(&prepare.pool).await;

        let mut router: TaskRouter<_> = HashMap::new();

        router.insert(
            SimpleTask::kind(),
            Box::new(|_ctx: Context| async { Err(anyhow!("Test Failed")) }),
        );

        let task = insert_task(&prepare.pool).await.unwrap();
        let context = &Context::new();
        let periodic_jobs = PeriodicJobs(HashMap::new());

        run_task::<anyhow::Error>(
            &prepare.pool,
            task.clone(),
            &router,
            &context,
            &prepare.name,
            &periodic_jobs,
        )
        .await;

        let task = TaskService::get_task(&prepare.pool, &task.id)
            .await
            .unwrap();

        let state = task.map(|t| t.state);

        assert_eq!(Some(TaskState::Retryable), state);

        utils::test::cleanup(prepare).await;
    }

    #[tokio::test]
    async fn failes_when_handler_does_not_exist() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::tasks(&prepare.pool).await;

        let router: TaskRouter<_> = HashMap::new();

        let task = insert_task(&prepare.pool).await.unwrap();
        let context = &Context::new();
        let periodic_jobs = PeriodicJobs(HashMap::new());

        run_task::<anyhow::Error>(
            &prepare.pool,
            task.clone(),
            &router,
            &context,
            &prepare.name,
            &periodic_jobs,
        )
        .await;

        let task = TaskService::get_task(&prepare.pool, &task.id)
            .await
            .unwrap();

        let state = task.map(|t| t.state);

        assert_eq!(Some(TaskState::Retryable), state);

        utils::test::cleanup(prepare).await;
    }

    #[derive(Serialize)]
    struct SimpleTask {
        value: String,
    }

    impl TaskKind for SimpleTask {
        fn kind() -> String {
            String::from("simple_task")
        }
    }

    async fn insert_task(db: &PgPool) -> anyhow::Result<Task> {
        let task = SimpleTask {
            value: "Chang".to_string(),
        };

        let id = Task::builder().task(task)?.build()?.insert(db).await?;

        let task = TaskService::get_task(db, &id).await?;
        Ok(task.unwrap())
    }
}
