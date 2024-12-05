use super::ds::{Cycle, WateringState};
use super::schedule::Schedule;
use num_derive::FromPrimitive;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, FromPrimitive)]
#[repr(usize)]
pub enum ModeIdx {
    Auto = 0,
    Manual = 1,
    Wizard = 2,
}

impl Display for ModeIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeIdx::Auto => write!(f, "auto"),
            ModeIdx::Manual => write!(f, "manual"),
            ModeIdx::Wizard => write!(f, "wizard"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModeAuto {
    pub cycle: Cycle,
    pub schedule: Schedule, // Store the schedule here
}

impl ModeAuto {
    pub fn new(cycle: Cycle, schedule: Schedule) -> Self {
        Self { cycle, schedule }
    }
}

#[derive(Clone, Debug)]
pub struct ModeWizard {
    pub paused_state: Option<(WateringState, Cycle, usize)>, // Track paused state
    pub schedule: Schedule,
}

impl ModeWizard {
    pub fn new(schedule: Schedule) -> Self {
        Self { paused_state: None, schedule }
    }
}

#[derive(Clone, Debug)]
pub struct ModeManual {
    pub cycle: Cycle,
}

impl ModeManual {
    pub fn new(cycle: Cycle) -> Self {
        Self { cycle }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::sod;
    use crate::watering::ds::{Cycle, EnvironmentalSignal, WaterSector, WateringState};
    use crate::watering::modes::ModeIdx;
    use crate::watering::water_state::WaterState;
    use crate::watering::{
        ds::SectorInfo,
        schedule::{AllowedTimeframe, Schedule, ScheduleEntry, ScheduleType, WateringSchedule},
    };
    use chrono::{TimeZone, Weekday};

    #[test]
    fn get_next_auto_schedule() {
        let current_time = chrono::Utc
            .with_ymd_and_hms(2024, 11, 25, 5, 0, 0) // Monday at 07:00 UTC
            .unwrap()
            .timestamp();
        let monday = sod(current_time);
        let tuesday = monday + 86400;
        // Create a schedule for auto mode with weekdays
        let schedule_entries = vec![
            ScheduleEntry {
                schedule_type: ScheduleType::Weekday(Weekday::Mon),
                start_times: vec![
                    WaterSector::new(1, monday + 6 * 3600, 30 * 60),  // Sector 1, 06:00 UTC, 30 min duration
                    WaterSector::new(1, monday + 18 * 3600, 30 * 60), // Sector 1, 18:00 UTC, 30 min duration
                ],
            },
            ScheduleEntry {
                schedule_type: ScheduleType::Weekday(Weekday::Tue),
                start_times: vec![
                    WaterSector::new(2, tuesday + 8 * 3600, 20 * 60), // Sector 2, 08:00 UTC, 20 min duration
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
            WaterSector::new (1, monday + 6 * 3600, 30 * 60) // Sector 1, start at 06:00 UTC, 30 min duration
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
            WaterSector::new(2, tuesday + 8 * 3600, 20 * 60) // Sector 3, start at 08:00 UTC, 20 min duration
        );

        // Test when no valid start times exist
        let current_time = chrono::Utc
            .with_ymd_and_hms(2024, 11, 27, 6, 0, 0) // Wednesday at 06:00 UTC
            .unwrap()
            .timestamp();

        assert!(schedule.get_next_auto_schedule(current_time).is_none());
    }

    #[test]
    fn handle_signal_pause_resume() {
        // let mock_db = Arc::new(MockDatabase::new());
        let sectors = vec![];
        let mut state_machine = WaterState::new(Some(ModeIdx::Wizard), sectors);
        state_machine.start_cycle(Cycle { id: 1, instructions: vec![WaterSector::new(1, 0, 30 * 3600)] });

        state_machine.handle_environmental_signal(EnvironmentalSignal::RainStart);
        assert_eq!(state_machine.state, WateringState::Idle);

        state_machine.handle_environmental_signal(EnvironmentalSignal::RainStop);
        assert!(state_machine.cycle.is_some());
    }

    #[test]
    fn daily_et_adjustment() {
        let mut sectors =
            vec![SectorInfo::build(1, 2.5, 1., 30 * 60, 1.5, 0.5), SectorInfo::build(2, 1.8, 0.8, 20 * 60, 0.5, 0.6)];

        // Mock ET value
        let daily_et = 0.3;

        WateringSchedule::adjust_progress_for_et_and_rain(
            &mut sectors.iter_mut().collect::<Vec<&mut SectorInfo>>(),
            daily_et,
            0.,
        );

        // Assert progress adjustments
        assert_eq!(sectors[0].progress, 1.2); // Reduced by 0.3
        assert_eq!(sectors[1].progress, 0.2); // Reduced by 0.3 but clamped to 0.2
    }

    #[test]
    fn recalculate_weekly_plan() {
        let sectors = vec![
            SectorInfo::build(1, 2.5, 1.0, 30 * 60, 1.0, 0.5), // Sector 1
            SectorInfo::build(2, 1.8, 0.8, 20 * 60, 0.5, 0.6), // Sector 2
        ];

        let current_time = chrono::Utc::now().timestamp();
        let ref_time = sod(current_time);
        let remaining_days = 4; // Assume 4 days left in the week
        let daily_et = 0.2; // Example daily evapotranspiration value
        let timeframe = AllowedTimeframe::new(22, 8);

        let plan =
            WateringSchedule::recalculate_remaining_plan(&sectors, remaining_days, daily_et, current_time, timeframe);

        // Assert that the plan contains entries for each remaining day
        assert_eq!(plan.entries.len(), remaining_days);

        // Assert that each day's plan contains valid sector schedules
        for entry in &plan.entries {
            assert!(!entry.start_times.is_empty()); // Ensure start times exist
            for sec in &entry.start_times {
                assert!(sectors.iter().any(|s| s.id == sec.id)); // Ensure valid sector IDs
                assert!(sec.start >= ref_time); // Ensure start times are valid UTC timestamps
                assert!(sec.duration > 0); // Ensure valid durations
            }
        }

        // Additional check: Ensure total durations are distributed across days
        let total_duration: i64 =
            plan.entries.iter().flat_map(|entry| entry.start_times.iter().map(|sec| sec.duration)).sum();

        assert!(total_duration > 0, "Total watering duration should be positive across the week");

        // Verify that no single day's plan exceeds the allowed watering timeframe
        for entry in &plan.entries {
            let daily_duration: i64 = entry.start_times.iter().map(|sec| sec.duration).sum();
            assert!(
                daily_duration <= timeframe.duration,
                "Daily watering duration {} exceeds allowed timeframe duration {}",
                daily_duration,
                timeframe.duration
            );
        }
    }
}
