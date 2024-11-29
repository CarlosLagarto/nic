use super::{
    ds::{EnvironmentalSignal, SectorInfo, WateringState},
    schedule::{AllowedTimeframe, Schedule, ScheduleEntry, WateringSchedule},
    state_machine::WateringStateMachine,
    watering_system::WateringSystem,
};
use crate::{
    db::DatabaseTrait,
    sensors::interface::SensorController,
    watering::ds::{Cycle, EventType},
    weather::calculate_et,
};
use chrono::{Datelike, Duration, NaiveTime};
use num_traits::FromPrimitive;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Clone, Debug)]
pub struct ModeWizard {
    pub paused_state: Option<(WateringState, Cycle, usize)>, // Track paused state
    pub schedule: Schedule,                                  // Encapsulate scheduling logic in a reusable struct
}

impl ModeWizard {
    pub fn new(schedule: Schedule) -> Self {
        Self { paused_state: None, schedule }
    }

    // TODO: something is missing here.  we may need a cycle list
    pub fn calculate_next_start(&self, current_time: NaiveTime, timeframe: AllowedTimeframe) -> Option<NaiveTime> {
        // Example logic: Start as soon as the timeframe opens
        if timeframe.is_within(current_time) {
            Some(current_time) // Start immediately if within timeframe
        } else if current_time < timeframe.start {
            Some(timeframe.start) // Start when the timeframe opens
        } else {
            None // No valid start time today
        }
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

    pub async fn update<C: SensorController, D: DatabaseTrait + 'static>(
        &mut self, water_sys: &mut WateringSystem<C>, db: &Arc<D>,
    ) {
        debug!("WizardMode: Performing periodic updates...");

        // Check weather conditions and log any changes
        if !self.valid_weather_conditions(db) {
            debug!("WizardMode: Weather conditions unsuitable for watering.");
            return;
        }

        // TODO: Recalculate schedules or adjust progress
        self.recalculate_progress(water_sys).await;
    }

    async fn recalculate_progress<C: SensorController>(&mut self, water_sys: &WateringSystem<C>) {
        debug!("WizardMode: Recalculating progress...");
        let secs = water_sys.sectors.read().await;
        for (_, sector) in secs.iter() {
            debug!("Sector {} progress: {:.2} / {:.2} cm", sector.id, sector.progress, sector.weekly_target);
        }
    }

