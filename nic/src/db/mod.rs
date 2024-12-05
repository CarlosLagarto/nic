use crate::utils::display_from_ts;
use crate::watering::ds::{Cycle, DailyPlan, SectorInfo, WaterSector, WateringEvent, WeatherConditions, WeeklyPlan};
use crate::watering::schedule::{Schedule, ScheduleEntry, ScheduleType};
use async_trait::async_trait;
use chrono::Weekday;
use num_traits::FromPrimitive;
use rusqlite::{params, Connection, Result, ToSql};
use std::sync::mpsc::{self, Sender};
use std::thread;

#[async_trait]
pub trait DatabaseTrait: Send + Sync {
    fn execute(&self, query: &str, params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<usize>;
    fn execute_batch(&self, query: &str) -> Result<()>;
    fn query_row(&self, query: &str, params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<String>;
    fn load_sectors(&self) -> Result<Vec<SectorInfo>>;
    fn load_cycles(&self) -> Result<Vec<Cycle>>;
    fn log_watering_event(&self, evt: WateringEvent) -> Result<()>;
    fn get_current_weather(&self) -> Option<WeatherConditions>;
    fn get_lastday_rain(&self, timestamp: i64) -> Option<f64>;
    fn get_daily_et(&self, timestamp: i64) -> Option<f64>;
    fn load_auto_schedule(&self) -> Result<Schedule>;
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
    GetLastdayRain {
        time: i64,
        response: Sender<Option<f64>>,
    },
    GetLastdayET {
        time: i64,
        response: Sender<Option<f64>>,
    },
    LoadAutoSchedule {
        response: Sender<Result<Schedule>>,
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
                    DatabaseCommand::Execute { query, params, response } => {
                        let params: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref() as &dyn ToSql).collect();
                        let result = conn.execute(&query, params.as_slice());
                        let _ = response.send(result);
                    }
                    DatabaseCommand::ExecuteBatch { query, response } => {
                        let result = conn.execute_batch(&query);
                        let _ = response.send(result);
                    }
                    DatabaseCommand::QueryRow { query, params, response } => {
                        let params: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref() as &dyn ToSql).collect();
                        let result: Result<String> = conn.query_row(&query, params.as_slice(), |row| row.get(0));
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
                    DatabaseCommand::GetLastdayRain { response, time } => {
                        let res = get_lastday_rain(time);
                        let _ = response.send(res);
                    }
                    DatabaseCommand::GetLastdayET { response, time } => {
                        let res = get_lastday_et(time);
                        let _ = response.send(res);
                    }
                    DatabaseCommand::LoadAutoSchedule { response } => {
                        let res = load_auto_schedule(&conn);
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
        self.sender.send(DatabaseCommand::Execute { query: query.to_string(), params, response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }

    fn execute_batch(&self, query: &str) -> Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::ExecuteBatch { query: query.to_string(), response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }

    fn query_row(&self, query: &str, params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<String> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender
            .send(DatabaseCommand::QueryRow { query: query.to_string(), params, response: response_tx })
            .unwrap();
        response_rx.recv().unwrap()
    }

    fn load_sectors(&self) -> Result<Vec<SectorInfo>> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::LoadSectors { response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }

    fn load_cycles(&self) -> Result<Vec<Cycle>> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::LoadCycles { response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }

    fn log_watering_event(&self, evt: WateringEvent) -> Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::LogWateringEvent { evt, response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }

    fn get_current_weather(&self) -> Option<WeatherConditions> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::GetCurrentWeather { response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }

    fn get_lastday_rain(&self, time: i64) -> Option<f64> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::GetLastdayRain { time, response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }

    fn get_daily_et(&self, time: i64) -> Option<f64> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::GetLastdayET { time, response: response_tx }).unwrap();
        response_rx.recv().unwrap()
    }
    fn load_auto_schedule(&self) -> Result<Schedule> {
        let (response_tx, response_rx) = mpsc::channel();
        self.sender.send(DatabaseCommand::LoadAutoSchedule { response: response_tx }).unwrap();
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
            start_time INTEGER NOT NULL,
            duration INTEGER NOT NULL,
            PRIMARY KEY (id, sector_id),
            FOREIGN KEY (sector_id) REFERENCES sectors(id)
        );
        CREATE TABLE IF NOT EXISTS watering_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cycle_id INTEGER,
            sector_id INTEGER NOT NULL,
            start_time_utc TEXT NOT NULL,  -- Store as UTC
            duration REAL NOT NULL,
            water_applied REAL NOT NULL,
            type TEXT NOT NULL,
            FOREIGN KEY (sector_id) REFERENCES sectors(id)
        );
        CREATE TABLE IF NOT EXISTS auto_schedules (
            day_of_week INTEGER NOT NULL, -- Weekday as an integer (0 for Monday, 6 for Sunday)
            sector_id INTEGER NOT NULL,
            start_time INTEGER NOT NULL, -- Start time as a Unix UTC timestamp
            duration INTEGER NOT NULL,     -- Duration of watering in seconds
            PRIMARY KEY (day_of_week, sector_id, start_time)
        );

        CREATE TABLE IF NOT EXISTS wizard_schedule (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date INTEGER NOT NULL,   -- Unix UTC timestamp for the date
            sector_id INTEGER NOT NULL,
            start_time INTEGER NOT NULL,  -- Start time as Unix UTC timestamp
            duration INTEGER NOT NULL  -- Duration in seconds
        );
        ";

    conn.execute_batch(query)?;
    Ok(())
}

pub fn load_sectors(conn: &Connection) -> Result<Vec<SectorInfo>> {
    let mut stmt = conn
        .prepare("SELECT id, sprinkler_debit, percolation_rate, max_duration, weekly_target, progress FROM sectors")?;
    let sectors = stmt
        .query_map([], |row| {
            Ok(SectorInfo {
                id: row.get(0)?,
                sprinkler_debit: row.get(1)?,
                percolation_rate: row.get(2)?,
                max_duration: row.get::<_, i64>(3)? * 60,
                weekly_target: row.get(4)?,
                progress: row.get(5)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(sectors)
}

pub fn load_cycles(conn: &Connection) -> Result<Vec<Cycle>> {
    let mut stmt = conn.prepare("SELECT id, sector_id, start_time, duration FROM cycles ORDER BY id, sector_id")?;
    let mut cycles_map: std::collections::HashMap<u32, Vec<WaterSector>> = std::collections::HashMap::new();

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)? * 60_i64))
    })?;

