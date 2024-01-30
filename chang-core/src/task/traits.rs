use crate::db::tasks::Task;
use crate::utils::context::{Context, CurrentTask};

use crate::db::tasks::TaskState;
use futures_util::Future;
use log::error;
use std::error::Error;
use std::fmt::Debug;
use std::pin::Pin;

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