    pub async fn execute<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &mut self, water_sys: &WateringSystem<C>, current_date: chrono::NaiveDate, db: &Arc<D>,
    ) {
        let sectors = water_sys.sectors.read().await;

        // Find today's scheduled sessions
        let today_plan = self.schedule.get_schedule_for_date(current_date);

        if today_plan.is_none() {
            debug!("Wizard Mode: No scheduled watering for today.");
            return;
        }

        let today_sessions = today_plan.unwrap();

        for (sector_id, duration) in today_sessions {
            if let Some(sector) = sectors.get(sector_id) {
                if let Some(irrigation_time) = self.calculate_irrigation_time(sector) {
                    info!("Wizard Mode: Executing watering for sector {} for {:?}.", sector.id, irrigation_time);

                    water_sys.handle_activating(irrigation_time.min(*duration), db, EventType::Wizard, sector).await;
                }
            }
        }

        let have_cyle = {
            let sm = water_sys.state_machine.read().await;
            sm.cycle.is_some()
        };
        if have_cyle {
            water_sys.update(db, EventType::Wizard).await;
        }
    }

    pub fn handle_rain_event(
        &mut self, rain_cm: f64, sectors: &mut [SectorInfo], timeframe: AllowedTimeframe, weekly_target: f64,
        daily_et: f64,
    ) {
        self.adjust_progress_for_rain(sectors, rain_cm);

        let remaining_days = 7 - chrono::Local::now().weekday().num_days_from_monday();
        let remaining_days = remaining_days as usize;

        // Recalculate the schedule based on remaining days
        let updated_entries =
            self.recalculate_remaining_schedule(sectors, timeframe.clone(), remaining_days, weekly_target, daily_et);

        // Update the schedule in the ModeWizard struct
        self.schedule = Schedule::new(updated_entries);

        info!("Wizard Mode: Recalculated weekly schedule after rain.");
    }

    fn recalculate_remaining_schedule(
        &self, sectors: &[SectorInfo], timeframe: AllowedTimeframe, remaining_days: usize, weekly_target: f64,
        daily_et: f64,
    ) -> Vec<ScheduleEntry> {
        let mut entries = Vec::new();

        // Distribute the watering sessions across remaining days
        for day in 0..remaining_days {
            // Calculate start times and sector durations for this day
            let distributed_starts = self.calculate_distributed_starts(
                sectors,
                timeframe.clone(),
                remaining_days as u32,
                weekly_target,
                daily_et,
            );

            // Flatten the results into a single list of sector durations
            let sector_durations: Vec<(u32, chrono::Duration)> = distributed_starts
                .into_iter()
                .flat_map(|(_, durations)| durations) // Extract the durations
                .collect();

            // Create a schedule entry
            entries.push(ScheduleEntry {
                day_of_week: chrono::Weekday::from_usize(day).unwrap(),
                start_times: sector_durations,
            });
        }

        entries
    }

    pub fn adjust_progress_for_et(&mut self, sectors: &mut [&mut SectorInfo], daily_et: f64) {
        for sector in sectors.iter_mut() {
            let evapotranspiration = daily_et.min(sector.progress); // Avoid negative progress
            sector.progress -= evapotranspiration;

            info!(
                "Sector {}: Adjusted progress by -{:.2} cm due to evapotranspiration. New progress: {:.2} cm.",
                sector.id, evapotranspiration, sector.progress
            );
        }
    }
    pub fn valid_weather_conditions<D: DatabaseTrait + 'static>(&self, db: &Arc<D>) -> bool {
        // TODO:
        // Simulate a weather check
        // In practice, this might query a database or external API
        debug!("Wizard Mode: Checking weather conditions...");

        // Example: Assume the weather conditions are stored in the database
        let weather_conditions = db.get_current_weather(); // Hypothetical method

        match weather_conditions {
            Some(weather) => {
                if weather.is_raining || weather.wind_speed > 20.0 {
                    info!(
                        "Wizard Mode: Unsuitable weather detected: Rain: {}, Wind: {}",
                        weather.is_raining, weather.wind_speed
                    );
                    false // Unsafe to water
                } else {
                    info!(
                        "Wizard Mode: Weather is suitable for watering: Rain: {}, Wind: {}",
                        weather.is_raining, weather.wind_speed
                    );
                    true // Safe to water
                }
            }
            None => {
                info!("Wizard Mode: No weather data available. Assuming safe to water.");
                true // Assume safe if no data is available
            }
        }
    }

    pub fn calculate_irrigation_time(&self, sector: &SectorInfo) -> Option<Duration> {
        let applied = sector.progress;
        let remaining = sector.weekly_target - applied;

        if remaining <= 0.0 {
            None // No watering needed; target met
        } else {
            // Time needed to apply the remaining water (in minutes)
            // remaining in cm / (debit in cm / hora)
            let irrigation_time_seconds = ((remaining / sector.sprinkler_debit) * 60.0) * 60.;

            // Maximum time the soil can absorb water without runoff
            // we convert the mm/hour to cm / hour
            let max_percolation_time_seconds =
                (((sector.percolation_rate * 10.) / sector.sprinkler_debit) * 60.0) * 60.;

            // Final duration is the minimum of required, percolation-limited, and max safe duration
            let irrigation_duration = Duration::seconds(irrigation_time_seconds.ceil() as i64);
            let percolation_duration = Duration::minutes(max_percolation_time_seconds.ceil() as i64);

            Some(irrigation_duration.min(percolation_duration).min(sector.max_duration))
        }
    }

    pub fn adjust_progress_for_rain(&mut self, sectors: &mut [SectorInfo], rain_cm: f64) {
        for sector in sectors.iter_mut() {
            // Reduce the target based on the rain's contribution
            let absorbed_rain = rain_cm.min(sector.weekly_target - sector.progress);
            sector.progress += absorbed_rain;
            info!(
                "Sector {}: Adjusted progress by {:.2} cm due to rain. New progress: {:.2} cm.",
                sector.id, absorbed_rain, sector.progress
            );
        }
    }

    pub fn recalculate_remaining_plan(
        &self, sectors: &[SectorInfo], timeframe: AllowedTimeframe, total_days: usize, daily_et: f64,
    ) -> Schedule {
        // Generate the weekly plan using the updated function signature
        let updated_entries = self.generate_weekly_plan(sectors, total_days, timeframe, daily_et);

        // Convert the generated weekly plan into a schedule format (ScheduleEntry)
        let schedule_entries: Vec<ScheduleEntry> = updated_entries
            .iter()
            .map(|(date, daily_plan)| ScheduleEntry { day_of_week: date.weekday(), start_times: daily_plan.clone() })
            .collect();

        Schedule::new(schedule_entries)
    }

    pub fn generate_weekly_plan(
        &self, sectors: &[SectorInfo], total_days: usize, timeframe: AllowedTimeframe, daily_et: f64,
    ) -> Vec<(chrono::NaiveDate, Vec<(u32, chrono::Duration)>)> {
        // Calculate the daily duration available for watering based on the timeframe and total days
        let daily_duration = WateringSchedule::calculate_daily_duration(timeframe, total_days);

        // Distribute sessions using the WateringSchedule logic
        let schedule = WateringSchedule::distribute_sessions(sectors, total_days, daily_duration, daily_et);

        // Convert the distributed sessions into a weekly plan with specific dates
        schedule
            .into_iter()
            .enumerate()
            .map(|(day, daily_plan)| {
                let date = chrono::Local::now().date_naive() + chrono::Duration::days(day as i64);
                (date, daily_plan)
            })
            .collect()
    }

    pub async fn handle_daily_adjustments<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &mut self, water_sys: &WateringSystem<C>, db: &Arc<D>,
    ) {
        debug!("WizardMode: Performing daily adjustments...");
        // 1. Calculate the daily evapotranspiration
        let daily_et = self.calculate_daily_evapotranspiration(&db);

        // 2. Adjust progress for each sector
        let mut sectors = water_sys.sectors.write().await;
        self.adjust_progress_for_et(&mut sectors.values_mut().collect::<Vec<_>>(), daily_et);

        // 3. Recalculate the remaining weekly plan
        let remaining_days = (7 - chrono::Local::now().weekday().num_days_from_monday()) as usize;
        self.schedule = self.recalculate_remaining_plan(
            &sectors.values().cloned().collect::<Vec<_>>(),
            *water_sys.timeframe.read().await,
            remaining_days,
            daily_et,
        );

        info!("Wizard Mode: Recalculated weekly plan after daily ET adjustment.");
    }

    pub fn calculate_daily_evapotranspiration<D: DatabaseTrait>(&self, db: &Arc<D>) -> f64 {
        // Fetch daily weather data (e.g., temperature, humidity, wind speed)
        if let Some(weather_data) = db.get_current_weather() {
            let et = calculate_et(
                weather_data.temperature,
                weather_data.humidity,
                weather_data.wind_speed,
                weather_data.solar_radiation,
            );
            info!("Calculated daily evapotranspiration: {:.2} cm", et);
            et
        } else {
            warn!("No weather data available for ET calculation. Assuming 0.");
            0.0
        }
    }

    pub fn calculate_distributed_starts(
        &self, sectors: &[SectorInfo], timeframe: AllowedTimeframe, days_remaining: u32, weekly_target: f64,
        daily_et: f64,
    ) -> Vec<(NaiveTime, Vec<(u32, chrono::Duration)>)> {
        let total_sectors = sectors.len();
        if total_sectors == 0 || days_remaining == 0 {
            return vec![]; // No sectors or no days remaining
        }

        let start = timeframe.start;
        let end = timeframe.end;
        let total_available_duration = (end - start).num_seconds().max(1); // Ensure we don't divide by zero

        // Calculate the total water needed per sector for the remaining days
        let water_needed_per_day = (weekly_target - daily_et * (7 - days_remaining) as f64)
            .max(0.0) // Ensure no negative values
            / days_remaining as f64;

        // Calculate watering duration for each sector based on the water needed and sprinkler debit
        let mut sector_durations: Vec<(u32, chrono::Duration)> = sectors
            .iter()
            .filter_map(|sector| {
                let water_needed = water_needed_per_day - sector.progress;
                if water_needed > 0.0 {
                    let duration_seconds = ((water_needed / sector.sprinkler_debit) * 3600.0).ceil() as i64;
                    let duration =
                        chrono::Duration::seconds(duration_seconds.min(sector.max_duration.num_seconds() as i64));
                    Some((sector.id, duration))
                } else {
                    None
                }
            })
            .collect();

        // Calculate the total watering duration for all sectors in a single cycle
        let total_cycle_duration: i64 = sector_durations.iter().map(|(_, d)| d.num_seconds()).sum();

        // Determine if two cycles are required
        let mut results = vec![];
        if total_cycle_duration > total_available_duration / 2 {
            // Two cycles required
            let half_cycle = chrono::Duration::seconds(total_available_duration / 2);

            // Schedule the first cycle at the start of the timeframe
            results.push((start, distribute_durations_within_cycle(half_cycle, &mut sector_durations)));

            // Schedule the second cycle so that it ends near the end of the timeframe
            let second_cycle_start = end - half_cycle;
            results.push((second_cycle_start, distribute_durations_within_cycle(half_cycle, &mut sector_durations)));
        } else {
            // Single cycle required
            results.push((
                start,
                distribute_durations_within_cycle(
                    chrono::Duration::seconds(total_cycle_duration),
                    &mut sector_durations,
                ),
            ));
        }

        results
    }

    pub fn calculate_deep_watering_schedule(
        &self, sectors: &[SectorInfo], timeframe: AllowedTimeframe, weekly_target: f64, daily_et: f64,
        days_remaining: u32,
    ) -> Vec<(NaiveTime, Vec<(u32, chrono::Duration)>)> {
        let total_sectors = sectors.len();
        if total_sectors == 0 || days_remaining == 0 {
            return vec![]; // No sectors or no days remaining
        }

        let start = timeframe.start;
        let end = timeframe.end;
        let total_available_duration = (end - start).num_seconds().max(1); // Ensure no divide-by-zero

        // Adjust each sector's remaining water needs based on ET and percolation
        let water_needed_per_day =
            (weekly_target - daily_et * (7 - days_remaining) as f64).max(0.0) / days_remaining as f64;

        let mut sector_durations: Vec<(u32, chrono::Duration)> = sectors
            .iter()
            .filter_map(|sector| {
                let adjusted_target = (water_needed_per_day - sector.percolation_rate).max(0.0);
                let remaining_water = adjusted_target - sector.progress;

                if remaining_water > 0.0 {
                    let duration_seconds = ((remaining_water / sector.sprinkler_debit) * 3600.0).ceil() as i64;
                    let duration =
                        chrono::Duration::seconds(duration_seconds.min(sector.max_duration.num_seconds() as i64));
                    Some((sector.id, duration))
                } else {
                    None
                }
            })
            .collect();

        // Sort sectors by their remaining water need (descending) for prioritization
        sector_durations.sort_by(|(_, d1), (_, d2)| d2.num_seconds().cmp(&d1.num_seconds()));

        let mut schedule = vec![];
        let mut remaining_duration = total_available_duration;

        // Allocate watering times across sectors
        let mut current_time = start;
        while remaining_duration > 0 && !sector_durations.is_empty() {
            let mut cycle_durations = vec![];
            let mut cycle_time_used = 0;

            while !sector_durations.is_empty() && cycle_time_used < remaining_duration {
                let (sector_id, duration) = sector_durations.remove(0);

                if cycle_time_used + duration.num_seconds() <= remaining_duration {
                    cycle_durations.push((sector_id, duration));
                    cycle_time_used += duration.num_seconds();
                } else {
                    // Partially water the sector
                    let partial_duration = chrono::Duration::seconds(remaining_duration - cycle_time_used);
                    cycle_durations.push((sector_id, partial_duration));
                    sector_durations.insert(0, (sector_id, duration - partial_duration));
                    break;
                }
            }

            schedule.push((current_time, cycle_durations));
            current_time = current_time + chrono::Duration::seconds(cycle_time_used);
            remaining_duration -= cycle_time_used;
        }

        schedule
    }
}