    for row in rows {
        let (cycle_id, sector_id, start_time, duration) = row?;
        cycles_map.entry(cycle_id).or_default().push(WaterSector::new(sector_id, start_time, duration));
    }

    Ok(cycles_map.into_iter().map(|(id, instructions)| Cycle { id, instructions }).collect())
}

pub fn load_auto_schedule(conn: &Connection) -> Result<Schedule> {
    let mut stmt = conn.prepare(
        "SELECT day_of_week, sector_id, start_time, duration FROM auto_schedules ORDER BY day_of_week, sector_id, start_time",
    )?;
    // Use a HashMap to group sector and duration entries by day_of_week
    let mut entries_map: std::collections::HashMap<Weekday, DailyPlan> = std::collections::HashMap::new();

    let rows = stmt.query_map([], |row| {
        Ok((
            {
                let week_day = row.get::<_, i64>(0)?;
                print!("week_day: {:?}", Weekday::from_i64(week_day));
                Weekday::from_i64(week_day).unwrap()
            },
            row.get::<_, u32>(1)?, // Sector ID
            row.get::<_, i64>(2)?, // Start time
            row.get::<_, i64>(3)?, // Duration
        ))
    })?;

    for row in rows {
        let (day_of_week, sector_id, start_time, duration) = row?;
        entries_map.entry(day_of_week).or_default().push(WaterSector::new(sector_id, start_time, duration));
    }

    // Convert the HashMap into a Vec<ScheduleEntry>
    let entries = entries_map
        .into_iter()
        .map(|(day_of_week, start_times)| ScheduleEntry {
            schedule_type: ScheduleType::Weekday(day_of_week),
            start_times,
        })
        .collect();

    Ok(Schedule::new(entries))
}

