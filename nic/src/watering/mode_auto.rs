use super::{
    ds::{Cycle, EventType},
    schedule::Schedule,
    watering_system::WateringSystem,
};
use crate::{db::DatabaseTrait, sensors::interface::SensorController, utils::timezone_offset};
use chrono::{Datelike, NaiveTime};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone, Debug)]
pub struct ModeAuto {
    pub cycle: Cycle,
    pub schedule: Schedule, // Store the schedule here
}

impl ModeAuto {
    pub fn new(cycle: Cycle, schedule: Schedule) -> Self {
        Self { cycle, schedule }
    }

    pub async fn execute<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &self, water_sys: &WateringSystem<C>, db: &Arc<D>, current_datetime_utc: chrono::DateTime<chrono::Utc>,
    ) {
        if water_sys.is_idle().await {
            info!("Auto Mode: Machine is stopped. Skipping execution.");
            return;
        }
        let current_datetime_local = current_datetime_utc.with_timezone(&chrono::Local);
        if !self.schedule.is_today_scheduled(current_datetime_local.date_naive()) {
            debug!("Auto Mode: No watering scheduled for today ({:?}).", current_datetime_local.date_naive().weekday());
            return;
        }
        let time = current_datetime_local.time();
        if let Some(next_start_time) = self.schedule.get_next_start_time(time) {
            if time >= next_start_time {
                let mut sm = water_sys.state_machine.write().await;
                if sm.cycle.is_none() {
                    info!("Auto Mode: Starting watering cycle at {:?} (local time).", next_start_time);
                    sm.start_cycle(self.cycle.clone());
                }
                water_sys.update(db, EventType::Auto).await;
            }
        }
        water_sys.update(db, EventType::Auto).await;
    }

    // TODO
    pub fn calculate_next_start(&self, current_time: NaiveTime) -> Option<NaiveTime> {
        self.schedule
            .entries
            .iter()
            .flat_map(|entry| &entry.start_times)
            .filter_map(|&(_sector_id, duration)| {
                // Assume that duration represents the start time as an offset
                let start_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap() + duration;
                if start_time >= current_time {
                    Some(start_time) // Only include times that are >= current_time
                } else {
                    None
                }
            })
            .min() // Find the earliest start time
            .map(|local_time| local_time + timezone_offset()) // Convert to UTC
    }
}

#[cfg(test)]
mod test {
    use chrono::Duration;

    use super::*;
    use crate::watering::schedule::ScheduleEntry;
    #[test]
    fn test_calculate_next_start() {
        // Define a schedule with multiple entries
        let schedule_entries = vec![
            ScheduleEntry {
                day_of_week: chrono::Weekday::Mon,
                start_times: vec![(1, Duration::minutes(30)), (1, Duration::minutes(30)), (1, Duration::minutes(30))],
            },
            ScheduleEntry { day_of_week: chrono::Weekday::Mon, start_times: vec![(2, Duration::minutes(20)), (2, Duration::minutes(20))] },
        ];

        let schedule = Schedule::new(schedule_entries);

        let auto_mode = ModeAuto::new(Cycle { id: 1, instructions: vec![] }, schedule);

        // Test within the schedule
        let current_time = NaiveTime::from_hms_opt(7, 30, 0).unwrap();
        assert_eq!(auto_mode.schedule.get_next_start_time(current_time), Some(NaiveTime::from_hms_opt(8, 0, 0).unwrap()));

        // Test before the first scheduled time
        let current_time = NaiveTime::from_hms_opt(5, 0, 0).unwrap();
        assert_eq!(auto_mode.schedule.get_next_start_time(current_time), Some(NaiveTime::from_hms_opt(6, 30, 0).unwrap()));

        // Test after the last scheduled time
        let current_time = NaiveTime::from_hms_opt(21, 0, 0).unwrap();
        assert_eq!(auto_mode.schedule.get_next_start_time(current_time), None);
    }
}
