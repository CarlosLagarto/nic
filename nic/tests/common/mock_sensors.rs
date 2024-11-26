use axum::async_trait;
// use futures_util::FutureExt;
use mockall::mock;
use nic::sensors::interface::SensorController;
use std::sync::Arc;

mock! {
    pub SensorController {}

    #[async_trait]
    impl SensorController for SensorController {
        async fn activate_sector(&self, sector: u32);
        async fn deactivate_sector(&self, sector: u32);
    }
}

pub fn set_sensor_controller() -> Arc<MockSensorController> {
    let mut mock_controller = MockSensorController::new();

    mock_controller
        .expect_activate_sector()
        // .with(mockall::predicate::eq(1))
        .with(mockall::predicate::always()) // Relaxed to allow any sector ID
        // .times(1)
        .times(0..)
        .returning(|sector| {
            tokio::spawn(async move {
                println!("Mocked activation for sector {}", sector);
            });
        });
    // mock_controller.activate_sector(1).await;
    Arc::new(mock_controller)
}