/// Helper function to distribute durations within a cycle
fn distribute_durations_within_cycle(
    cycle_duration: chrono::Duration, sector_durations: &mut Vec<(u32, chrono::Duration)>,
) -> Vec<(u32, chrono::Duration)> {
    let mut allocated = vec![];
    let mut remaining_cycle_duration = cycle_duration.num_seconds();

    while remaining_cycle_duration > 0 && !sector_durations.is_empty() {
        let (sector_id, duration) = sector_durations.remove(0);

        if duration.num_seconds() <= remaining_cycle_duration {
            allocated.push((sector_id, duration));
            remaining_cycle_duration -= duration.num_seconds();
        } else {
            let partial_duration = chrono::Duration::seconds(remaining_cycle_duration);
            allocated.push((sector_id, partial_duration));
            sector_durations.insert(0, (sector_id, duration - partial_duration));
            break;
        }
    }

    allocated
}
#[cfg(test)]
mod mode_wizard_tests {
    use super::*;
    use chrono::{Duration, NaiveTime};

    #[test]
    fn test_handle_signal_pause_resume() {
        let schedule = Schedule::new(vec![]); // Create an empty schedule for the wizard mode
        let mut wizard = ModeWizard::new(schedule);

        let mut state_machine = WateringStateMachine::new();
        state_machine.start_cycle(Cycle { id: 1, instructions: vec![(1, Duration::minutes(30))] });

        wizard.handle_signal(EnvironmentalSignal::RainStart, &mut state_machine);
        assert_eq!(state_machine.state, WateringState::Idle);

        wizard.handle_signal(EnvironmentalSignal::RainStop, &mut state_machine);
        assert!(state_machine.cycle.is_some());
    }
    #[test]
    fn test_calculate_next_start() {
        // Create a schedule for the wizard mode
        let schedule_entries = vec![ScheduleEntry {
            day_of_week: chrono::Weekday::Mon,
            start_times: vec![(1, Duration::minutes(30)), (1, Duration::minutes(30)), (1, Duration::minutes(30))],
        }];

        let schedule = Schedule::new(schedule_entries);
        let wizard_mode = ModeWizard::new(schedule);

        // Test within timeframe
        let current_time = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert_eq!(
            wizard_mode.schedule.get_next_start_time(current_time),
            Some(NaiveTime::from_hms_opt(8, 0, 0).unwrap())
        );

        // Test before timeframe
        let current_time = NaiveTime::from_hms_opt(5, 0, 0).unwrap();
        assert_eq!(
            wizard_mode.schedule.get_next_start_time(current_time),
            Some(NaiveTime::from_hms_opt(8, 0, 0).unwrap())
        );

        // Test after timeframe
        let current_time = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        assert_eq!(wizard_mode.schedule.get_next_start_time(current_time), None);
    }

