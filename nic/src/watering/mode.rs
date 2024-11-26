use std::sync::Arc;

use super::{
    interface::SensorController, mode_auto::ModeAuto, mode_manual::ModeManual,
    mode_wizard::ModeWizard, state_machine::WateringStateMachine,
};
use crate::db::Database;
use chrono::NaiveTime;

#[derive(Clone, Debug)]
pub enum ModeEnum {
    Manual(ModeManual),
    Auto(ModeAuto),
    Wizard(ModeWizard),
}

impl ModeEnum {
    pub async fn execute<C: SensorController>(
        &mut self,
        state_machine: &mut WateringStateMachine,
        db: Database,
        current_time: NaiveTime,
        controller: &Arc<C>,
    ) {
        match self {
            ModeEnum::Manual(mode) => mode.execute(state_machine, db, controller).await,
            ModeEnum::Auto(mode) => {
                mode.execute(state_machine, db, current_time, controller)
                    .await
            }
            ModeEnum::Wizard(mode) => {
                mode.execute(state_machine, db, current_time, controller)
                    .await
            }
        }
    }
}
