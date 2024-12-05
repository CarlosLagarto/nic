use crate::time::TimeProvider;
use async_trait::async_trait;
use chrono::TimeZone;
use tracing_subscriber::fmt::time::FormatTime;
use std::{
    any::Any,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};

pub struct MockTimeProvider {
    current_time: Arc<AtomicI64>,
}

impl MockTimeProvider {
    pub fn new(start_time: i64) -> Self {
        Self { current_time: Arc::new(AtomicI64::new(start_time)) }
    }

    pub fn advance_time(&self, seconds: i64) {
        self.current_time.fetch_add(seconds, Ordering::SeqCst);
    }

    pub fn set(&self, new_time: i64) {
        self.current_time.store(new_time, Ordering::SeqCst)
    }
}

#[async_trait]
impl TimeProvider for MockTimeProvider {
    fn now(&self) -> i64 {
        self.current_time.load(Ordering::SeqCst)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn sleep(&self, _duration: Duration) {}
}

pub struct MockTimeFormatter<T: TimeProvider> {
    pub time_provider: Arc<T>,
}

impl<T: TimeProvider> FormatTime for MockTimeFormatter<T> {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let mock_time = self.time_provider.now();
        let time = chrono::Utc.timestamp_opt(mock_time, 0).unwrap(); // Convert to `chrono::DateTime`
        write!(w, "{}", time.to_rfc3339())
    }
}

