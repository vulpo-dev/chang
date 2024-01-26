use chrono::{Timelike, Utc};
use cron;
use std::iter::Iterator;
use std::str::FromStr;
use std::time::Duration;

use crate::task::Task;
use crate::utils::context::Context;

use super::{
    task_runner::PeriodicJobs, Db, FromTaskContext, NewTask, TaskKind, TaskService, TaskState,
};

pub struct ChangSchedulePeriodicTask;

impl TaskKind for ChangSchedulePeriodicTask {
    fn kind() -> String {
        String::from("chang_schedule_periodic_task")
    }
}

pub async fn schedule_periodic_task(ctx: Context) -> anyhow::Result<TaskState> {
    let db = Db::from_context(&ctx)?;
    let task = Task::from_context(&ctx)?;
    let periodic_jobs = PeriodicJobs::from_context(&ctx)?;

    let queue = task.queue.unwrap_or("default".to_string());
    let tasks = get_scheduled_tasks(
        &periodic_jobs.entries().collect::<Vec<(&String, &String)>>(),
        &queue,
    );

    if tasks.is_empty() {
        return Ok(TaskState::Completed);
    }

    TaskService::batch_insert(&*db, tasks).await?;

    let now = Utc::now();
    let schedule_in = (60 - now.minute()) * 60;
    let scheduled_at = now + Duration::from_secs(schedule_in.into());

    Task::builder()
        .kind(&ChangSchedulePeriodicTask::kind())
        .args(serde_json::Value::Null)
        .scheduled_at(&scheduled_at)
        .priority(std::i16::MAX)
        .build()?
        .insert(&*db)
        .await?;

    return Ok(TaskState::Completed);
}

pub fn get_scheduled_tasks(periodic_jobs: &[(&String, &String)], queue: &str) -> Vec<NewTask> {
    let hour = Duration::from_secs(60 * 60);
    let start_time = Utc::now() + hour;
    let schedule_in = 60 - start_time.minute();
    let start_next_hour = start_time + Duration::from_secs((schedule_in * 60).into());
    let end_next_hour = start_next_hour + Duration::from_secs(60 * 60);

    periodic_jobs
        .into_iter()
        .filter_map(|(kind, schedule)| {
            cron::Schedule::from_str(&schedule).ok().map(|schedule| {
                schedule
                    .after(&start_next_hour)
                    .take_while(|schedule_at| schedule_at < &end_next_hour)
                    .map(|schedule_at| {
                        Task::builder()
                            .kind(&kind)
                            .args(serde_json::value::Value::Null)
                            .scheduled_at(&schedule_at)
                            .queue(queue)
                            .build()
                            .unwrap()
                    })
                    .collect::<Vec<NewTask>>()
            })
        })
        .flatten()
        .collect::<Vec<NewTask>>()
}