pub fn save_auto_schedule(conn: &mut Connection, schedule: &Schedule) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch("DELETE FROM auto_schedules")?; // Clear previous schedule

    for entry in &schedule.entries {
        if let ScheduleType::Weekday(day_of_week) = entry.schedule_type {
            for &sec in &entry.start_times {
                tx.execute(
                    "INSERT INTO auto_schedules (day_of_week, sector_id, start_time, duration) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![day_of_week.num_days_from_monday(), sec.id, sec.start, sec.duration],
                )?;
            }
        }
    }

    tx.commit()
}

pub fn store_plan_in_db(conn: &mut Connection, weekly_plan: &WeeklyPlan) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch("DELETE FROM wizard_schedule")?; // Clear previous schedule
    for (date, sessions) in weekly_plan {
        for sec in sessions {
            tx.execute(
                "INSERT INTO wizard_schedule (date, sector_id, start_time, duration) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![date, sec.id, sec.start, sec.duration],
            )?;
        }
    }

    tx.commit()
}

pub fn log_watering_event(conn: &Connection, evt: WateringEvent) -> Result<()> {
    conn.execute(
        "INSERT INTO watering_events (cycle_id, sector_id, start_time, duration, water_applied, type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            evt.cycle_id,
            evt.sector.id,
            display_from_ts(evt.sector.start),
            evt.sector.duration as f64 / 60.,
            evt.water_applied,
            evt.mode.to_string()
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
        wind_speed: 15.0,
        temperature: 15.,
        humidity: 40.,
        solar_radiation: 1., // Example: Wind speed is 15 km/h
    })
}

pub fn get_lastday_rain(_time: i64) -> Option<f64> {
    // TODO:
    // Simulate retrievingrain
    // Replace with actual database or API query
    Some(1.)
}

pub fn get_lastday_et(_time: i64) -> Option<f64> {
    // TODO:
    // Simulate retrieving et
    // Replace with actual database or API query
    Some(1.)
}

#[cfg(test)]
mod test {
    use chrono::Weekday;

    use crate::{
        db::load_auto_schedule,
        watering::{ds::WaterSector, schedule::ScheduleType},
    };

    #[test]
    fn test_load_auto_schedule() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        // Adjusted table schema to match the refactored version
        conn.execute(
        "CREATE TABLE auto_schedules (day_of_week INTEGER, sector_id INTEGER, start_time INTEGER, duration INTEGER)",
        [],
    )
    .unwrap();

        // Insert test data with Unix UTC timestamps
        conn.execute(
            "INSERT INTO auto_schedules (day_of_week, sector_id, start_time, duration) VALUES (0, 101, 21600, 1800)",
            [],
        )
        .unwrap(); // Monday, sector 101, start time 06:00 UTC, 30 min duration
        conn.execute(
            "INSERT INTO auto_schedules (day_of_week, sector_id, start_time, duration) VALUES (0, 102, 28800, 3600)",
            [],
        )
        .unwrap(); // Monday, sector 102, start time 08:00 UTC, 60 min duration
        conn.execute(
            "INSERT INTO auto_schedules (day_of_week, sector_id, start_time, duration) VALUES (1, 201, 18000, 1200)",
            [],
        )
        .unwrap(); // Tuesday, sector 201, start time 05:00 UTC, 20 min duration

        let schedule = load_auto_schedule(&conn).unwrap();

        // Verify that we have two entries: one for Monday and one for Tuesday
        assert_eq!(schedule.entries.len(), 2);

        // Check Monday's schedule
        let monday_schedule = schedule
            .entries
            .iter()
            .find(|entry| matches!(entry.schedule_type, ScheduleType::Weekday(Weekday::Mon)))
            .unwrap();
        assert_eq!(
            monday_schedule.start_times,
            vec![WaterSector::new(101, 21600, 1800), WaterSector::new(102, 28800, 3600)] // Verify start times and durations
        );

        // Check Tuesday's schedule
        let tuesday_schedule = schedule
            .entries
            .iter()
            .find(|entry| matches!(entry.schedule_type, ScheduleType::Weekday(Weekday::Tue)))
            .unwrap();
        assert_eq!(
            tuesday_schedule.start_times,
            vec![WaterSector::new(201, 18000, 1200)] // Verify start time and duration
        );
    }
}
