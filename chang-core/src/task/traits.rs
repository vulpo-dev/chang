use crate::db::tasks::Task;
use crate::utils::context::{Context, CurrentTask};

use log::error;
use std::error::Error;
use std::fmt::Debug;

pub trait FromTaskContext {
    type Error: Into<Box<dyn Error + Send + Sync>>;
    fn from_context(ctx: &Context) -> Result<Self, Self::Error>
    where
        Self: Sized,
        Self::Error: Into<Box<dyn Error + Send + Sync>>;
}

#[derive(thiserror::Error, Debug)]
pub enum TaskError {
    #[error("task not found")]
    NotFound,
}

impl FromTaskContext for Task {
    type Error = TaskError;

    fn from_context(ctx: &Context) -> Result<Self, Self::Error> {
        let task = ctx.get::<Task>().ok_or(TaskError::NotFound)?;
        Ok(task.clone())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CurrentTaskError {
    #[error("current task not found")]
    NotFound,

    #[error(transparent)]
    Task(#[from] TaskError),
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
        let current_task = Task::from_context(ctx)?;
        Ok(CurrentTask(current_task.args.clone()))
    }
}
