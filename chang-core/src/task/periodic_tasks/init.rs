use chrono::{DateTime, Timelike, Utc};
use log::info;
use sqlx::PgPool;
use std::{collections::HashMap, time::Duration};

use crate::task::{ChangSchedulePeriodicTask, Task, TaskBuildError, TaskKind, TaskService};

pub type PeriodicJobs = HashMap<String, String>;

#[derive(thiserror::Error, Debug)]
pub enum InitPeriodicJobsError {
    #[error("failed to fetch current task({kind:?}): {queue:?}")]
    GetCurrentTask {
        kind: String,
        queue: String,
        #[source]
        error: sqlx::Error,
    },

    #[error("failed to build task({kind:?}): {queue:?}")]
    BuildTask {
        kind: String,
        queue: String,
        #[source]
        error: TaskBuildError,
    },

    #[error("failed to insert task({kind:?}): {queue:?}")]
    InsertTask {
        kind: String,
        queue: String,
        #[source]
        error: sqlx::Error,
    },
}

pub async fn init(
    periodic_jobs: &PeriodicJobs,
    db: &PgPool,
    queue: &str,
    now: &DateTime<Utc>,
) -> Result<(), InitPeriodicJobsError> {
    info!("Init {}", ChangSchedulePeriodicTask::kind());

    if periodic_jobs.is_empty() {
        return Ok(());
    }

    let kind = ChangSchedulePeriodicTask::kind();

    let current_task = TaskService::get_tasks_by_kind(db, &kind, &queue, 1)
        .await
        .map_err(|error| InitPeriodicJobsError::GetCurrentTask {
            kind: kind.clone(),
            queue: queue.to_string(),
            error,
        })?;

    if current_task.first().is_some() {
        info!("{} exists, abort insert", ChangSchedulePeriodicTask::kind());
        return Ok(());
    }

    let seconds = now.minute() * 60;
    let start_of_hour = *now - Duration::from_secs(seconds.into());

    let task = Task::builder()
        .kind(&kind)
        .args(serde_json::Value::Null)
        .scheduled_at(&start_of_hour)
        .priority(std::i16::MAX)
        .queue(&queue)
        .build()
        .map_err(|error| InitPeriodicJobsError::BuildTask {
            kind: kind.clone(),
            queue: queue.to_string(),
            error,
        })?;

    task.insert(db)
        .await
        .map_err(|error| InitPeriodicJobsError::InsertTask {
            kind,
            queue: queue.to_string(),
            error,
        })?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::migration;
    use crate::task::TaskState;
    use crate::utils;

    #[tokio::test]
    async fn can_insert_task() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::tasks(&prepare.pool).await;

        let expected_scheduled_at = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let now = "2014-11-28T11:23:00Z".parse::<DateTime<Utc>>().unwrap();
        let mut periodic_jobs: PeriodicJobs = HashMap::new();
        periodic_jobs.insert("0 */5 * * * * *".into(), "test_can_insert_task".into());

        let result = init(&periodic_jobs, &prepare.pool, &prepare.name, &now).await;
        assert!(result.is_ok(), "failed to insert task");

        let tasks = TaskService::get_tasks_by_kind(
            &prepare.pool,
            &ChangSchedulePeriodicTask::kind(),
            &prepare.name,
            1,
        )
        .await
        .unwrap();

        assert!(tasks.first().is_some(), "failed to fetch task");

        let task = tasks.first().unwrap();

        assert_eq!(task.state, TaskState::Available);
        assert_eq!(task.queue, Some(prepare.name.clone()));
        assert_eq!(task.scheduled_at, Some(expected_scheduled_at));

        utils::test::cleanup(prepare).await;
    }

    #[tokio::test]
    async fn wont_insert_task_when_periodic_jobs_are_empty() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::tasks(&prepare.pool).await;

        let now = "2014-11-28T11:23:00Z".parse::<DateTime<Utc>>().unwrap();
        let periodic_jobs: PeriodicJobs = HashMap::new();

        let result = init(&periodic_jobs, &prepare.pool, &prepare.name, &now).await;
        assert!(result.is_ok(), "failed to insert task");

        let tasks = TaskService::get_tasks_by_kind(
            &prepare.pool,
            &ChangSchedulePeriodicTask::kind(),
            &prepare.name,
            1,
        )
        .await
        .unwrap();

        assert!(
            tasks.first().is_none(),
            "inserted \"chang_schedule_periodic_task\""
        );

        utils::test::cleanup(prepare).await;
    }

    #[tokio::test]
    async fn wont_insert_task_exists() {
        let prepare = utils::test::prepare().await;

        migration::base(&prepare.pool).await;
        migration::tasks(&prepare.pool).await;

        let expected_scheduled_at = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let now = "2014-11-28T11:23:00Z".parse::<DateTime<Utc>>().unwrap();
        let periodic_jobs: PeriodicJobs = HashMap::new();

        Task::builder()
            .kind(&ChangSchedulePeriodicTask::kind())
            .args(serde_json::Value::Null)
            .scheduled_at(&"2014-11-28T10:00:00Z".parse::<DateTime<Utc>>().unwrap())
            .queue(&prepare.name)
            .build()
            .unwrap()
            .insert(&prepare.pool)
            .await
            .unwrap();

        let result = init(&periodic_jobs, &prepare.pool, &prepare.name, &now).await;
        assert!(result.is_ok(), "failed to insert task");

        let tasks = TaskService::get_tasks_by_kind(
            &prepare.pool,
            &ChangSchedulePeriodicTask::kind(),
            &prepare.name,
            1,
        )
        .await
        .unwrap();

        assert!(tasks.first().is_some(), "failed to fetch task");

        let task = tasks.first().unwrap();
        assert_ne!(task.scheduled_at, Some(expected_scheduled_at));

        utils::test::cleanup(prepare).await;
    }
}
