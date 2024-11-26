use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender};

use crate::watering::ds::{Cycle, SectorInfo, WeatherConditions};

use super::DatabaseCommand;

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
                    DatabaseCommand::Execute { query, response, .. } => {
                        println!("Mock execute: {}", query);
                        let _ = response.send(Ok(1)); // Simulate a successful execution
                    }
                    DatabaseCommand::ExecuteBatch { query, response } => {
                        println!("Mock execute batch: {}", query);
                        let _ = response.send(Ok(())); // Simulate a successful batch execution
                    }
                    DatabaseCommand::QueryRow { query, response, .. } => {
                        println!("Mock query row: {}", query);
                        let result = data_clone.lock().unwrap().get(&query).cloned();
                        let _ = response.send(result.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows));
                    }
                    DatabaseCommand::LoadSectors { response } => {
                        println!("Mock load sectors");
                        let sectors = vec![SectorInfo {
                            id: 1,
                            weekly_target: 2.5,
                            sprinkler_debit: 1.0,
                            max_duration: chrono::Duration::minutes(30),
                            percolation_rate: 0.5,
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