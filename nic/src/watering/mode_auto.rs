use std::sync::Arc;

use super::{
    ds::{Cycle, EventType},
    schedule::AllowedTimeframe,
    watering_system::WateringSystem,
};
use crate::{db::DatabaseTrait, sensors::interface::SensorController};
use chrono::NaiveTime;
use tracing::{debug, info};

#[derive(Clone, Debug)]
pub struct ModeAuto {
    pub cycle: Cycle,
}

impl ModeAuto {
    pub fn new(cycle: Cycle) -> Self {
        Self { cycle }
    }

    pub async fn execute<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &self,
        water_sys: &WateringSystem<C>,
        db: &Arc<D>,
        current_time: NaiveTime,
    ) {
        if water_sys.is_idle().await {
            info!("Auto Mode: Machine is stopped. Skipping execution.");
            return;
        }
        {
            let timeframe = water_sys.timeframe.read().await;
            if !timeframe.is_within(current_time) {
                debug!(
                    "Auto Mode: Current time is outside the allowed timeframe. Skipping watering."
                );
                return;
            }
        }
        {
            let mut sm = water_sys.state_machine.write().await;
            if sm.cycle.is_none() {
                info!("Auto Mode: Starting auto cycle.");
                sm.start_cycle(self.cycle.clone());
            }
        }
        water_sys.update(db, EventType::Auto).await;
    }

    // TODO
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::watering::schedule::AllowedTimeframe;
    #[test]
    fn test_calculate_next_start() {
        let timeframe = AllowedTimeframe {
            start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        };
        let auto_mode = ModeAuto::new(Cycle {
            id: 1,
            instructions: vec![],
        });

        // Test within timeframe
        let current_time = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert_eq!(
            auto_mode.calculate_next_start(current_time, timeframe),
            Some(current_time)
        );

        // Test before timeframe
        let current_time = NaiveTime::from_hms_opt(5, 0, 0).unwrap();
        assert_eq!(
            auto_mode.calculate_next_start(current_time, timeframe),
            Some(timeframe.start)
        );

        // Test after timeframe
        let current_time = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        assert_eq!(
            auto_mode.calculate_next_start(current_time, timeframe),
            None
        );
    }
}
