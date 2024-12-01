use super::{
    ds::{Cycle, EventType},
    schedule::Schedule,
    watering_system::WateringSystem,
};
use crate::{db::DatabaseTrait, sensors::interface::SensorController, utils::display_from_ts};
use std::sync::Arc;
use tracing::info;

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
        &self, water_sys: &WateringSystem<C>, db: &Arc<D>, current_utc_ts: i64,
    ) {
        if let Some(next_start_time) = self.schedule.get_next_auto_schedule(current_utc_ts) {
            if current_utc_ts >= next_start_time.1 {
                let mut sm = water_sys.state_machine.write().await;
                if sm.cycle.is_none() {
                    info!(
                        "Auto Mode: Starting watering cycle at {:?} (local time).",
                        display_from_ts(next_start_time.1)
                    );
                    let new_cycle = Cycle {
                        id: current_utc_ts as u32,
                        instructions: vec![(next_start_time.0, next_start_time.2)], // (sector_id, duration)
                    };
                    sm.start_cycle(new_cycle);
                }
                drop(sm);  // release lock
                water_sys.update(db, EventType::Auto).await;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::TimeZone;

    use crate::{
        utils::sod,
        watering::schedule::{Schedule, ScheduleEntry, ScheduleType},
    };

    #[test]
    fn test_get_next_auto_schedule() {
        let current_time = chrono::Utc
            .with_ymd_and_hms(2024, 11, 25, 5, 0, 0) // Monday at 07:00 UTC
            .unwrap()
            .timestamp();
        let monday = sod(current_time);
        let tuesday = monday + 86400;
        // Create a schedule for auto mode with weekdays
        let schedule_entries = vec![
            ScheduleEntry {
                schedule_type: ScheduleType::Weekday(chrono::Weekday::Mon),
                start_times: vec![
                    (1, monday + 6 * 3600, 30 * 60),  // Sector 1, 06:00 UTC, 30 min duration
                    (1, monday + 18 * 3600, 30 * 60), // Sector 1, 18:00 UTC, 30 min duration
                ],
            },
            ScheduleEntry {
                schedule_type: ScheduleType::Weekday(chrono::Weekday::Tue),
                start_times: vec![
                    (2, tuesday + 8 * 3600, 20 * 60), // Sector 2, 08:00 UTC, 20 min duration
                ],
            },
        ];

        let schedule = Schedule::new(schedule_entries);

        // Test for Monday

        // Get the next start time for the auto schedule
        let result = schedule.get_next_auto_schedule(current_time);

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            (1, monday + 6 * 3600, 30 * 60) // Sector 1, start at 06:00 UTC, 30 min duration
        );

        // Test for Tuesday
        let current_time = chrono::Utc
            .with_ymd_and_hms(2024, 11, 26, 6, 0, 0) // Tuesday at 06:00 UTC
            .unwrap()
            .timestamp();

        let result = schedule.get_next_auto_schedule(current_time);

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            (2, tuesday + 8 * 3600, 20 * 60) // Sector 3, start at 08:00 UTC, 20 min duration
        );

        // Test when no valid start times exist
        let current_time = chrono::Utc
            .with_ymd_and_hms(2024, 11, 27, 6, 0, 0) // Wednesday at 06:00 UTC
            .unwrap()
            .timestamp();

        assert!(schedule.get_next_auto_schedule(current_time).is_none());
    }
}
