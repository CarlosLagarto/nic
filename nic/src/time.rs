use async_trait::async_trait;
use std::{any::Any, fmt::Debug, time::Duration};

#[async_trait]
pub trait TimeProvider: Send + Sync + Debug {
    fn now(&self) -> i64; // Returns the current time as a Unix UTC timestamp
    fn as_any(&self) -> &dyn Any;
    async fn sleep(&self, duration: Duration);
    async fn advance_time(&self, seconds: i64);
    fn set(&self, new_time: i64);
}

#[derive(Debug)]
pub struct RealTimeProvider;

#[async_trait]
impl TimeProvider for RealTimeProvider {
    fn now(&self) -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    async fn advance_time(&self, _seconds: i64) {
        self.sleep(Duration::from_secs(1)).await;
    }

    fn set(&self, _new_time: i64) {}
}
