use super::{
    ds::{Cycle, EnvironmentalSignal, SectorInfo},
    modes::*,
    schedule::{AllowedTimeframe, Schedule, WateringSchedule},
};
use crate::{
    utils::{display_from_ts, get_week_day_from_ts, load_sectors_into_hashmap, sod},
    watering::ds::{WaterSector, WateringState},
};
use std::collections::HashMap;
use tracing::{debug, info, trace};

pub enum StateMessage {
    Update(i64),
    SwitchMode(ModeIdx),
    Stop,
}

#[derive(Debug, Clone)]
pub struct WaterState {
    pub sectors: HashMap<u32, SectorInfo>,
    pub mode_wizard: ModeWizard,
    pub mode_auto: ModeAuto,
    pub mode_manual: ModeManual,
    pub active_mode: ModeIdx,
    pub active_sec: Option<WaterSector>,
    pub timeframe: AllowedTimeframe,
    pub state: WateringState,
    pub cycle: Option<Cycle>,
    pub current_instruction: usize,
}

impl WaterState {
    pub fn new(starting_mode: Option<ModeIdx>, sectors: Vec<SectorInfo>) -> Self {
        let timeframe = AllowedTimeframe::new(22, 8);

        let sectors = load_sectors_into_hashmap(sectors);

        let active_mode_index = if let Some(mode) = starting_mode { mode } else { ModeIdx::Auto };
        let mode_auto = ModeAuto::new(Cycle::default(), Schedule::new(vec![]));
        let mode_manual = ModeManual::new(Cycle::default());
        let mode_wizard = ModeWizard::new(Schedule::new(vec![]));

        Self {
            sectors,
            mode_auto,
            mode_manual,
            mode_wizard,
            active_mode: active_mode_index,
            active_sec: None,
            timeframe,
            state: WateringState::Idle,
            cycle: None,
            current_instruction: 0,
        }
    }

    pub fn switch_mode(&mut self, new_mode: ModeIdx) {
        self.active_mode = new_mode;
        debug!("Switching to {} Mode", new_mode);
    }

    pub fn handle_environmental_signal(&mut self, env_signal: EnvironmentalSignal) {
        match env_signal {
            EnvironmentalSignal::RainStart | EnvironmentalSignal::HighWind => {
                if self.mode_wizard.paused_state.is_none() && self.cycle.is_some() {
                    info!("Wizard Mode: Detected {:?}. Pausing irrigation.", env_signal);
                    // Save the current state, cycle, and instruction index
                    self.mode_wizard.paused_state =
                        Some((self.state.clone(), self.cycle.clone().unwrap(), self.current_instruction));
                    // Stop irrigation
                    self.set_idle();
                }
            }
            EnvironmentalSignal::RainStop | EnvironmentalSignal::LowWind => {
                // TODO we may need to check if it is in the acceptable timeframe
                if let Some((saved_state, saved_cycle, saved_instruction)) = self.mode_wizard.paused_state.take() {
                    info!("Resuming irrigation after {:?}.", env_signal);

                    // Restore the saved state
                    self.state = saved_state;
                    self.cycle = Some(saved_cycle);
                    self.current_instruction = saved_instruction;
                } else {
                    debug!("Wizard Mode: No paused state to resume. Ignoring {:?}:?", env_signal);
                }
            }
        }
    }

    pub fn handle_deactivating(&mut self, current_time: i64) {
        self.current_instruction += 1;

        // Check if there are more instructions in the current cycle
        if self.current_instruction < self.cycle.as_mut().unwrap().instructions.len() {
            self.state = WateringState::Idle; // Ready for the next sector
            info!("Preparing to start the next sector in the cycle.");
        } else {
            // Cycle completed; recalculate schedules
            info!("Cycle completed. Recalculating the next watering schedule...");
            match self.active_mode {
                ModeIdx::Wizard => {
                    info!("Recalculating schedule for Wizard mode...");
                    // Assume weekly recalculation for Wizard mode // TODO:
                    // Replace with daily evapotranspiration (ET) if available TODO:
                    self.set_idle();
                    self.recalculate_next_schedule(7, 0.0, current_time, self.timeframe);
                }
                ModeIdx::Auto => {
                    info!("Waiting for the next schedule in Auto mode...");
                    self.set_idle();
                }
                ModeIdx::Manual => {
                    info!("Manual mode: Remaining idle until user command.");
                    self.set_idle();
                }
            }
        }
    }

    pub fn recalculate_next_schedule(
        &mut self, total_days: usize, daily_et: f64, current_time: i64, timeframe: AllowedTimeframe,
    ) {
        let (schedule_type, mode_idx) = {
            match self.active_mode {
                ModeIdx::Wizard => {
                    info!("Recalculating Wizard schedule...");
                    (Some(self.mode_wizard.schedule.clone()), ModeIdx::Wizard)
                }
                ModeIdx::Auto => {
                    info!("Auto mode: Using pre-defined schedule. No recalculation needed.");
                    (None, ModeIdx::Auto)
                }
                ModeIdx::Manual => {
                    info!("Manual mode: No schedule changes. User commands dictate actions.");
                    (None, ModeIdx::Manual)
                }
            }
        };
        if mode_idx == ModeIdx::Wizard {
            if let Some(mut wizard_schedule) = schedule_type {
                // Perform schedule recalculation for Wizard mode
                let secs = self.sectors.values().cloned().collect::<Vec<_>>();
                let new_schedule =
                    WateringSchedule::recalculate_remaining_plan(&secs, total_days, daily_et, current_time, timeframe);
                wizard_schedule.entries = new_schedule.entries;

                self.mode_wizard.schedule = wizard_schedule;
            }
        }
    }

