use std::sync::Arc;

use super::{
    ds::{Cycle, EventType, WateringState},
    schedule::AllowedTimeframe,
    state_machine::WateringStateMachine,
};
use crate::{db::Database, sensors::interface::SensorController};
use chrono::NaiveTime;

#[derive(Clone, Debug)]
pub struct ModeAuto {
    cycle: Cycle,
    timeframe: AllowedTimeframe,
}

impl ModeAuto {
    pub fn new(cycle: Cycle, timeframe: AllowedTimeframe) -> Self {
        Self { cycle, timeframe }
    }

    pub async fn execute<C: SensorController + 'static>(
        &mut self,
        state_machine: &mut WateringStateMachine,
        db: Database,
        current_time: NaiveTime,
        controller: &Arc<C>,
    ) {
        if state_machine.state == WateringState::Idle {
            println!("Auto Mode: Machine is stopped. Skipping execution.");
            return;
        }
        if !self.timeframe.is_within(current_time) {
            println!(
                "Auto Mode: Current time is outside the allowed timeframe. Skipping watering."
            );
            return;
        }
        if state_machine.cycle.is_none() {
            println!("Auto Mode: Starting auto cycle.");
            state_machine.start_cycle(self.cycle.clone());
        }
        state_machine.update(db, EventType::Auto, controller).await;
    }
}
