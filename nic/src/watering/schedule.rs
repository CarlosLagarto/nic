use super::ds::{DailyPlan, SectorInfo};
use crate::utils::sod;
use chrono::{Datelike, TimeZone};
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub struct AllowedTimeframe {
    pub start: i64,    // Start time in seconds from the start of the base day
    pub duration: i64, // Duration in seconds (can span across days)
}

impl AllowedTimeframe {
    /// Create a new timeframe with a start hour and duration in hours.
    pub fn new(hour_start: i64, duration_hours: i64) -> Self {
        let start = hour_start * 3600;
        let duration = duration_hours * 3600;
        Self { start, duration }
    }

    /// Compute the absolute start and end times based on the base day's Unix timestamp (start of day).
    pub fn for_day(&self, base_time: i64) -> (i64, i64) {
        let start_time = base_time + self.start;
        let end_time = start_time + self.duration - 1; // Inclusive end
        (start_time, end_time)
    }

    /// Check if a given `current_time` falls within the allowed timeframe,
    /// considering possible cross-day spans.
    pub fn is_within(&self, current_time: i64, base_time: i64) -> bool {
        let (start_time, end_time) = self.for_day(base_time);

        if end_time < start_time {
            // Cross-day case: split into two intervals
            let first_interval = (start_time, base_time + 86400 - 1); // Until the end of the base day
            let second_interval = (base_time + 86400, end_time + 86400); // Start of next day

            (current_time >= first_interval.0 && current_time <= first_interval.1)
                || (current_time >= second_interval.0 && current_time <= second_interval.1)
        } else {
            // Same-day case
            current_time >= start_time && current_time <= end_time
        }
    }
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub entries: Vec<ScheduleEntry>,
}

#[derive(Clone, Debug)]
pub enum ScheduleType {
    Weekday(chrono::Weekday), // For auto mode
    Date(i64),                // For wizard mode (specific dates)
}

#[derive(Clone, Debug)]
pub struct ScheduleEntry {
    pub schedule_type: ScheduleType,
    pub start_times: DailyPlan,
}

impl Schedule {
    pub fn new(entries: Vec<ScheduleEntry>) -> Self {
        Self { entries }
    }

    pub fn get_next_wizard_schedule(&self, timestamp: i64) -> Option<&DailyPlan> {
        let day_start = sod(timestamp); // 86,400 seconds in a day

        self.entries
            .iter()
            .find(|entry| match &entry.schedule_type {
                ScheduleType::Date(day) => *day == day_start,
                _ => false,
            })
            .map(|entry| &entry.start_times)
            .filter(|plan| {
                // Filter out sessions where the current time is after their end time
                plan.iter().any(|(_, start_time, duration)| timestamp <= *start_time + *duration)
            })
    }

    pub fn get_next_auto_schedule(&self, current_time: i64) -> Option<(u32, i64, i64)> {
        let now = chrono::Utc.timestamp_opt(current_time, 0).unwrap();
        let weekday = now.weekday();

        // Find the next schedule entry for the current or subsequent weekdays
        self.entries
            .iter()
            .filter_map(|entry| match &entry.schedule_type {
                ScheduleType::Weekday(day) if *day == weekday => {
                    entry
                        .start_times
                        .iter()
                        .filter_map(|&(sector_id, start_time, duration)| {
                            if start_time >= current_time {
                                Some((sector_id, start_time, duration)) // Return sector_id, start_time, and duration
                            } else {
                                None
                            }
                        })
                        .min() // Get the earliest valid start_time
                }
                _ => None, // Ignore wizard mode entries
            })
            .min() // Find the earliest time across all applicable entries
    }
}

/// Handles scheduling logic for watering
pub struct WateringSchedule;

impl WateringSchedule {
    pub fn adjust_progress_for_et_and_rain(sectors: &mut [&mut SectorInfo], daily_et: f64, daily_rain: f64) {
        let adjustment = daily_et - daily_rain;
        for sector in sectors.iter_mut() {
            sector.progress = (sector.progress - adjustment).max(0.0);
            info!(
                "Sector {}: Adjusted progress by -{:.2} cm due to evapotranspiration and +{:.2} mm due to rain. New progress: {:.2} cm.",
                sector.id, daily_et, daily_rain, sector.progress
            );
        }
    }

    pub fn calculate_irrigation_time(sector: &SectorInfo) -> Option<i64> {
        let irrigation_duration = sector.weekly_target - sector.progress; // Total water needed in cm
        let irrigation_time = ((irrigation_duration / sector.sprinkler_debit) * 3600.0) as i64;
        if irrigation_time <= 0 {
            None // No watering needed; target met
        } else {
            let soil_capacity_cm = 2.5; // Temporary holding capacity in cm
            let extra_time_seconds = (soil_capacity_cm / sector.sprinkler_debit) * 3600.0;

            let tolerance_factor = 1.2; // Allow 20% tolerance
            let adjusted_percolation_rate = sector.percolation_rate * tolerance_factor;
            let max_percolation_time_seconds = ((adjusted_percolation_rate / 10.0) / sector.sprinkler_debit) * 3600.0;

            let adjusted_percolation_time = (max_percolation_time_seconds + extra_time_seconds) as i64;

            let empirical_cap = sector.max_duration; // Empirical cap, typical is 30 minutes

            Some(irrigation_time.min(adjusted_percolation_time).min(empirical_cap))
        }
    }