    pub fn do_daily_adjustments(&mut self, current_time: i64, daily_et: f64, daily_rain: f64) {
        debug!("WizardMode: Performing daily adjustments...");

        // 1. Adjust progress for each sector
        let secs_clone = &mut self.sectors.values_mut().collect::<Vec<_>>();
        WateringSchedule::adjust_progress_for_et_and_rain(secs_clone, daily_et, daily_rain);

        // 2. Recalculate the remaining weekly plan
        let secs_clone = &self.sectors.values().cloned().collect::<Vec<_>>();
        let remaining_days = (7 - get_week_day_from_ts(current_time).num_days_from_monday()) as usize;
        self.mode_wizard.schedule = WateringSchedule::recalculate_remaining_plan(
            secs_clone,
            remaining_days,
            daily_et,
            current_time,
            self.timeframe,
        );

        info!("Wizard Mode: Recalculated weekly plan after daily ET adjustment.");
    }

    fn prepare_deactivation(&mut self) {
        match self.state {
            WateringState::Activating(sec) | WateringState::Watering(sec) => {
                self.state = WateringState::Deactivating(sec)
            }
            _ => (),
        }
    }
    pub fn execute_wizard(&mut self, current_time: i64) {
        let base_time = sod(current_time); // Start of the current day
        if !self.timeframe.is_within(current_time, base_time) {
            trace!(
                "Wizard Mode: Current time {:?} is outside the allowed timeframe. Skipping execution.",
                display_from_ts(current_time)
            );
            self.prepare_deactivation();
            return;
        }

        // Find today's scheduled sessions
        let today_plan = self.mode_wizard.schedule.get_next_wizard_schedule(current_time);

        if today_plan.is_none() {
            trace!("Wizard Mode: No scheduled watering for today.");
            self.prepare_deactivation();
            return;
        }

        let today_sessions = today_plan.unwrap();

        let mut cycle = Cycle {
            id: current_time as u32, // Use timestamp as a unique cycle ID
            instructions: vec![],
        };
        match self.state {
            WateringState::Idle => {
                let mut acc = current_time;
                for sec in today_sessions {
                    if let Some(sector) = self.sectors.get(&sec.id) {
                        if sec.start >= current_time {
                            let irrigation_time =
                                WateringSchedule::calculate_irrigation_time(sector).unwrap_or(sec.duration);
                            let actual_duration = irrigation_time.min(sec.duration).min(sector.max_duration);
                            info!(
                                "Wizard Mode: Adding watering for sector {} with duration {:?}.",
                                sector.id, actual_duration
                            );
                            cycle.instructions.push(WaterSector::new(sector.id, acc, actual_duration));
                            acc += actual_duration + 20; //TODO - make a const
                        }
                    }
                }
                if !cycle.instructions.is_empty() {
                    self.start_cycle(cycle);
                    let cycle = self.cycle.as_ref().unwrap();
                    info!("Wizard Mode: Started new cycle with {} instructions.", cycle.instructions.len());
                    if let Some(sec) = cycle.instructions.get(self.current_instruction).cloned() {
                        if self.sectors.contains_key(&sec.id) {
                            self.state = WateringState::Activating(sec);
                        }
                    }
                }
            }

            WateringState::Activating(_sec) => {} //todo this shouldnt happen here}
            WateringState::Deactivating(_sec) => {} // this also shouldnt happen
            WateringState::Watering(sec) => {
                if current_time >= sec.start + sec.duration {
                    self.state = WateringState::Deactivating(sec);
                }
            }
        }
    }

    pub fn execute_auto(&mut self, current_time: i64) {
        debug!("Auto schedule {:?}", self.mode_auto.schedule);
        if let Some(sec) = self.mode_auto.schedule.get_next_auto_schedule(current_time) {
            if current_time >= sec.start && self.cycle.is_none() {
                info!("Auto Mode: Starting watering cycle at {:?} (local time).", display_from_ts(sec.start));
                let new_cycle = Cycle { id: current_time as u32, instructions: vec![sec] };
                self.start_cycle(new_cycle);
                self.state = WateringState::Activating(sec);
            }
        }
    }

    pub fn execute_manual(&mut self, current_time: i64) {
        if self.is_idle() {
            debug!("Manual Mode: Machine is stopped. Skipping execution.");
        }
        // TODO:  this is not right.  things missing to
        if self.cycle.is_none() {
            info!("Manual Mode: Starting auto cycle.");
            let cycle = Cycle {
                id: current_time as u32, // Use timestamp as a unique cycle ID
                instructions: vec![],
            };
            self.start_cycle(cycle);
        }
    }

    pub fn start_cycle(&mut self, cycle: Cycle) {
        self.cycle = Some(cycle);
        self.current_instruction = 0;
        self.state = WateringState::Idle;
    }

    pub fn set_idle(&mut self) {
        self.cycle = None;
        self.active_sec = None;
        self.state = WateringState::Idle;
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, WateringState::Idle)
    }

    pub fn is_activating(&self) -> bool {
        matches!(self.state, WateringState::Activating(_))
    }

    pub fn is_watering(&self) -> bool {
        matches!(self.state, WateringState::Watering(_))
    }

    pub fn is_deactivating(&self) -> bool {
        matches!(self.state, WateringState::Deactivating(_))
    }
}
