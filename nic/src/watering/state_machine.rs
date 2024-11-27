use super::ds::{Cycle, WateringState};

pub struct WateringStateMachine {
    pub state: WateringState,
    pub cycle: Option<Cycle>,
    pub current_instruction: usize,
}

impl WateringStateMachine {
    pub fn new() -> Self {
        Self {
            state: WateringState::Idle,
            cycle: None,
            current_instruction: 0,
        }
    }

    pub fn start_cycle(&mut self, cycle: Cycle) {
        self.cycle = Some(cycle);
        self.current_instruction = 0;
        self.state = WateringState::Idle;
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, WateringState::Idle)
    }

    pub fn is_watering(&self) -> bool {
        matches!(self.state, WateringState::Watering(_, _))
    }
}