    #[tokio::test]
    async fn test_daily_et_adjustment() {
        // Mock sectors
        let mut sectors = vec![
            SectorInfo {
                id: 1,
                sprinkler_debit: 1.0,
                percolation_rate: 0.5,
                max_duration: Duration::minutes(30),
                weekly_target: 2.5,
                progress: 1.5,
            },
            SectorInfo {
                id: 2,
                sprinkler_debit: 0.8,
                percolation_rate: 0.6,
                max_duration: Duration::minutes(20),
                weekly_target: 1.8,
                progress: 0.5,
            },
        ];

        // Mock ET value
        let daily_et = 0.3;

        let schedule = Schedule::new(vec![]); // Create an empty schedule for the wizard mode
        let mut wizard_mode = ModeWizard::new(schedule);
        wizard_mode.adjust_progress_for_et(&mut sectors.iter_mut().collect::<Vec<&mut SectorInfo>>(), daily_et);

        // Assert progress adjustments
        assert_eq!(sectors[0].progress, 1.2); // Reduced by 0.3
        assert_eq!(sectors[1].progress, 0.2); // Reduced by 0.3 but clamped to 0.2
    }
    #[tokio::test]
    async fn test_recalculate_weekly_plan() {
        let sectors = vec![
            SectorInfo {
                id: 1,
                sprinkler_debit: 1.0,  // cm/hour
                percolation_rate: 0.5, // mm/hour
                max_duration: Duration::minutes(30),
                weekly_target: 2.5,
                progress: 1.0, // Remaining: 1.5
            },
            SectorInfo {
                id: 2,
                sprinkler_debit: 0.8,  // cm/hour
                percolation_rate: 0.6, // mm/hour
                max_duration: Duration::minutes(20),
                weekly_target: 1.8,
                progress: 0.5, // Remaining: 1.3
            },
        ];

        let timeframe = AllowedTimeframe {
            start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
        };

        let remaining_days = 4; // Assume 4 days left in the week
        let daily_et = 0.2; // Example daily evapotranspiration value

        let schedule = Schedule::new(vec![]); // Create an empty schedule for the wizard mode
        let wizard_mode = ModeWizard::new(schedule);
        let plan = wizard_mode.recalculate_remaining_plan(&sectors, timeframe, remaining_days, daily_et);

        // Assert that the plan contains entries for each remaining day
        assert_eq!(plan.entries.len(), remaining_days);

        // Assert that each day's plan contains valid sector schedules
        for entry in &plan.entries {
            assert!(!entry.start_times.is_empty()); // Ensure start times exist
            for (sector_id, duration) in &entry.start_times {
                assert!(sectors.iter().any(|s| s.id == *sector_id)); // Ensure valid sector IDs
                assert!(duration.num_minutes() > 0); // Ensure valid durations
            }
        }

        // Additional check: Ensure total durations are distributed across days
        let total_duration: i64 = plan
            .entries
            .iter()
            .flat_map(|entry| entry.start_times.iter().map(|(_, duration)| duration.num_seconds()))
            .sum();

        assert!(total_duration > 0); // Ensure total duration is positive
    }
}
