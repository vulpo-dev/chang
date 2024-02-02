pub enum SchedulingStrategy {
    FCFS,
    Priority,
}

pub struct TaskQueue {
    pub strategy: SchedulingStrategy,
    pub name: String,
    pub limit: i64,
    pub interval: u64,
}

impl TaskQueue {
    pub fn builder() -> TaskQueueBuilder {
        TaskQueueBuilder::default()
    }
}

pub struct TaskQueueBuilderInner {
    strategy: Option<SchedulingStrategy>,
    name: Option<String>,
    limit: i64,
    interval: u64,
}

impl Default for TaskQueueBuilderInner {
    fn default() -> Self {
        TaskQueueBuilderInner {
            strategy: Some(SchedulingStrategy::FCFS),
            name: Some("default".into()),
            limit: 10,
            interval: 500,
        }
    }
}

pub struct TaskQueueBuilder {
    inner: TaskQueueBuilderInner,
}

impl Default for TaskQueueBuilder {
    fn default() -> Self {
        let inner = TaskQueueBuilderInner::default();
        TaskQueueBuilder { inner }
    }
}

impl TaskQueueBuilder {
    pub fn strategy(mut self, strategy: SchedulingStrategy) -> Self {
        self.inner.strategy = Some(strategy);
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.inner.name = Some(name.into());
        self
    }

    pub fn limit(mut self, limit: i64) -> Self {
        self.inner.limit = limit;
        self
    }

    pub fn interval(mut self, interval: u64) -> Self {
        self.inner.interval = interval;
        self
    }

    pub fn build(self) -> TaskQueue {
        TaskQueue {
            strategy: self.inner.strategy.unwrap_or(SchedulingStrategy::FCFS),
            name: self.inner.name.unwrap_or(String::from("name")),
            limit: self.inner.limit,
            interval: self.inner.interval,
        }
    }
}
