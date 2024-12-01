use super::{mode_auto::ModeAuto, mode_manual::ModeManual, mode_wizard::ModeWizard, watering_system::WateringSystem};
use crate::{db::DatabaseTrait, sensors::interface::SensorController};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum ModeEnum {
    Manual(ModeManual),
    Auto(ModeAuto),
    Wizard(ModeWizard),
}

impl ModeEnum {
    pub async fn execute<C: SensorController + 'static, D: DatabaseTrait + 'static>(
        &mut self, water_sys: &mut WateringSystem<C>, db: &Arc<D>, current_time: i64,
    ) {
        match self {
            ModeEnum::Manual(mode) => mode.execute(water_sys, db).await,
            ModeEnum::Auto(mode) => mode.execute(water_sys, db, current_time).await,
            ModeEnum::Wizard(mode) => mode.execute(water_sys, current_time, db).await,
        }
    }
}
