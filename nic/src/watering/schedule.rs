use super::ds::{DailyPlan, SectorInfo};
use chrono::{Datelike, NaiveDate, NaiveTime};

#[derive(Debug, Clone, Copy)]
pub struct AllowedTimeframe {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl AllowedTimeframe {
    pub fn is_within(&self, current_time: NaiveTime) -> bool {
        if self.start <= self.end {
            current_time >= self.start && current_time <= self.end
        } else {
            // Handles timeframes that span midnight (e.g., 10 PM to 6 AM)
            current_time >= self.start || current_time <= self.end
        }
    }
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub entries: Vec<ScheduleEntry>,
}

#[derive(Clone, Debug)]
pub struct ScheduleEntry {
    pub day_of_week: chrono::Weekday,
    pub start_times: DailyPlan, // DailyPlan is Vec<(u32, TimeDelta)>
}

impl Schedule {
    pub fn new(entries: Vec<ScheduleEntry>) -> Self {
        Self { entries }
    }

    pub fn is_today_scheduled(&self, date: chrono::NaiveDate) -> bool {
        self.entries.iter().any(|entry| entry.day_of_week == date.weekday())
    }

    pub fn get_next_start_time(&self, current_time: NaiveTime) -> Option<NaiveTime> {
        self.entries
            .iter()
            .flat_map(|entry| {
                entry.start_times.iter().map(|&(_sector_id, duration)| {
                    let duration_as_time = NaiveTime::from_num_seconds_from_midnight_opt(duration.num_seconds() as u32, 0)
                        .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap()); // Fallback to 00:00:00
                    duration_as_time
                })
            })
            .filter(|&start_time| start_time >= current_time)
            .min()
    }

    pub fn get_schedule_for_date(&self, date: NaiveDate) -> Option<&DailyPlan> {
        let day_of_week = date.weekday(); // Extract the weekday from the date
        self.entries
            .iter()
            .find(|entry| entry.day_of_week == day_of_week) // Match by day_of_week
            .map(|entry| &entry.start_times)
    }
}

/// Handles scheduling logic for watering
pub struct WateringSchedule;

impl WateringSchedule {
    /// Distributes watering sessions across sectors and days
    pub fn distribute_sessions(sectors: &[SectorInfo], total_days: usize, daily_duration: chrono::Duration, daily_et: f64) -> Vec<DailyPlan> {
        let mut plans = Vec::new();

        // Loop through each day to distribute watering sessions
        for _ in 0..total_days {
            let mut daily_plan = Vec::new();

            for sector in sectors {
                let remaining_target = sector.weekly_target - sector.progress;
                if remaining_target > 0.0 {
                    // Adjust for daily evapotranspiration
                    let adjusted_target = (remaining_target / total_days as f64) - daily_et;
                    if adjusted_target > 0.0 {
                        let duration = chrono::Duration::seconds(((adjusted_target / sector.sprinkler_debit) * 3600.0).ceil() as i64);
                        // Ensure the watering duration does not exceed the daily limit
                        daily_plan.push((sector.id, duration.min(daily_duration)));
                    }
                }
            }

            plans.push(daily_plan);
        }

        plans
    }

    pub fn calculate_daily_duration(timeframe: AllowedTimeframe, total_days: usize) -> chrono::Duration {
        let total_available_seconds = (timeframe.end - timeframe.start).num_seconds().max(1); // Avoid division by zero

        // Divide the total available duration evenly across the total days
        chrono::Duration::seconds(total_available_seconds / total_days as i64)
    }
}

#[test]
fn test_allowed_timeframe() {
    let timeframe = AllowedTimeframe { start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(), end: NaiveTime::from_hms_opt(6, 0, 0).unwrap() };

    assert!(timeframe.is_within(NaiveTime::from_hms_opt(23, 0, 0).unwrap())); // 11:00 PM
    assert!(timeframe.is_within(NaiveTime::from_hms_opt(5, 30, 0).unwrap())); // 5:30 AM
    assert!(!timeframe.is_within(NaiveTime::from_hms_opt(7, 0, 0).unwrap())); // 7:00 AM
}
