use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

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

pub fn start_log() {
    tracing_subscriber::fmt()
        .with_env_filter("nic=debug")
        .with_target(false) // Hide target module info
        .init();
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