    pub fn recalculate_remaining_plan(
        sectors: &[SectorInfo], total_days: usize, daily_et: f64, current_time: i64, timeframe: AllowedTimeframe,
    ) -> Schedule {
        // Generate the weekly plan using the updated function signature
        let updated_entries =
            WateringSchedule::generate_weekly_plan(sectors, total_days, daily_et, current_time, timeframe);

        // Convert the generated weekly plan into a schedule format (ScheduleEntry)
        let schedule_entries: Vec<ScheduleEntry> = updated_entries
            .iter()
            .map(|(date, daily_plan)| ScheduleEntry {
                schedule_type: ScheduleType::Date(*date),
                start_times: daily_plan.clone(),
            })
            .collect();

        Schedule::new(schedule_entries)
    }

    pub fn generate_weekly_plan(
        sectors: &[SectorInfo], total_days: usize, daily_et: f64, current_time: i64, timeframe: AllowedTimeframe,
    ) -> Vec<(i64, DailyPlan)> {
        // Distribute sessions using the WateringSchedule logic
        let schedule = WateringSchedule::distribute_sessions(sectors, total_days, daily_et, timeframe, current_time);
        let start_of_day = sod(current_time);

        // Convert the distributed sessions into a weekly plan with specific dates
        schedule
            .into_iter()
            .enumerate()
            .map(|(day, daily_plan)| {
                let day_timestamp = start_of_day + (day as i64 * 86_400); // Add days in seconds
                (
                    day_timestamp, // Unix UTC timestamp for the start of the day
                    daily_plan.into_iter().map(|tuple| tuple).collect(),
                )
            })
            .collect()
    }

    /// Distributes watering sessions across sectors and days
    pub fn distribute_sessions(
        sectors: &[SectorInfo], total_days: usize, daily_et: f64, timeframe: AllowedTimeframe, mut current_time: i64,
    ) -> Vec<DailyPlan> {
        let mut plans = Vec::new();

        // Loop through each day to distribute watering sessions
        for day in 0..total_days {
            let mut daily_plan = Vec::new();

            // Calculate the start and end times for the current day
            let base_time = sod(current_time) + (day as i64 * 86_400);
            let (start_time, end_time) = timeframe.for_day(base_time);

            // Adjust current_time to the start of the allowed timeframe
            if current_time < start_time {
                current_time = start_time;
            }

            for sector in sectors {
                let remaining_target = sector.weekly_target - sector.progress;
                if remaining_target > 0.0 {
                    // Adjust for daily evapotranspiration
                    let adjusted_target = (remaining_target / total_days as f64) - daily_et;
                    if adjusted_target > 0.0 {
                        let duration = ((adjusted_target / sector.sprinkler_debit) * 3600.0).ceil() as i64;

                        // Ensure the watering duration does not exceed the sector's daily limit
                        let actual_duration = duration.min(sector.max_duration);

                        // Add session to the daily plan if it fits within the timeframe
                        if current_time + actual_duration <= end_time {
                            daily_plan.push((sector.id, current_time, actual_duration));
                            current_time += actual_duration;
                        } else {
                            break; // Stop adding sessions if no more time is available
                        }
                    }
                }
            }
            plans.push(daily_plan);
            current_time = base_time + 86_400; // Move to the next day's start of day
        }
        plans
    }
}

#[cfg(test)]
mod test {
    use chrono::TimeZone;

    use crate::watering::{
        ds::SectorInfo,
        schedule::{AllowedTimeframe, WateringSchedule},
    };

    #[test]
    fn test_allowed_timeframe() {
        // Example: Define a timeframe from 6:00 AM to 8:00 AM (2 hours)
        let timeframe = AllowedTimeframe::new(6, 2);

        // Set the base time to Nov 25, 2024, midnight (start of the day)
        let base_time = chrono::Utc.with_ymd_and_hms(2024, 11, 25, 0, 0, 0).unwrap().timestamp();

        // Compute the actual timeframe for the specific day
        let (start_time, end_time) = timeframe.for_day(base_time);

        // Verify the converted start and end times
        assert_eq!(start_time, chrono::Utc.with_ymd_and_hms(2024, 11, 25, 6, 0, 0).unwrap().timestamp());
        assert_eq!(end_time, chrono::Utc.with_ymd_and_hms(2024, 11, 25, 7, 59, 59).unwrap().timestamp());

        // Test if a time is within the timeframe
        let test_time = chrono::Utc.with_ymd_and_hms(2024, 11, 25, 7, 0, 0).unwrap().timestamp();
        assert!(timeframe.is_within(test_time, base_time));

        // Test a time outside the timeframe (after end time)
        let outside_time_after = chrono::Utc.with_ymd_and_hms(2024, 11, 25, 9, 0, 0).unwrap().timestamp();
        assert!(!timeframe.is_within(outside_time_after, base_time));

        // Test a time outside the timeframe (before start time)
        let outside_time_before = chrono::Utc.with_ymd_and_hms(2024, 11, 25, 5, 0, 0).unwrap().timestamp();
        assert!(!timeframe.is_within(outside_time_before, base_time));
    }

    #[tokio::test]
    async fn test_et_adjustments() {
        let mut sectors = vec![SectorInfo::build(1, 3., 1., 30 * 60, 0.5, 0.5)];

        WateringSchedule::adjust_progress_for_et_and_rain(
            &mut sectors.iter_mut().collect::<Vec<&mut SectorInfo>>(),
            1.,
            0.5,
        )
    }
}
