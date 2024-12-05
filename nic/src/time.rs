use async_trait::async_trait;
use std::{any::Any, sync::Arc, time::Duration};

use crate::test::utils::mock_time::MockTimeProvider;

#[async_trait]
pub trait TimeProvider: Send + Sync {
    fn now(&self) -> i64; // Returns the current time as a Unix UTC timestamp
    fn as_any(&self) -> &dyn Any;
    async fn sleep(&self, duration: Duration);
}

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

}

pub async fn advance_time<T: TimeProvider>(time_provider: &Arc<T>) {
    if let Some(mock_time) = time_provider.as_any().downcast_ref::<MockTimeProvider>() {
        mock_time.advance_time(1);
    } else {
        time_provider.sleep(Duration::from_secs(1)).await;
    }
}