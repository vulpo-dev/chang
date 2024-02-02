mod init;
pub mod schedule;

pub use init::init;

use super::FromTaskContext;
use crate::utils::context::Context;
use std::collections::{hash_map, HashMap};

#[derive(Clone)]
pub struct PeriodicJobs(pub HashMap<String, String>);

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
