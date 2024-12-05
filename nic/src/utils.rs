use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Datelike, Local, NaiveDateTime, TimeZone, Utc, Weekday};
use tokio::sync::{broadcast::{self, Receiver, Sender}, Mutex};

use crate::{test::utils::mock_time::MockTimeFormatter, time::TimeProvider, watering::ds::{ControlSignal, SectorInfo}};

pub fn display_time(utc_time: chrono::DateTime<Utc>) -> String {
    let local_time = utc_time.with_timezone(&chrono::Local);
    local_time.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn parse_datetime_to_utc_timestamp(date_str: &str, format: &str) -> Result<i64, chrono::ParseError> {
    let naive_datetime = NaiveDateTime::parse_from_str(date_str, format)?;
    let datetime_utc: DateTime<Utc> = Utc.from_utc_datetime(&naive_datetime);
    Ok(datetime_utc.timestamp())
}

pub fn display_from_ts(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0).unwrap().to_string()
}

pub fn timezone_offset() -> chrono::Duration {
    let local_time = Local::now();
    let utc_time = local_time.with_timezone(&Utc);
    local_time.naive_local() - utc_time.naive_utc()
}

pub fn sod(ts: i64) -> i64 {
    ts - (ts % 86400)
}

pub fn start_log<T: TimeProvider + 'static>(time_provider: Option<Arc<T>>) {
    if let Some(time_provider) = time_provider {
        let time_formatter = MockTimeFormatter { time_provider };

        tracing_subscriber::fmt()
            .with_timer(time_formatter)
            .with_env_filter("nic=debug")
            .with_target(false) // Hide target module info
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("nic=debug")
            .with_target(false) // Hide target module info
            .init();
    }
}

pub fn get_week_day_from_ts(time: i64) -> Weekday {
    let datetime = DateTime::<Utc>::from_timestamp(time, 0).unwrap();
    datetime.weekday()
}

pub fn init_channels()->(Arc<Sender<ControlSignal>>, Arc<Mutex<Receiver<ControlSignal>>>){
    let (tx, rx) = broadcast::channel::<ControlSignal>(100);
    (Arc::new(tx),Arc::new(Mutex::new(rx)))
}

/// Assumes that whevever the machine stops, nect start will be with 0 progress.<br>
/// It is not supposed that the machine stops.  It it does (maintenance), or other reason, we do not know for sure how long it stopped.<br>
pub fn load_sectors_into_hashmap(sectors: Vec<SectorInfo>) -> HashMap<u32, SectorInfo> {
    // TODO: maybe we can start with a parameter giving that indication and move from there to make the thing smarter.
    // or just get some last rec from db and use that
    let sectors = sectors
        .iter()
        .map(|sector| {
            let mut sec = sector.clone();
            sec.progress = 0.;
            (sector.id, sec)
        })
        .collect();
    sectors
}


#[cfg(test)]
mod test {
    use crate::utils::timezone_offset;

    #[test]
    fn lx() {
        let offset = timezone_offset();
        println!("Timezone offset: {}", offset);
    }
}
