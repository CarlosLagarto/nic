use std::sync::Arc;

use super::{
    ds::{EnvironmentalSignal, SectorInfo, WateringState},
    interface::SensorController,
    schedule::AllowedTimeframe,
    state_machine::WateringStateMachine,
};
use crate::{db::Database, watering::ds::{Cycle, EventType}};
use chrono::{Duration, NaiveTime};

#[derive(Clone, Debug)]
pub struct ModeWizard {
    pub sectors: Vec<SectorInfo>,
    pub progress: std::collections::HashMap<u32, f64>,
    pub paused_state: Option<(WateringState, Cycle, usize)>, // Track paused state
    pub timeframe: AllowedTimeframe,
}

impl ModeWizard {
    pub fn new(sectors: Vec<SectorInfo>, timeframe: AllowedTimeframe) -> Self {
        // Initialize all progress to 0
        let progress = sectors.iter().map(|sector| (sector.id, 0.0)).collect();
        Self {
            sectors,
            progress,
            paused_state: None,
            timeframe,
        }
    }

    pub fn handle_signal(
        &mut self,
        signal: EnvironmentalSignal,
        state_machine: &mut WateringStateMachine,
    ) {
        match signal {
            EnvironmentalSignal::RainStart | EnvironmentalSignal::HighWind => {
                println!("Wizard Mode: Detected {:?}. Pausing irrigation.", signal);
                if self.paused_state.is_none() && state_machine.cycle.is_some() {
                    println!("Pausing irrigation due to {:?}", signal);
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
                    println!("Resuming irrigation after {:?}.", signal);

                    // Restore the saved state
                    state_machine.state = saved_state;
                    state_machine.cycle = Some(saved_cycle);
                    state_machine.current_instruction = saved_instruction;
                } else {
                    println!(
                        "Wizard Mode: No paused state to resume. Ignoring {:?}:?",
                        signal
                    );
                }
            }
        }
    }

    pub async fn update(&mut self, state_machine: &mut WateringStateMachine, db: &Database) {
        println!("WizardMode: Performing periodic updates...");

        // Check weather conditions and log any changes
        if !self.valid_weather_conditions(db.clone()) {
            println!("WizardMode: Weather conditions unsuitable for watering.");
            return;
        }

        // TODO: Recalculate schedules or adjust progress
        self.recalculate_progress(state_machine);
    }

    fn recalculate_progress(&mut self, _state_machine: &WateringStateMachine) {
        println!("WizardMode: Recalculating progress...");
        for sector in &self.sectors {
            if let Some(applied) = self.progress.get(&sector.id) {
                println!(
                    "Sector {} progress: {:.2} / {:.2} cm",
                    sector.id, applied, sector.weekly_target
                );
            }
        }
    }

    pub async fn execute<C: SensorController + 'static>(
        &mut self,
        state_machine: &mut WateringStateMachine,
        db: Database,
        current_time: NaiveTime,
        controller: &Arc<C>,
    ) {
        if !self.timeframe.is_within(current_time) {
            println!(
                "Wizard Mode: Current time ({:?}) is outside the allowed timeframe. Skipping watering.",
                current_time
            );
            return;
        }
        if !self.valid_weather_conditions(db.clone()) {
            println!("Unsuitable weather conditions. Skipping watering.");
            return;
        }
        println!("Wizard Mode: Dynamic schedule execution.");

        let sectors = self.sectors.clone();

        for sector in sectors {
            if state_machine.cycle.is_none() {
                if let Some(duration) = self.calculate_irrigation_time(&sector) {
                    println!(
                        "Wizard Mode: Watering Sector {} for {:?}.",
                        sector.id, duration
                    );

                    // Create a dynamic cycle for the sector
                    let cycle = Cycle {
                        id: 0, // Temporary cycle ID
                        instructions: vec![(sector.id, duration)],
                    };

                    state_machine.start_cycle(cycle);

                    self.update_progress(sector.id, duration, &sector);
                } else {
                    println!(
                        "Sector {} has already reached its weekly target. Skipping.",
                        sector.id
                    );
                }
            }
        }

        if state_machine.cycle.is_some() {
            state_machine.update(db.clone(),EventType::Wizard , controller).await;
        }
    }

    pub fn valid_weather_conditions(&self, db: Database) -> bool {
        // TODO:
        // Simulate a weather check
        // In practice, this might query a database or external API
        println!("Wizard Mode: Checking weather conditions...");

        // Example: Assume the weather conditions are stored in the database
        let weather_conditions = db.get_current_weather(); // Hypothetical method

        match weather_conditions {
            Some(weather) => {
                if weather.is_raining || weather.wind_speed > 20.0 {
                    println!(
                        "Wizard Mode: Unsuitable weather detected: Rain: {}, Wind: {}",
                        weather.is_raining, weather.wind_speed
                    );
                    false // Unsafe to water
                } else {
                    println!(
                        "Wizard Mode: Weather is suitable for watering: Rain: {}, Wind: {}",
                        weather.is_raining, weather.wind_speed
                    );
                    true // Safe to water
                }
            }
            None => {
                println!("Wizard Mode: No weather data available. Assuming safe to water.");
                true // Assume safe if no data is available
            }
        }
    }

    fn calculate_irrigation_time(&self, sector: &SectorInfo) -> Option<Duration> {
        let applied = self.progress.get(&sector.id).copied().unwrap_or(0.0);
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
    fn update_progress(&mut self, sector_id: u32, duration: Duration, sector: &SectorInfo) {
        let water_applied = (duration.num_seconds() as f64 * 60.0) * sector.sprinkler_debit;
        self.progress
            .entry(sector_id)
            .and_modify(|progress| *progress += water_applied);
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
        }
    }

    #[test]
    fn test_calculate_irrigation_time() {
        let sector = mock_sector(1, 2.5, 1.0, Duration::minutes(30));
        let timeframe = AllowedTimeframe {
            start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        };
        let wizard = ModeWizard::new(vec![sector.clone()], timeframe);

        // No progress yet
        let result = wizard.calculate_irrigation_time(&sector);
        assert_eq!(result, Some(Duration::minutes(30))); // 2.5 cm at 1.0 cm/hour
    }

    #[test]
    fn test_handle_signal_pause_resume() {
        let timeframe = AllowedTimeframe {
            start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        };
        let mut wizard = ModeWizard::new(vec![], timeframe.clone());

        let mut state_machine = WateringStateMachine::new(timeframe);
        state_machine.start_cycle(Cycle {
            id: 1,
            instructions: vec![(1, Duration::minutes(30))],
        });

        wizard.handle_signal(EnvironmentalSignal::RainStart, &mut state_machine);
        assert_eq!(state_machine.state, WateringState::Idle);

        wizard.handle_signal(EnvironmentalSignal::RainStop, &mut state_machine);
        assert!(state_machine.cycle.is_some());
    }
}
