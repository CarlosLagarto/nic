use std::sync::Arc;

use tracing::{debug, info};

use super::{
    ds::{Cycle, EventType},
    watering_system::WateringSystem,
};
use crate::{db::DatabaseTrait, sensors::interface::SensorController};

#[derive(Clone, Debug)]
pub struct ModeManual {
    cycle: Cycle,
}

impl ModeManual {
    pub fn new(cycle: Cycle) -> Self {
        Self { cycle }
    }

    pub async fn execute<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &mut self,
        water_sys: &mut WateringSystem<C>,
        db: &Arc<D>,
    ) {
        if water_sys.is_idle().await {
            debug!("Manual Mode: Machine is stopped. Skipping execution.");
            return;
        }
        {
            let mut sm = water_sys.state_machine.write().await;
            if sm.cycle.is_none() {
                info!("Auto Mode: Starting manual cycle.");
                sm.start_cycle(self.cycle.clone());
            }
        }
        water_sys.update(db, EventType::Manual).await;
    }
}
