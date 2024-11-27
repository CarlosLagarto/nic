use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nic::db::DatabaseCommand;
use nic::db::DatabaseTrait;
use nic::sensors::interface::SensorController;
use nic::watering::ds::WateringEvent;
use nic::watering::ds::{AppState, Cycle, SectorInfo, WeatherConditions};
use nic::watering::watering_system::WateringSystem;
use rusqlite::Result;
use nic::error::AppError;


pub async fn new_with_mock<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    db: Arc<D>,
    controler: Arc<C>,
) -> Result<Arc<AppState<C, D>>, AppError> {
    let watering_system = WateringSystem::new(controler, db.clone()).await?;
    Ok(Arc::new(AppState {
        db,
        watering_system,
    }))
}

#[derive(Clone)]
pub struct MockDatabase {
    pub sender: Sender<DatabaseCommand>,
    pub data: Arc<Mutex<HashMap<String, String>>>, // Simulates database storage
}

impl MockDatabase {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let data = Arc::new(Mutex::new(HashMap::new()));

        // Simulate the background thread processing commands
        let data_clone = Arc::clone(&data);
        std::thread::spawn(move || {
            while let Ok(command) = rx.recv() {
                match command {
                    DatabaseCommand::Execute {
                        query, response, ..
                    } => {
                        println!("Mock execute: {}", query);
                        let _ = response.send(Ok(1)); // Simulate a successful execution
                    }
                    DatabaseCommand::ExecuteBatch { query, response } => {
                        println!("Mock execute batch: {}", query);
                        let _ = response.send(Ok(())); // Simulate a successful batch execution
                    }
                    DatabaseCommand::QueryRow {
                        query, response, ..
                    } => {
                        println!("Mock query row: {}", query);
                        let result = data_clone.lock().unwrap().get(&query).cloned();
                        let _ = response
                            .send(result.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows));
                    }
                    DatabaseCommand::LoadSectors { response } => {
                        println!("Mock load sectors");
                        let sectors = vec![SectorInfo {
                            id: 1,
                            weekly_target: 2.5,
                            sprinkler_debit: 1.0,
                            max_duration: chrono::Duration::minutes(30),
                            percolation_rate: 0.5,
                            progress: 0.,
                        }];
                        let _ = response.send(Ok(sectors));
                    }
                    DatabaseCommand::LoadCycles { response } => {
                        println!("Mock load cycles");
                        let cycles = vec![Cycle {
                            id: 1,
                            instructions: vec![(1, chrono::Duration::minutes(30))],
                        }];
                        let _ = response.send(Ok(cycles));
                    }
                    DatabaseCommand::LogWateringEvent { evt, response } => {
                        println!("Mock log watering event: {:?}", evt);
                        let _ = response.send(Ok(())); // Simulate successful logging
                    }
                    DatabaseCommand::GetCurrentWeather { response } => {
                        println!("Mock get current weather");
                        let weather = WeatherConditions {
                            is_raining: false,
                            wind_speed: 10.0,
                        };
                        let _ = response.send(Some(weather));
                    }
                }
            }
        });

        MockDatabase { sender: tx, data }
    }
}

#[async_trait]
impl DatabaseTrait for MockDatabase {
    fn execute(
        &self,
        _query: &str,
        _params: Vec<Box<dyn rusqlite::ToSql + Send>>,
    ) -> Result<usize> {
        Ok(1) // Simulate success
    }

    fn execute_batch(&self, _query: &str) -> Result<()> {
        Ok(()) // Simulate success
    }

    fn query_row(
        &self,
        query: &str,
        _params: Vec<Box<dyn rusqlite::ToSql + Send>>,
    ) -> Result<String> {
        self.data
            .lock()
            .unwrap()
            .get(&query.to_owned())
            .cloned()
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    fn load_sectors(&self) -> Result<Vec<SectorInfo>> {
        Ok(vec![SectorInfo {
            id: 1,
            weekly_target: 2.5,
            sprinkler_debit: 1.0,
            max_duration: chrono::Duration::minutes(30),
            percolation_rate: 0.5,
            progress: 0.,
        }])
    }

    fn load_cycles(&self) -> Result<Vec<Cycle>> {
        Ok(vec![Cycle {
            id: 1,
            instructions: vec![(1, chrono::Duration::minutes(30))],
        }])
    }

    fn log_watering_event(&self, _evt: WateringEvent) -> Result<()> {
        Ok(()) // Simulate success
    }

    fn get_current_weather(&self) -> Option<WeatherConditions> {
        Some(WeatherConditions {
            is_raining: false,
            wind_speed: 10.0,
        })
    }
}
