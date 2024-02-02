use std::time::Duration;

use futures::future;
use log::error;
use sqlx::PgPool;
use std::error::Error;
use std::fmt::Debug;
use tokio::time;
use tokio::{self, select};
use tokio_util::sync::CancellationToken;

use crate::task::{
    run_task::{run_task, TaskRouter},
    SchedulingStrategy, TaskQueue, TaskService,
};
use crate::utils::context::Context;

pub async fn start<E: Into<Box<dyn Error + Send + Sync>>>(
    label: &str,
    cancel_token: &CancellationToken,
    db: &PgPool,
    queue: &TaskQueue,
    router: &TaskRouter<E>,
    context: &Context,
) where
    E: std::fmt::Display + Debug,
{
    let mut interval = time::interval(Duration::from_millis(queue.interval));

    loop {
        if cancel_token.is_cancelled() || db.is_closed() {
            break;
        }

        let get_tasks = match queue.strategy {
            SchedulingStrategy::Priority => {
                TaskService::get_priority_tasks(db, &queue.name, queue.limit).await
            }
            SchedulingStrategy::FCFS => TaskService::get_tasks(db, &queue.name, queue.limit).await,
        };

        let tasks = match get_tasks {
            Ok(tasks) => tasks,
            Err(err) => {
                error!("[{}] task error: failed to fetch tasks {:?}", label, err);
                interval.tick().await;
                continue;
            }
        };

        if tasks.is_empty() {
            interval.tick().await;
            continue;
        }

        let mut futures: Vec<_> = vec![];

        for task in tasks.into_iter() {
            let fut = run_task::<E>(&db, task, &router, &context, &label);
            futures.push(Box::pin(fut));
        }

        let _ = future::select_all(futures).await;

        select! {
            _ = cancel_token.cancelled() => {
                break;
            }

            _ = db.close_event() => {
                break;
            }

            _ = interval.tick() => {
                continue;
            }
        }
    }
}
