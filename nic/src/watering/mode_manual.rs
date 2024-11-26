use std::sync::Arc;

use super::{
    ds::{Cycle, WateringState},
    interface::SensorController,
    state_machine::WateringStateMachine,
};
use crate::db::Database;

#[derive(Clone, Debug)]
pub struct ModeManual {
    cycle: Cycle,
}

impl ModeManual {
    pub fn new(cycle: Cycle) -> Self {
        Self { cycle }
    }

    pub async fn execute<C: SensorController>(
        &mut self,
        state_machine: &mut WateringStateMachine,
        db: Database,
        controller: &Arc<C>,
    ) {
        if state_machine.state == WateringState::Idle {
            println!("Manual Mode: Machine is stopped. Skipping execution.");
            return;
        }
        if state_machine.cycle.is_none() {
            println!("Manual Mode: Starting manual cycle.");
            state_machine.start_cycle(self.cycle.clone());
        }
        state_machine.update(db, "Manual", controller).await;
    }
}
