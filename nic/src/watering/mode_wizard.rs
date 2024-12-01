use super::{
    ds::{EnvironmentalSignal, WateringState},
    schedule::{Schedule, WateringSchedule},
    state_machine::WateringStateMachine,
    watering_system::WateringSystem,
};
use crate::{
    db::DatabaseTrait,
    sensors::interface::SensorController,
    watering::ds::{Cycle, EventType},
};
use chrono::Datelike;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone, Debug)]
pub struct ModeWizard {
    pub paused_state: Option<(WateringState, Cycle, usize)>, // Track paused state
    pub schedule: Schedule,                                  // Encapsulate scheduling logic in a reusable struct
}

impl ModeWizard {
    pub fn new(schedule: Schedule) -> Self {
        Self { paused_state: None, schedule }
    }

    pub fn handle_signal(&mut self, env_signal: EnvironmentalSignal, state_machine: &mut WateringStateMachine) {
        match env_signal {
            EnvironmentalSignal::RainStart | EnvironmentalSignal::HighWind => {
                if self.paused_state.is_none() && state_machine.cycle.is_some() {
                    info!("Wizard Mode: Detected {:?}. Pausing irrigation.", env_signal);
                    // Save the current state, cycle, and instruction index
                    self.paused_state = Some((
                        state_machine.state.clone(),
                        state_machine.cycle.clone().unwrap(),
                        state_machine.current_instruction,
                    ));

                    // Stop irrigation
                    state_machine.cycle = None;
                    state_machine.state = WateringState::Idle;
                }
            }
            EnvironmentalSignal::RainStop | EnvironmentalSignal::LowWind => {
                if let Some((saved_state, saved_cycle, saved_instruction)) = self.paused_state.take() {
                    info!("Resuming irrigation after {:?}.", env_signal);

                    // Restore the saved state
                    state_machine.state = saved_state;
                    state_machine.cycle = Some(saved_cycle);
                    state_machine.current_instruction = saved_instruction;
                } else {
                    debug!("Wizard Mode: No paused state to resume. Ignoring {:?}:?", env_signal);
                }
            }
        }
    }

