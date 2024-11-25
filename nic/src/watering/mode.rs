use super::{
    mode_auto::ModeAuto, mode_manual::ModeManual, mode_wizard::ModeWizard,
    state_machine::WateringStateMachine,
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
    pub async fn execute(
        &mut self,
        state_machine: &mut WateringStateMachine,
        db: Database,
        current_time: NaiveTime,
    ) {
        match self {
            ModeEnum::Manual(mode) => mode.execute(state_machine, db).await,
            ModeEnum::Auto(mode) => mode.execute(state_machine, db, current_time).await,
            ModeEnum::Wizard(mode) => mode.execute(state_machine, db, current_time).await,
        }
    }
}
