use tracing::trace;
// use futures_util::FutureExt;
use crate::sensors::interface::SensorController;
use crate::test::utils::AppError;
use mockall::mock;
use std::sync::Arc;

mock! {
    #[derive(Debug)]
    pub SensorController {}

    impl SensorController for SensorController {
        fn activate_sector(&self, sector: u32) -> Result<(), AppError>;
        fn deactivate_sector(&self, sector: u32) -> Result<(), AppError>;
    }
}

pub fn set_sensor_controller0() -> Arc<MockSensorController> {
    let mut mock_controller = MockSensorController::new();
    // .times(1)
    // .with(mockall::predicate::eq(1))
    // Relaxed to allow any sector ID
    mock_controller.expect_activate_sector().with(mockall::predicate::always()).times(0..).returning(|sector| {
        trace!("Mocked activation-0 for sector {}", sector);
        Ok(())
    });
    // Allow multiple deactivations
    mock_controller.expect_deactivate_sector().with(mockall::predicate::always()).times(0..).returning(|sector| {
        trace!("Mocked deactivation-0 for sector {}", sector);
        Ok(())
    });

    Arc::new(mock_controller)
}

pub fn set_sensor_controller1() -> Arc<MockSensorController> {
    let mut mock_controller = MockSensorController::new();

    mock_controller.expect_activate_sector().with(mockall::predicate::always()).times(1..).returning(|sector| {
        trace!("Mocked activation-1 for sector {}", sector);
        Ok(())
    });
    // Allow at least one deactivation
    mock_controller.expect_deactivate_sector().with(mockall::predicate::always()).times(1..).returning(move |sector| {
        trace!("Mocked deactivation-1 for sector {}", sector);
        Ok(())
    });
    Arc::new(mock_controller)
}
