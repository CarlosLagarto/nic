use std::sync::Arc;

use super::{
    ds::{EnvironmentalSignal, SectorInfo, WateringState},
    schedule::AllowedTimeframe,
    state_machine::WateringStateMachine,
    watering_system::WateringSystem,
};
use crate::{
    db::DatabaseTrait,
    sensors::interface::SensorController,
    watering::ds::{Cycle, EventType},
};
use chrono::{Duration, NaiveTime};
use tracing::{debug, info};

#[derive(Clone, Debug)]
pub struct ModeWizard {
    pub paused_state: Option<(WateringState, Cycle, usize)>, // Track paused state
}

impl ModeWizard {
    pub fn new() -> Self {
        Self { paused_state: None }
    }

    // TODO: something is missing here.  we may need a cycle list
    pub fn calculate_next_start(
        &self,
        current_time: NaiveTime,
        timeframe: AllowedTimeframe,
    ) -> Option<NaiveTime> {
        // Example logic: Start as soon as the timeframe opens
        if timeframe.is_within(current_time) {
            Some(current_time) // Start immediately if within timeframe
        } else if current_time < timeframe.start {
            Some(timeframe.start) // Start when the timeframe opens
        } else {
            None // No valid start time today
        }
    }

    pub fn handle_signal(
        &mut self,
        env_signal: EnvironmentalSignal,
        state_machine: &mut WateringStateMachine,
    ) {
        match env_signal {
            EnvironmentalSignal::RainStart | EnvironmentalSignal::HighWind => {
                if self.paused_state.is_none() && state_machine.cycle.is_some() {
                    info!(
                        "Wizard Mode: Detected {:?}. Pausing irrigation.",
                        env_signal
                    );
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
                if let Some((saved_state, saved_cycle, saved_instruction)) =
                    self.paused_state.take()
                {
                    info!("Resuming irrigation after {:?}.", env_signal);

                    // Restore the saved state
                    state_machine.state = saved_state;
                    state_machine.cycle = Some(saved_cycle);
                    state_machine.current_instruction = saved_instruction;
                } else {
                    debug!(
                        "Wizard Mode: No paused state to resume. Ignoring {:?}:?",
                        env_signal
                    );
                }
            }
        }
    }

    pub async fn update<C: SensorController, D: DatabaseTrait + 'static>(
        &mut self,
        water_sys: &mut WateringSystem<C>,
        db: &Arc<D>,
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
            debug!(
                "Sector {} progress: {:.2} / {:.2} cm",
                sector.id, sector.progress, sector.weekly_target
            );
        }
    }

    pub async fn execute<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &mut self,
        water_sys: &WateringSystem<C>,
        current_time: NaiveTime,
        db: &Arc<D>,
    ) {
        if !water_sys.timeframe.read().await.is_within(current_time) {
            debug!(
                "Wizard Mode: Current time ({:?}) is outside the allowed timeframe. Skipping watering.",
                current_time
            );
            return;
        }
        if !self.valid_weather_conditions(db) {
            info!("Unsuitable weather conditions. Skipping watering.");
            return;
        }
        info!("Wizard Mode: Dynamic schedule execution.");

        // let sectors = water_sys.sectors.clone();
        let dont_have_cyle = {
            let sm = water_sys.state_machine.read().await;
            sm.cycle.is_none()
        };
        let secs = water_sys.sectors.read().await;
        for (id, sector) in secs.iter() {
            if dont_have_cyle {
                if let Some(duration) = self.calculate_irrigation_time(&sector) {
                    info!("Wizard Mode: Watering Sector {} for {:?}.", id, duration);

                    water_sys
                        .handle_activating(duration, db, EventType::Wizard, sector)
                        .await;
                } else {
                    info!(
                        "Sector {} has already reached its weekly target. Skipping.",
                        sector.id
                    );
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

    fn calculate_irrigation_time(&self, sector: &SectorInfo) -> Option<Duration> {
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
            let percolation_duration =
                Duration::minutes(max_percolation_time_seconds.ceil() as i64);

            Some(
                irrigation_duration
                    .min(percolation_duration)
                    .min(sector.max_duration),
            )
        }
    }

    /// Update progress for a sector after watering
    fn update_progress(&mut self, duration: Duration, sector: &mut SectorInfo) {
        let water_applied = (duration.num_seconds() as f64 * 60.0) * sector.sprinkler_debit;
        sector.progress += water_applied;
    }
}

#[cfg(test)]
mod mode_wizard_tests {
    use super::*;
    use chrono::{Duration, NaiveTime};

    fn mock_sector(
        id: u32,
        weekly_target: f64,
        sprinkler_debit: f64,
        max_duration: Duration,
    ) -> SectorInfo {
        SectorInfo {
            id,
            weekly_target,
            sprinkler_debit,
            percolation_rate: 0.5, // Mock value
            max_duration,
            progress: 0.,
        }
    }

    #[test]
    fn test_calculate_irrigation_time() {
        let sector = mock_sector(1, 2.5, 1.0, Duration::minutes(30));
        let wizard = ModeWizard::new();

        // No progress yet
        let result = wizard.calculate_irrigation_time(&sector);
        assert_eq!(result, Some(Duration::minutes(30))); // 2.5 cm at 1.0 cm/hour
    }

    #[test]
    fn test_handle_signal_pause_resume() {
        // let timeframe = AllowedTimeframe {
        //     start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        //     end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        // };
        let mut wizard = ModeWizard::new();

        let mut state_machine = WateringStateMachine::new();
        state_machine.start_cycle(Cycle {
            id: 1,
            instructions: vec![(1, Duration::minutes(30))],
        });

        wizard.handle_signal(EnvironmentalSignal::RainStart, &mut state_machine);
        assert_eq!(state_machine.state, WateringState::Idle);

        wizard.handle_signal(EnvironmentalSignal::RainStop, &mut state_machine);
        assert!(state_machine.cycle.is_some());
    }

    #[test]
    fn test_calculate_next_start() {
        let timeframe = AllowedTimeframe {
            start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        };
        let wizard_mode = ModeWizard::new();

        // Test within timeframe
        let current_time = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert_eq!(
            wizard_mode.calculate_next_start(current_time, timeframe),
            Some(current_time)
        );

        // Test before timeframe
        let current_time = NaiveTime::from_hms_opt(5, 0, 0).unwrap();
        assert_eq!(
            wizard_mode.calculate_next_start(current_time, timeframe),
            Some(timeframe.start)
        );

        // Test after timeframe
        let current_time = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        assert_eq!(
            wizard_mode.calculate_next_start(current_time, timeframe),
            None
        );
    }
}
