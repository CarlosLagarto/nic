use super::ds::{DailyPlan, SectorInfo, WaterSector};
use crate::utils::{get_week_day_from_ts, sod};
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


    pub fn get_next_wizard_schedule(&self, timestamp: i64) -> Option<DailyPlan> {
        let day_start = sod(timestamp); // 86,400 seconds in a day


        self.entries
            .iter()
            .filter_map(|entry| match &entry.schedule_type {
                ScheduleType::Date(date) if *date == day_start => {
                    // Return all valid start_times for the specific date
                    Some(
                        entry.start_times.iter().filter(|sec| sec.start + sec.duration >= timestamp).cloned().collect(),
                    )
                }
                _ => None,
            })
            .next() // Get the first matching schedule
    }

    pub fn get_next_auto_schedule(&self, current_time: i64) -> Option<WaterSector> {
        let weekday = get_week_day_from_ts(current_time);

        // Find the next schedule entry for the current or subsequent weekdays
        self.entries
            .iter()
            .filter_map(|entry| match &entry.schedule_type {
                ScheduleType::Weekday(day) if *day == weekday => {
                    entry
                        .start_times
                        .iter()
                        .filter_map(|&sec| if sec.start >= current_time { Some(sec) } else { None })
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
            return None; // No watering needed; target met
        }
        let soil_capacity_cm = 2.5; // Temporary holding capacity in cm
        let extra_time_seconds = (soil_capacity_cm / sector.sprinkler_debit) * 3600.0;

        let tolerance_factor = 1.2; // Allow 20% tolerance
        let adjusted_percolation_rate = sector.percolation_rate * tolerance_factor;
        let max_percolation_time_seconds = ((adjusted_percolation_rate / 10.0) / sector.sprinkler_debit) * 3600.0;

        let adjusted_percolation_time = (max_percolation_time_seconds + extra_time_seconds) as i64;

        let empirical_cap = sector.max_duration; // Empirical cap, typical is 30 minutes

        Some(irrigation_time.min(adjusted_percolation_time).min(empirical_cap))
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
                    daily_plan.into_iter().collect(),
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

            // let acc = current_time;
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
                        if current_time + actual_duration + 20 <= end_time {
                            //TODO const for safe factor
                            daily_plan.push(WaterSector::new(sector.id, current_time, actual_duration));
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
    use chrono::{TimeZone, Weekday};

    use crate::{utils::sod, watering::{
        ds::{SectorInfo, WaterSector},
        schedule::{AllowedTimeframe, Schedule, ScheduleEntry, ScheduleType, WateringSchedule},
    }};

    #[test]
    fn allowed_timeframe() {
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
    async fn et_adjustments() {
        let mut sectors = vec![SectorInfo::build(1, 3., 1., 30 * 60, 0.5, 0.5)];

        WateringSchedule::adjust_progress_for_et_and_rain(
            &mut sectors.iter_mut().collect::<Vec<&mut SectorInfo>>(),
            1.,
            0.5,
        )
    }

    #[test]
    fn get_next_auto_schedule() {
        // Create a schedule with mixed ScheduleType entries (Weekday and Date)
        let entries = vec![
            ScheduleEntry {
                schedule_type: ScheduleType::Weekday(Weekday::Mon),
                start_times: vec![
                    WaterSector { id: 1, start: 1667836800, duration: 3600 }, // Example Monday timestamp
                ],
            },
            ScheduleEntry {
                schedule_type: ScheduleType::Date(1667923200), // Specific date
                start_times: vec![
                    WaterSector { id: 2, start: 1667923200, duration: 3600 }, // Example specific date timestamp
                ],
            },
            ScheduleEntry {
                schedule_type: ScheduleType::Weekday(Weekday::Tue),
                start_times: vec![
                    WaterSector { id: 3, start: 1667926800, duration: 3600 }, // Example Tuesday timestamp
                ],
            },
        ];

        // Create a schedule instance
        let schedule = Schedule { entries };

        // Simulate a Monday timestamp
        let monday_timestamp = 1667836800; // Example Monday timestamp
        let next_schedule = schedule.get_next_auto_schedule(monday_timestamp);

        // Expected result: The earliest entry for Monday
        assert_eq!(next_schedule, Some(WaterSector { id: 1, start: 1667836800, duration: 3600 }));

        // Simulate a timestamp on the specific date (should be ignored for auto mode)
        let date_specific_timestamp = 1667923200; // Example timestamp for the specific date
        let next_schedule = schedule.get_next_auto_schedule(date_specific_timestamp);

        // Expected result: The Tuesday entry since the Date entry should be ignored
        assert_eq!(next_schedule, Some(WaterSector { id: 3, start: 1667926800, duration: 3600 }));

        // Simulate a Tuesday timestamp
        let tuesday_timestamp = 1667926800; // Example Tuesday timestamp
        let next_schedule = schedule.get_next_auto_schedule(tuesday_timestamp);

        // Expected result: The Tuesday entry
        assert_eq!(next_schedule, Some(WaterSector { id: 3, start: 1667926800, duration: 3600 }));

        // Simulate a timestamp after all entries
        let late_tuesday_timestamp = 1667930400; // After Tuesday's entry
        let next_schedule = schedule.get_next_auto_schedule(late_tuesday_timestamp);

        // Expected result: No valid entries
        assert_eq!(next_schedule, None);
    }

    #[test]
    fn get_next_wizard_schedule() {
        // Create a schedule with specific dates
        let schedule_entries = vec![ScheduleEntry {
            schedule_type: ScheduleType::Date(sod(1692508800)), // Unix UTC timestamp for the start of the day
            start_times: vec![
                WaterSector::new(1, 1692512400, 30 * 60), // Sector 1, starts at 02:00 UTC, duration 30 min
                WaterSector::new(2, 1692519600, 20 * 60), // Sector 2, starts at 04:00 UTC, duration 20 min
                WaterSector::new(3, 1692526800, 45 * 60), // Sector 3, starts at 06:00 UTC, duration 45 min
            ],
        }];

        let schedule = Schedule::new(schedule_entries);

        // Test for a specific day
        let current_time = 1692516000; // 03:00 UTC on the same day
        let result = schedule.get_next_wizard_schedule(current_time);

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            vec![
                // WaterSector::new(1, 1692512400, 30 * 60), // Sector 1, starts at 02:00 UTC, duration 30 min
                WaterSector::new(2, 1692519600, 20 * 60), // Sector 2, starts at 04:00 UTC, duration 20 min
                WaterSector::new(3, 1692526800, 45 * 60), // Sector 3, starts at 06:00 UTC, duration 45 min
            ]
        );

        // Test for a day with no schedule
        let current_time = 1692595200; // A different day
        assert!(schedule.get_next_wizard_schedule(current_time).is_none());
    }

}
