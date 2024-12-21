use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use chrono::{DateTime, Datelike, Local, NaiveDateTime, TimeZone, Timelike, Utc, Weekday};
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    Mutex,
};

use crate::{
    test::utils::mock_time::MockTimeFormatter,
    time::TimeProvider,
    watering::ds::{CtrlSignal, SectorInfo},
    MAX_MSGS,
};

pub fn utc_datetime_to_string(utc_time: chrono::DateTime<Utc>) -> String {
    utc_time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn parse_datetime_to_utc_timestamp(date_str: &str, format: &str) -> Result<i64, chrono::ParseError> {
    NaiveDateTime::parse_from_str(date_str, format).map(|naive| Utc.from_utc_datetime(&naive).timestamp())
}

pub fn ux_ts_to_string(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0).unwrap().to_string()
}

pub fn timezone_offset() -> chrono::Duration {
    let local_time = Local::now();
    let utc_time = local_time.with_timezone(&Utc);
    local_time.naive_local() - utc_time.naive_utc()
}

pub fn sod(ts: i64) -> i64 {
    ts - (ts % 86_400)
}

pub fn start_log(time_provider: Option<Arc<dyn TimeProvider>>) {
    let subscriber_builder = tracing_subscriber::fmt().with_env_filter("nic=debug").with_target(false); // Hide target module info

    if let Some(time_provider) = time_provider {
        let time_formatter = MockTimeFormatter { time_provider };
        subscriber_builder.with_timer(time_formatter).init();
    } else {
        subscriber_builder.init();
    }
}

pub fn get_week_day_from_ts(time: i64) -> Weekday {
    let datetime = DateTime::<Utc>::from_timestamp(time, 0).unwrap();
    datetime.weekday()
}

pub fn get_hour_from_ts(time: i64) -> u32 {
    let datetime = DateTime::<Utc>::from_timestamp(time, 0).unwrap();
    datetime.hour()
}

pub fn init_channels() -> (Arc<Sender<CtrlSignal>>, Arc<Mutex<Receiver<CtrlSignal>>>) {
    let (tx, rx) = broadcast::channel::<CtrlSignal>(MAX_MSGS);
    (Arc::new(tx), Arc::new(Mutex::new(rx)))
}

pub fn init_broadcast_channels() -> (tokio::sync::broadcast::Sender<CtrlSignal>, tokio::sync::broadcast::Receiver<CtrlSignal>) {
    let (tx, rx) = broadcast::channel::<CtrlSignal>(MAX_MSGS);
    (tx, rx)
}

/// Assumes that whevever the machine stops, nect start will be with 0 progress.<br>
/// It is not supposed that the machine stops.  It it does (maintenance), or other reason, we do not know for sure how long it stopped.<br>
pub fn load_sectors_into_hashmap(sectors: Vec<SectorInfo>) -> HashMap<u32, SectorInfo> {
    // TODO: maybe we can start with a parameter giving that indication and move from there to make the thing smarter.
    // or just get some last rec from db and use that
    sectors
        .into_iter()
        .map(|sector| {
            let mut sec = sector.clone();
            sec.progress = 0.;
            (sector.id, sec)
        })
        .collect()
}

pub fn remove_folder_from_path(path: &Path, target_folder: &str) -> PathBuf {
    let mut new_path = PathBuf::new();

    for component in path.components() {
        // Check if the current component matches the target folder
        if component.as_os_str() != target_folder {
            new_path.push(component);
        }
    }

    new_path
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