    pub async fn execute<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &mut self, water_sys: &WateringSystem<C>, current_time: i64, db: &Arc<D>,
    ) {
        let sectors = water_sys.sectors.read().await;

        // Find today's scheduled sessions
        let today_plan = self.schedule.get_next_wizard_schedule(current_time);

        if today_plan.is_none() {
            debug!("Wizard Mode: No scheduled watering for today.");
            return;
        }

        let today_sessions = today_plan.unwrap();

        let mut cycle = Cycle {
            id: current_time as u32, // Use timestamp as a unique cycle ID
            instructions: vec![],
        };

        for (sector_id, start_time, duration) in today_sessions {
            if let Some(sector) = sectors.get(sector_id) {
                if current_time >= *start_time {
                    let irrigation_time = WateringSchedule::calculate_irrigation_time(sector).unwrap_or(*duration);
                    let actual_duration = irrigation_time.min(*duration).min(sector.max_duration);
                    info!("Wizard Mode: Adding watering for sector {} with duration {:?}.", sector.id, actual_duration);
                    // Add to the cycle
                    cycle.instructions.push((sector.id, actual_duration));
                }
            }
        }

        // Start the cycle in the state machine if there are valid instructions
        if !cycle.instructions.is_empty() {
            let mut sm = water_sys.state_machine.write().await;
            sm.start_cycle(cycle);
            info!("Wizard Mode: Started cycle with {} instructions.", sm.cycle.as_ref().unwrap().instructions.len());
            drop(sm); // because update will also need the state machine and we can't hold the write lock

            // Process the first instruction immediately
            water_sys.update(db, EventType::Wizard).await;
        } else {
            debug!("Wizard Mode: No valid instructions for the cycle.");
        }
    }

    pub async fn handle_daily_adjustments<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &mut self, water_sys: &WateringSystem<C>, db: &Arc<D>, current_time: i64,
    ) {
        debug!("WizardMode: Performing daily adjustments...");
        // 1. Calculate the daily evapotranspiration
        let daily_et = db.get_daily_et(current_time).unwrap_or(0.); // Default to 0.0 if no ET data available
        let rain = db.get_lastday_rain(current_time).unwrap_or(0.);

        // 2. Adjust progress for each sector
        let mut sectors = water_sys.sectors.write().await;
        WateringSchedule::adjust_progress_for_et_and_rain(
            &mut sectors.values_mut().collect::<Vec<_>>(),
            daily_et,
            rain,
        );

        // 3. Recalculate the remaining weekly plan
        let timeframe = *water_sys.timeframe.read().await; // Allowed watering timeframe
        let remaining_days = (7 - chrono::Local::now().weekday().num_days_from_monday()) as usize;
        self.schedule = WateringSchedule::recalculate_remaining_plan(
            &sectors.values().cloned().collect::<Vec<_>>(),
            remaining_days,
            daily_et,
            current_time,
            timeframe,
        );

        info!("Wizard Mode: Recalculated weekly plan after daily ET adjustment.");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        utils::sod,
        watering::{
            ds::SectorInfo,
            schedule::{AllowedTimeframe, ScheduleEntry, ScheduleType},
        },
    };

    #[test]
    fn test_handle_signal_pause_resume() {
        let schedule = Schedule::new(vec![]); // Create an empty schedule for the wizard mode
        let mut wizard = ModeWizard::new(schedule);

        let mut state_machine = WateringStateMachine::new();
        state_machine.start_cycle(Cycle { id: 1, instructions: vec![(1, 30 * 3600)] });

        wizard.handle_signal(EnvironmentalSignal::RainStart, &mut state_machine);
        assert_eq!(state_machine.state, WateringState::Idle);

        wizard.handle_signal(EnvironmentalSignal::RainStop, &mut state_machine);
        assert!(state_machine.cycle.is_some());
    }
    #[test]
    fn test_get_next_wizard_schedule() {
        // Create a schedule with specific dates
        let schedule_entries = vec![ScheduleEntry {
            schedule_type: ScheduleType::Date(sod(1692508800)), // Unix UTC timestamp for the start of the day
            start_times: vec![
                (1, 1692512400, 30 * 60), // Sector 1, starts at 02:00 UTC, duration 30 min
                (2, 1692519600, 20 * 60), // Sector 2, starts at 04:00 UTC, duration 20 min
                (3, 1692526800, 45 * 60), // Sector 3, starts at 06:00 UTC, duration 45 min
            ],
        }];

        let schedule = Schedule::new(schedule_entries);

        // Test for a specific day
        let current_time = 1692516000; // 03:00 UTC on the same day
        let result = schedule.get_next_wizard_schedule(current_time);

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            &vec![
                (1, 1692512400, 30 * 60), // Sector 1, starts at 02:00 UTC, duration 30 min
                (2, 1692519600, 20 * 60), // Sector 2, starts at 04:00 UTC, duration 20 min
                (3, 1692526800, 45 * 60), // Sector 3, starts at 06:00 UTC, duration 45 min
            ]
        );

        // Test for a day with no schedule
        let current_time = 1692595200; // A different day
        assert!(schedule.get_next_wizard_schedule(current_time).is_none());
    }

    #[tokio::test]
    async fn test_daily_et_adjustment() {
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

    #[tokio::test]
    async fn test_recalculate_weekly_plan() {
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
            for (sector_id, start_time, duration) in &entry.start_times {
                assert!(sectors.iter().any(|s| s.id == *sector_id)); // Ensure valid sector IDs
                assert!(*start_time >= ref_time); // Ensure start times are valid UTC timestamps
                assert!(*duration > 0); // Ensure valid durations
            }
        }

        // Additional check: Ensure total durations are distributed across days
        let total_duration: i64 =
            plan.entries.iter().flat_map(|entry| entry.start_times.iter().map(|(_, _, duration)| duration)).sum();

        assert!(total_duration > 0, "Total watering duration should be positive across the week");

        // Verify that no single day's plan exceeds the allowed watering timeframe
        for entry in &plan.entries {
            let daily_duration: i64 = entry.start_times.iter().map(|(_, _, duration)| duration).sum();
            assert!(
                daily_duration <= timeframe.duration,
                "Daily watering duration {} exceeds allowed timeframe duration {}",
                daily_duration,
                timeframe.duration
            );
        }
    }
}
