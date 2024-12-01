use async_trait::async_trait;
use nic::db::{DatabaseCommand, DatabaseTrait};
use nic::error::AppError;
use nic::sensors::interface::SensorController;
use nic::utils::sod;
use nic::watering::ds::{AppState, Cycle, SectorInfo, WateringEvent, WeatherConditions};
use nic::watering::schedule::{Schedule, ScheduleEntry, ScheduleType};
use nic::watering::watering_system::WateringSystem;
use rusqlite::Result;
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

pub async fn new_with_mock<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    db: Arc<D>, controler: Arc<C>,
) -> Result<Arc<AppState<C, D>>, AppError> {
    let watering_system = WateringSystem::new(controler, db.clone()).await?;
    Ok(Arc::new(AppState { db, watering_system }))
}

#[derive(Clone)]
pub struct MockDatabase {
    pub sender: Sender<DatabaseCommand>,
    pub data: Arc<Mutex<HashMap<String, String>>>, // Simulates database storage
    pub et_data: HashMap<i64, f64>,
    pub rain_data: HashMap<i64, f64>,
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
                        let sectors = mock_sector();
                        let _ = response.send(Ok(sectors));
                    }
                    DatabaseCommand::LoadCycles { response } => {
                        println!("Mock load cycles");
                        let cycles = vec![Cycle { id: 1, instructions: vec![(1, 30 * 3600)] }];
                        let _ = response.send(Ok(cycles));
                    }
                    DatabaseCommand::LogWateringEvent { evt, response } => {
                        println!("Mock log watering event: {:?}", evt);
                        let _ = response.send(Ok(())); // Simulate successful logging
                    }
                    DatabaseCommand::GetCurrentWeather { response } => {
                        println!("Mock get current weather");
                        let weather = mock_weather();
                        let _ = response.send(Some(weather));
                    }
                    DatabaseCommand::GetLastdayRain { response, .. } => {
                        println!("Mock get last day rain");
                        let _ = response.send(Some(1.));
                    }
                    DatabaseCommand::GetLastdayET { response, .. } => {
                        println!("Mock get last day rain");
                        let _ = response.send(Some(1.));
                    }
                    DatabaseCommand::LoadAutoSchedule { response, .. } => {
                        println!("Mock load auto schedule");
                        let entries = mock_auto_schedule();
                        let _ = response.send(Ok(Schedule::new(entries)));
                    }
                }
            }
        });

        MockDatabase { sender: tx, data, et_data: HashMap::new(), rain_data: HashMap::new() }
    }
}

fn mock_sector() -> Vec<SectorInfo> {
    let sectors = vec![SectorInfo {
        id: 1,
        weekly_target: 2.5,
        sprinkler_debit: 1.0,
        max_duration: 30 * 3600,
        percolation_rate: 0.5,
        progress: 0.,
    }];
    sectors
}

fn mock_weather() -> WeatherConditions {
    WeatherConditions { is_raining: false, wind_speed: 10.0, humidity: 20., solar_radiation: 1., temperature: 15. }
}

fn mock_auto_schedule() -> Vec<ScheduleEntry> {
    let entries = vec![
        ScheduleEntry {
            schedule_type: ScheduleType::Weekday(chrono::Weekday::Mon),
            start_times: vec![(1, 6 * 3600, 30 * 60), (2, 7 * 3600, 20 * 60)],
        },
        ScheduleEntry {
            schedule_type: ScheduleType::Weekday(chrono::Weekday::Wed),
            start_times: vec![(3, 8 * 3600, 40 * 60)],
        },
        ScheduleEntry {
            schedule_type: ScheduleType::Weekday(chrono::Weekday::Fri),
            start_times: vec![(4, 9 * 3600, 50 * 60)],
        },
    ];
    entries
}

#[async_trait]
impl DatabaseTrait for MockDatabase {
    fn execute(&self, _query: &str, _params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<usize> {
        Ok(1) // Simulate success
    }

    fn execute_batch(&self, _query: &str) -> Result<()> {
        Ok(()) // Simulate success
    }

    fn query_row(&self, query: &str, _params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<String> {
        self.data.lock().unwrap().get(&query.to_owned()).cloned().ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    fn load_sectors(&self) -> Result<Vec<SectorInfo>> {
        Ok(mock_sector())
    }

    fn load_cycles(&self) -> Result<Vec<Cycle>> {
        Ok(vec![Cycle { id: 1, instructions: vec![(1, 30 * 3600)] }])
    }

    fn log_watering_event(&self, _evt: WateringEvent) -> Result<()> {
        Ok(()) // Simulate success
    }

    fn get_current_weather(&self) -> Option<WeatherConditions> {
        Some(mock_weather())
    }

    fn get_lastday_rain(&self, timestamp: i64) -> Option<f64> {
        self.rain_data.get(&sod(timestamp)).cloned()
    }

    fn get_daily_et(&self, timestamp: i64) -> Option<f64> {
        self.et_data.get(&sod(timestamp)).cloned()
    }

    fn load_auto_schedule(&self) -> Result<Schedule, rusqlite::Error> {
        Ok(Schedule::new(mock_auto_schedule()))
    }
}