use chrono::{Local, Utc};

pub fn display_time(utc_time: chrono::DateTime<Utc>) -> String {
    let local_time = utc_time.with_timezone(&chrono::Local);
    local_time.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn timezone_offset() -> chrono::Duration {
    let local_time = Local::now();
    let utc_time = local_time.with_timezone(&Utc);
    local_time.naive_local() - utc_time.naive_utc()
}

#[cfg(test)]
mod test{
    use crate::utils::timezone_offset;

    #[test]
    fn lx() {
        let offset = timezone_offset();
        println!("Timezone offset: {}", offset);
    }
}