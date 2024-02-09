use chrono::{DateTime, Timelike, Utc};
use cron;
use log::info;
use std::iter::Iterator;
use std::str::FromStr;
use std::time::Duration;

use crate::task::Task;
use crate::utils::context::Context;

use crate::task::{
    periodic_tasks::PeriodicJobs, Db, FromTaskContext, NewTask, TaskKind, TaskService, TaskState,
};

pub struct ChangSchedulePeriodicTask;

impl TaskKind for ChangSchedulePeriodicTask {
    fn kind() -> String {
        String::from("chang_schedule_periodic_task")
    }
}

pub async fn schedule_periodic_task(ctx: Context) -> anyhow::Result<TaskState> {
    info!("schedule periodic task");
    let db = Db::from_context(&ctx)?;
    let task = Task::from_context(&ctx)?;
    let periodic_jobs = PeriodicJobs::from_context(&ctx)?;

    let queue = task.queue.unwrap_or("default".to_string());
    let schedule = get_schedule(Utc::now());
    let tasks = get_scheduled_tasks(
        schedule,
        &periodic_jobs.entries().collect::<Vec<(&String, &String)>>(),
        &queue,
    );

    info!("insert {} tasks into {} queue", tasks.len(), queue);

    if !tasks.is_empty() {
        TaskService::batch_insert(&*db, &tasks).await?;
    }

    let scheduled_at = get_next_periodic_task_schedule(Utc::now());

    info!(
        "schedule chang_schedule_periodic_task at {}",
        scheduled_at.to_rfc3339()
    );
    Task::builder()
        .kind(&ChangSchedulePeriodicTask::kind())
        .args(serde_json::Value::Null)
        .scheduled_at(&scheduled_at)
        .priority(std::i16::MAX)
        .build()?
        .insert(&*db)
        .await?;

    Ok(TaskState::Completed)
}

fn get_next_periodic_task_schedule(now: DateTime<Utc>) -> DateTime<Utc> {
    let schedule_in = (60 - now.minute()) * 60;
    now + Duration::from_secs(schedule_in.into()) - Duration::from_secs(now.second().into())
}

#[derive(Debug, PartialEq)]
pub struct ScheduleSlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

pub fn get_schedule(start_time: DateTime<Utc>) -> ScheduleSlot {
    let schedule_in = 60 - start_time.minute();
    let start_next_hour = start_time + Duration::from_secs((schedule_in * 60).into());
    let end_next_hour = start_next_hour + Duration::from_secs(60 * 60);

    ScheduleSlot {
        start_time: start_next_hour,
        end_time: end_next_hour,
    }
}

pub fn get_scheduled_tasks(
    schedule_slot: ScheduleSlot,
    periodic_jobs: &[(&String, &String)],
    queue: &str,
) -> Vec<NewTask> {
    periodic_jobs
        .iter()
        .filter_map(|(kind, schedule)| {
            cron::Schedule::from_str(schedule).ok().map(|schedule| {
                schedule
                    .after(&schedule_slot.start_time)
                    .take_while(|schedule_at| schedule_at <= &schedule_slot.end_time)
                    .map(|schedule_at| {
                        Task::builder()
                            .kind(kind)
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

#[cfg(test)]
mod test {
    use super::*;

    use chrono::{DateTime, Utc};

    #[test]
    fn can_get_schedule() {
        let now = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let start_time = "2014-11-28T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let end_time = "2014-11-28T13:00:00Z".parse::<DateTime<Utc>>().unwrap();

        let expected = ScheduleSlot {
            start_time,
            end_time,
        };

        let schedule = get_schedule(now);

        assert_eq!(expected, schedule)
    }

    #[test]
    fn can_get_task_schedules() {
        let now = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let schedule = get_schedule(now);

        let tasks = get_scheduled_tasks(
            schedule,
            &[(&"every-five-minutes".into(), &"0 */5 * * * * *".into())],
            "default",
        );

        let mut expected: Vec<NewTask> = Vec::new();
        let start = now + Duration::from_secs(60 * 60);
        let five_minutes = Duration::from_secs(60 * 5);

        for i in 1..13 {
            expected.push(
                Task::builder()
                    .kind("every-five-minutes")
                    .args(serde_json::value::Value::Null)
                    .scheduled_at(&(start + (five_minutes * i)))
                    .queue("default")
                    .build()
                    .unwrap(),
            )
        }

        assert_eq!(expected.len(), tasks.len());
        assert_eq!(expected, tasks);
    }

    #[test]
    fn can_get_task_hourly_schedules() {
        let now = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let schedule = get_schedule(now);

        let tasks = get_scheduled_tasks(
            schedule,
            &[(&"hourly".into(), &"@hourly".into())],
            "default",
        );

        let scheduled_at = "2014-11-28T13:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let expected = Task::builder()
            .kind("hourly")
            .args(serde_json::value::Value::Null)
            .scheduled_at(&scheduled_at)
            .queue("default")
            .build()
            .unwrap();

        assert_eq!(expected, tasks.first().unwrap().clone());
    }

    #[test]
    fn can_get_task_every_30_minutes_schedules() {
        let now = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let schedule = get_schedule(now);

        let tasks = get_scheduled_tasks(
            schedule,
            &[(&"every-30-minutes".into(), &"0 */30 * * * * *".into())],
            "default",
        );

        let mut expected: Vec<NewTask> = Vec::new();
        let start = now + Duration::from_secs(60 * 60);
        let thirty_minutes = Duration::from_secs(60 * 30);

        for i in 1..3 {
            expected.push(
                Task::builder()
                    .kind("every-30-minutes")
                    .args(serde_json::value::Value::Null)
                    .scheduled_at(&(start + (thirty_minutes * i)))
                    .queue("default")
                    .build()
                    .unwrap(),
            )
        }

        assert_eq!(expected.len(), tasks.len());
        assert_eq!(expected, tasks);
    }

    #[test]
    fn can_get_multiple_tasks_schedules() {
        let now = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let schedule = get_schedule(now);

        let tasks = get_scheduled_tasks(
            schedule,
            &[
                (&"every-30-minutes".into(), &"0 */30 * * * * *".into()),
                (&"every-five-minutes".into(), &"0 */5 * * * * *".into()),
            ],
            "default",
        );

        let mut expected: Vec<NewTask> = Vec::new();
        let start = now + Duration::from_secs(60 * 60);
        let thirty_minutes = Duration::from_secs(60 * 30);

        for i in 1..3 {
            expected.push(
                Task::builder()
                    .kind("every-30-minutes")
                    .args(serde_json::value::Value::Null)
                    .scheduled_at(&(start + (thirty_minutes * i)))
                    .queue("default")
                    .build()
                    .unwrap(),
            )
        }

        let five_minutes = Duration::from_secs(60 * 5);

        for i in 1..13 {
            expected.push(
                Task::builder()
                    .kind("every-five-minutes")
                    .args(serde_json::value::Value::Null)
                    .scheduled_at(&(start + (five_minutes * i)))
                    .queue("default")
                    .build()
                    .unwrap(),
            )
        }

        assert_eq!(expected.len(), tasks.len());
        assert_eq!(expected, tasks);
    }

    #[test]
    fn can_get_next_periodic_task_schedule() {
        let expected = "2014-11-28T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let now = "2014-11-28T11:00:00Z".parse::<DateTime<Utc>>().unwrap();

        assert_eq!(expected, get_next_periodic_task_schedule(now));

        let now = "2014-11-28T11:05:05Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(expected, get_next_periodic_task_schedule(now));

        let now = "2014-11-28T11:59:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(expected, get_next_periodic_task_schedule(now));
    }
}
