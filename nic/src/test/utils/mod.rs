pub mod mock_db;
pub mod mock_sector;
pub mod mock_sensors;
pub mod mock_time;

use crate::{
    error::AppError,
    utils::init_channels,
    watering::{ds::AppState, modes::ModeIdx, watering_system::WateringSystem},
};
use mock_db::{new_with_mock, MockDatabase};
use mock_sensors::{set_sensor_controller0, set_sensor_controller1, MockSensorController};
use mock_time::MockTimeProvider;
use std::sync::Arc;

pub type MockAppState = Arc<AppState<MockSensorController, MockDatabase, MockTimeProvider>>;

pub fn set_app_state(start_time: i64) -> MockAppState {
    let db = Arc::new(MockDatabase::new());
    let controller = set_sensor_controller0();
    let time_provider = Arc::new(MockTimeProvider::new(start_time));
    new_with_mock(db, controller.clone(), time_provider).unwrap()
}

pub fn set_app_and_ws(start_time: i64, starting_mode: Option<ModeIdx>,) -> Result<(MockAppState,WateringSystem<MockSensorController, MockDatabase, MockTimeProvider>), AppError> {
    let db = Arc::new(MockDatabase::new());
    let controller = set_sensor_controller0();
    let time_provider = Arc::new(MockTimeProvider::new(start_time));
    let app_state= new_with_mock(db.clone(), controller.clone(), time_provider.clone()).unwrap();
    let (tx, rx) = init_channels();
    Ok((app_state,WateringSystem::new(controller.clone(), db, time_provider, starting_mode, tx, rx)?))
}

pub fn set_app_state1(start_time: i64) -> MockAppState {
    let db = Arc::new(MockDatabase::new());
    let time_provider = Arc::new(MockTimeProvider::new(start_time));
    let controller = set_sensor_controller1();

    new_with_mock(db, controller.clone(), time_provider).unwrap()
}

pub fn set_ws1(
    start_time: i64, starting_mode: Option<ModeIdx>,
) -> Result<WateringSystem<MockSensorController, MockDatabase, MockTimeProvider>, AppError> {
    let db = Arc::new(MockDatabase::new());
    let time_provider = Arc::new(MockTimeProvider::new(start_time));
    let controller = set_sensor_controller1();
    let (tx, rx) = init_channels();
    WateringSystem::new(controller.clone(), db.clone(), time_provider.clone(), starting_mode, tx, rx)
}

pub fn set_ws0(
    start_time: i64, starting_mode: Option<ModeIdx>, db: Option<Arc<MockDatabase>>,
) -> Result<WateringSystem<MockSensorController, MockDatabase, MockTimeProvider>, AppError> {
    let db = if let Some(db) = db { db } else { Arc::new(MockDatabase::new()) };
    let time_provider = Arc::new(MockTimeProvider::new(start_time));
    let controller = set_sensor_controller0();
    let (tx, rx) = init_channels();
    WateringSystem::new(controller.clone(), db.clone(), time_provider.clone(), starting_mode, tx, rx)
}

pub async fn set_app_state_and_controller(start_time: i64, db: Option<Arc<MockDatabase>>,) -> (MockAppState, Arc<MockSensorController>) {
    let db = if let Some(db) = db { db } else { Arc::new(MockDatabase::new()) };
    let controller = set_sensor_controller0();
    let time_provider = Arc::new(MockTimeProvider::new(start_time));
    let app_state = new_with_mock(db, controller.clone(), time_provider).unwrap();

    (app_state, controller)
}
