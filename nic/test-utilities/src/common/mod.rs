pub mod mock_db;
pub mod mock_sector;
pub mod mock_sensors;

use mock_db::{new_with_mock, MockDatabase};
use mock_sensors::{set_sensor_controller0, set_sensor_controller1, MockSensorController};
use nic::watering::ds::AppState;
use nic::watering::state_machine::WateringStateMachine;
use std::sync::Arc;

pub fn setup_mock_state_machine() -> WateringStateMachine {
    WateringStateMachine::new()
}

pub async fn set_app_state() -> Arc<AppState<MockSensorController, MockDatabase>> {
    let db = Arc::new(MockDatabase::new());
    let controller = set_sensor_controller0();
    let app_state = new_with_mock(db, controller.clone()).await.unwrap();

    app_state
}

pub async fn set_app_state1() -> Arc<AppState<MockSensorController, MockDatabase>> {
    let db = Arc::new(MockDatabase::new());
    let controller = set_sensor_controller1();
    let app_state = new_with_mock(db, controller.clone()).await.unwrap();

    app_state
}

pub async fn set_app_state_and_controller(
) -> (Arc<AppState<MockSensorController, MockDatabase>>, Arc<MockSensorController>) {
    let db = Arc::new(MockDatabase::new());
    let controller = set_sensor_controller0();
    let app_state = new_with_mock(db, controller.clone()).await.unwrap();

    (app_state, controller)
}
