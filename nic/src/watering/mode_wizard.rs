use super::{
    ds::{EnvironmentalSignal, SectorInfo, WateringState},
    schedule::AllowedTimeframe,
    state_machine::WateringStateMachine,
};
use crate::{db::Database, watering::ds::Cycle};
use chrono::{Duration, NaiveTime};

#[derive(Clone, Debug)]
pub struct ModeWizard {
    pub sectors: Vec<SectorInfo>,
    pub progress: std::collections::HashMap<u32, f64>,
    pub paused_state: Option<(WateringState, Cycle, usize)>, // Track paused state
    timeframe: AllowedTimeframe,
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
                println!("Wizard Mode: Detected rain. Pausing irrigation.");
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
            EnvironmentalSignal::RainStop => {
                if let Some((saved_state, saved_cycle, saved_instruction)) =
                    self.paused_state.take()
                {
                    println!("Resuming irrigation after rain.");

                    // Restore the saved state
                    state_machine.state = saved_state;
                    state_machine.cycle = Some(saved_cycle);
                    state_machine.current_instruction = saved_instruction;
                }
            }
        }
    }

    pub async fn update(&mut self, state_machine: &mut WateringStateMachine, db: &Database) {
        println!("WizardMode: Performing periodic updates...");

        // Check weather conditions and log any changes
        if !self.check_weather_conditions(db.clone()) {
            println!("WizardMode: Weather conditions unsuitable for watering.");
            return;
        }

        // Optional: Recalculate schedules or adjust progress
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

    pub async fn execute(
        &mut self,
        state_machine: &mut WateringStateMachine,
        db: Database,
        current_time: NaiveTime,
    ) {
        if state_machine.state == WateringState::Idle {
            println!("Wizard Mode: Machine is stopped. Skipping execution.");
            return;
        }
        if !self.timeframe.is_within(current_time) {
            println!(
                "Wizard Mode: Current time is outside the allowed timeframe. Skipping watering."
            );
            return;
        }
        if self.check_weather_conditions(db.clone()) {
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
                    state_machine.update(db.clone(), "Wizard").await;
                    self.update_progress(sector.id, duration, &sector);
                } else {
                    println!(
                        "Sector {} has already reached its weekly target. Skipping.",
                        sector.id
                    );
                }
            }
        }
    }

    pub fn check_weather_conditions(&self, db: Database) -> bool {
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
            let irrigation_time_minutes = (remaining / sector.sprinkler_debit) * 60.0;

            // Maximum time the soil can absorb water without runoff
            let max_percolation_time_minutes =
                (sector.percolation_rate / sector.sprinkler_debit) * 60.0;

            // Final duration is the minimum of required, percolation-limited, and max safe duration
            let irrigation_duration = Duration::minutes(irrigation_time_minutes.ceil() as i64);
            let percolation_duration =
                Duration::minutes(max_percolation_time_minutes.ceil() as i64);

            Some(
                irrigation_duration
                    .min(percolation_duration)
                    .min(sector.max_duration),
            )
        }
    }

    /// Update progress for a sector after watering
    fn update_progress(&mut self, sector_id: u32, duration: Duration, sector: &SectorInfo) {
        let water_applied = (duration.num_minutes() as f64 / 60.0) * sector.sprinkler_debit;
        self.progress
            .entry(sector_id)
            .and_modify(|progress| *progress += water_applied);
    }
}
