use crate::watering::ds::{Cycle, SectorInfo, WateringEvent, WeatherConditions};
use async_trait::async_trait;
use chrono::Duration;
use rusqlite::{params, Connection, Result, ToSql};
use std::sync::mpsc::{self, Sender};
use std::thread;


#[async_trait]
pub trait DatabaseTrait: Send + Sync {
    fn execute(&self, query: &str, params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<usize>;
    fn execute_batch(&self, query: &str) -> Result<()>;
    fn query_row(
        &self,
        query: &str,
        params: Vec<Box<dyn rusqlite::ToSql + Send>>,
    ) -> Result<String>;
    fn load_sectors(&self) -> Result<Vec<SectorInfo>>;
    fn load_cycles(&self) -> Result<Vec<Cycle>>;
    fn log_watering_event(&self, evt: WateringEvent) -> Result<()>;
    fn get_current_weather(&self) -> Option<WeatherConditions>;
}

pub enum DatabaseCommand {
    Execute {
        query: String,
        params: Vec<Box<dyn rusqlite::ToSql + Send>>,
        response: Sender<Result<usize>>,
    },
    ExecuteBatch {
        query: String,
        response: Sender<Result<()>>,
    },
    QueryRow {
        query: String,
        params: Vec<Box<dyn rusqlite::ToSql + Send>>,
        response: Sender<Result<String>>, // Adjust based on your query result type
    },
    LoadSectors {
        response: Sender<Result<Vec<SectorInfo>>>,
    },
    LoadCycles {
        response: Sender<Result<Vec<Cycle>>>,
    },
    LogWateringEvent {
        evt: WateringEvent,
        response: Sender<Result<()>>,
    },
    GetCurrentWeather {
        response: Sender<Option<WeatherConditions>>,
    },
}

#[derive(Clone)]
pub struct Database {
    pub sender: Sender<DatabaseCommand>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let conn = Connection::open(path).unwrap();
        initialize(&conn)?;
        thread::spawn(move || {
            while let Ok(command) = rx.recv() {
                match command {
                    DatabaseCommand::Execute {
                        query,
                        params,
                        response,
                    } => {
                        let params: Vec<&dyn ToSql> =
                            params.iter().map(|p| p.as_ref() as &dyn ToSql).collect();
                        let result = conn.execute(&query, params.as_slice());
                        let _ = response.send(result);
                    }
                    DatabaseCommand::ExecuteBatch { query, response } => {
                        let result = conn.execute_batch(&query);
                        let _ = response.send(result);
                    }
                    DatabaseCommand::QueryRow {
                        query,
                        params,
                        response,
                    } => {
                        let params: Vec<&dyn ToSql> =
                            params.iter().map(|p| p.as_ref() as &dyn ToSql).collect();
                        let result: Result<String> =
                            conn.query_row(&query, params.as_slice(), |row| row.get(0));
                        let _ = response.send(result);
                    }
                    DatabaseCommand::LoadSectors { response } => {
                        let res = load_sectors(&conn);
                        let _ = response.send(res);
                    }
                    DatabaseCommand::LoadCycles { response } => {
                        let res = load_cycles(&conn);
                        let _ = response.send(res);
                    }
                    DatabaseCommand::LogWateringEvent { evt, response } => {
                        let res = log_watering_event(&conn, evt);
                        let _ = response.send(res);
                    }
                    DatabaseCommand::GetCurrentWeather { response } => {
                        let res = get_current_weather();
                        let _ = response.send(res);
                    }
                }
            }
        });

        Ok(Self { sender: tx })
    }
}

#[async_trait]
impl DatabaseTrait for Database {
    fn execute(&self, query: &str, params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<usize> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::Execute {
                query: query.to_string(),
                params,
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }

    fn execute_batch(&self, query: &str) -> Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::ExecuteBatch {
                query: query.to_string(),
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }

    fn query_row(
        &self,
        query: &str,
        params: Vec<Box<dyn rusqlite::ToSql + Send>>,
    ) -> Result<String> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::QueryRow {
                query: query.to_string(),
                params,
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }

    fn load_sectors(&self) -> Result<Vec<SectorInfo>> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::LoadSectors {
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }

    fn load_cycles(&self) -> Result<Vec<Cycle>> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::LoadCycles {
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }

    fn log_watering_event(&self, evt: WateringEvent) -> Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::LogWateringEvent {
                evt,
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }

    fn get_current_weather(&self) -> Option<WeatherConditions> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::GetCurrentWeather {
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }
}

pub fn initialize(conn: &Connection) -> Result<()> {
    let query = "
        CREATE TABLE IF NOT EXISTS sectors (
            id INTEGER PRIMARY KEY,
            sprinkler_debit REAL NOT NULL,
            percolation_rate REAL NOT NULL,
            max_duration INTEGER NOT NULL,
            weekly_target REAL NOT NULL,
            progress REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cycles (
            id INTEGER NOT NULL,
            sector_id INTEGER NOT NULL,
            duration INTEGER NOT NULL,
            PRIMARY KEY (id, sector_id),
            FOREIGN KEY (sector_id) REFERENCES sectors(id)
        );

        CREATE TABLE IF NOT EXISTS watering_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cycle_id INTEGER,
            sector_id INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            duration INTEGER NOT NULL,
            water_applied REAL NOT NULL,
            type TEXT NOT NULL,
            FOREIGN KEY (sector_id) REFERENCES sectors(id)
        );
        ";

    conn.execute_batch(query)?;
    Ok(())
}

pub fn load_sectors(conn: &Connection) -> Result<Vec<SectorInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, sprinkler_debit, percolation_rate, max_duration, weekly_target, progress FROM sectors",
    )?;
    let sectors = stmt
        .query_map([], |row| {
            Ok(SectorInfo {
                id: row.get(0)?,
                sprinkler_debit: row.get(1)?,
                percolation_rate: row.get(2)?,
                max_duration: Duration::minutes(row.get::<_, i64>(3)?),
                weekly_target: row.get(4)?,
                progress: row.get(5)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(sectors)
}

pub fn load_cycles(conn: &Connection) -> Result<Vec<Cycle>> {
    let mut stmt =
        conn.prepare("SELECT id, sector_id, duration FROM cycles ORDER BY id, sector_id")?;
    let mut cycles_map: std::collections::HashMap<u32, Vec<(u32, Duration)>> =
        std::collections::HashMap::new();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            Duration::minutes(row.get::<_, i64>(2)?),
        ))
    })?;

    for row in rows {
        let (cycle_id, sector_id, duration) = row?;
        cycles_map
            .entry(cycle_id)
            .or_default()
            .push((sector_id, duration));
    }

    Ok(cycles_map
        .into_iter()
        .map(|(id, instructions)| Cycle { id, instructions })
        .collect())
}

pub fn log_watering_event(conn: &Connection, evt: WateringEvent) -> Result<()> {
    conn.execute(
        "INSERT INTO watering_events (cycle_id, sector_id, start_time, duration, water_applied, type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            evt.cycle_id,
            evt.sector_id,
            evt.start_time,
            evt.duration.num_minutes(),
            evt.water_applied,
            evt.event_type.to_string()
        ],
    )?;
    Ok(())
}

pub fn get_current_weather() -> Option<WeatherConditions> {
    // TODO:
    // Simulate retrieving weather conditions
    // Replace with actual database or API query
    Some(WeatherConditions {
        is_raining: false, // Example: No rain
        wind_speed: 15.0,  // Example: Wind speed is 15 km/h
    })
}
