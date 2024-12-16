use super::{
    ds::{CtrlSignal, Cycle, SectorInfo, WaterSector, WeatherSignal},
    modes::*,
    water_window::WaterWin,
    watering_alg::*,
};
use crate::{
    db::DatabaseTrait,
    sensors::interface::SensorController,
    utils::{ux_ts_to_string, get_week_day_from_ts, load_sectors_into_hashmap},
    watering::ds::WateringEvent,
};
use chrono::Weekday;
use std::fmt::Debug;
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info, trace};

#[derive(Debug, Clone, PartialEq)]
pub struct PausedData {
    pub state: Box<SMState>,
    pub signals: Vec<WeatherSignal>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum SMState {
    #[default]
    Idle,
    Watering(WaterSector),
    Paused(PausedData),
}

impl SMState {
    pub fn is_watering(&self) -> bool {
        matches!(self, SMState::Watering(_))
    }

    pub fn is_idle(&self) -> bool {
        *self == SMState::Idle
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, SMState::Paused(_))
    }

    pub fn boxed(&self) -> Box<SMState> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct StateMachine {
    pub controller: Arc<dyn SensorController>,
    pub db: Arc<dyn DatabaseTrait>,
    pub sectors: HashMap<u32, SectorInfo>,
    pub timeframe: WaterWin,

    pub state: SMState,
    pub current_mode: Mode,

    pub cycle: Option<Cycle>,

    pub mode_manual: ModeManual,
    pub mode_auto: ModeAuto,
    pub mode_wizard: ModeWizard,
}

impl StateMachine {
    pub fn new(
        controller: Arc<dyn SensorController>, starting_mode: Option<Mode>, sectors: Vec<SectorInfo>,
        current_time: i64, db: Arc<dyn DatabaseTrait>,
    ) -> Self {
        // TODO - have to recalculate/load the schedule on init
        Self {
            state: SMState::Idle,
            sectors: load_sectors_into_hashmap(sectors),
            current_mode: starting_mode.unwrap_or(Mode::Auto),
            timeframe: WaterWin::new(current_time, 22, 8),
            controller,
            db,
            mode_manual: ModeManual,
            mode_auto: ModeAuto { daily_plan: Vec::with_capacity(2) },
            mode_wizard: ModeWizard { daily_plan: Vec::with_capacity(2) },
            cycle: None,
        }
    }

    // Update the machine on every time tick
    pub fn update(&mut self, current_time: i64) {
        self.timeframe.roll_window(current_time);
        match self.state {
            SMState::Watering(sec) => {
                trace!("Watering sector {}", sec.id);
                if current_time >= sec.start + sec.duration {
                    self.deactivate_sector(current_time, sec);
                    if let Some(next_sec) = self.cycle.as_mut().and_then(|cycle| cycle.next_sector()) {
                        self.activate_sector(next_sec);
                    } else {
                        info!("Cycle completed. Returning to Idle state.");
                        self.stop();
                    }
                } else {
                    self.update_active_sector(sec, current_time);
                }
            }
            SMState::Idle if matches!(self.current_mode, Mode::Wizard | Mode::Auto) => {
                self.trans_watering(current_time)
            }
            _ => trace!("Update ignored in current state."),
        }
    }

    pub fn trans_watering(&mut self, current_time: i64) {
        let daily_plan = match self.current_mode {
            Mode::Auto => &self.mode_auto.daily_plan,
            Mode::Wizard => &self.mode_wizard.daily_plan,
            _ => unreachable!(),
        };
        if !daily_plan.is_empty() {
            trace!("{} mode schedule {:?}", self.current_mode, daily_plan);
            if let Some(mut cycle) = daily_plan.first().unwrap().get_cycle(current_time) {
                info!(
                    "{} Mode: Starting watering cycle at {}.",
                    self.current_mode,
                    ux_ts_to_string(cycle.get_start_unchecked())
                );

                if let Some(sec) = cycle.next_sector() {
                    self.cycle = Some(cycle);
                    self.activate_sector(sec);
                }
            }
        }
    }

    fn activate_sector(&mut self, sec: WaterSector) {
        self.state = SMState::Watering(sec);
        // we know that we have one sector at least, otherwise next_sector returns None
        if let Err(e) = self.controller.activate_sector(sec.id) {
            error!("Failed to activate sector {}: {}", sec.id, e);
        } else {
            info!("Moving to sector: {}", sec.id);
        }
    }

    fn deactivate_sector(&mut self, current_time: i64, sec: WaterSector) {
        self.sectors.get_mut(&sec.id).unwrap().last_water = current_time;
        if let Err(e) = self.controller.deactivate_sector(sec.id) {
            println!("Failed to deactivate sector {}: {}", sec.id, e);
        };
    }

    fn update_active_sector(&mut self, sec: WaterSector, current_time: i64) {
        let elapsed_secs = (current_time - sec.start) as f64;

        let sector = self.sectors.get_mut(&sec.id).unwrap();
        if elapsed_secs >= sec.duration as f64 {
            info!("Completed watering for sector {}", sector.id);
            let water_applied = (elapsed_secs / 3600.0) * sector.sprinkler_debit; // Final water applied

            _ = self.db.log_watering_event(WateringEvent::new(None, sec, water_applied, self.current_mode));
            return;
        }
        sector.progress += (1.0 / 3600.0) * sector.sprinkler_debit;
        trace!("Sector {} watering progress: {:.2} cm", sector.id, sector.progress);
    }

    pub fn trans_pause(&mut self, signal: WeatherSignal, current_time: i64) {
        if self.current_mode != Mode::Wizard {
            trace!("Pause not applicable in the current mode: {:?}", self.current_mode);
            return;
        }
        match &mut self.state {
            SMState::Watering(sec) => {
                let sec_clone = *sec;
                self.deactivate_sector(current_time, sec_clone);
                info!("Sector {} deactivated due to pause signal {:?}", sec_clone.id, signal);
                let paused_data = PausedData { state: self.state.boxed(), signals: vec![signal] };
                self.state = SMState::Paused(paused_data);
            }
            SMState::Paused(data) => {
                if data.signals.iter().all(|existing_signal| *existing_signal != signal) {
                    data.signals.push(signal);
                }
            }
            _ => (), //nop
        }
    }

    pub fn stop(&mut self) {
        self.cycle = None;
        match self.current_mode {
            Mode::Auto => {
                self.mode_auto.daily_plan.remove(0);
            } // we have only 2 cycles per day, max, so remove/shifting 1 element is ok
            Mode::Wizard => {
                self.mode_wizard.daily_plan.remove(0);
            } // we have only 2 cycles per day, max, so remove/shifting 1 element is ok
            _ => (),
        }
        self.state = SMState::Idle;
    }

    pub fn trans_resume(&mut self, env_signal: WeatherSignal, current_time: i64) {
        if !matches!(env_signal, WeatherSignal::LowWind | WeatherSignal::RainStop) {
            return; // Ignore irrelevant signals early
        }

        if let SMState::Paused(data) = &mut self.state {
            if data.signals.len() == 1 {
                self.state = std::mem::replace(&mut data.state, SMState::Idle);

                if self.timeframe.is_within(current_time) {
                    info!("Resuming paused watering");
                    let cycle = self.cycle.as_ref().unwrap();
                    let sec = cycle.daily_plan.0[cycle.curr_sector];
                    self.activate_sector(sec);
                    
                } else {
                    self.stop();
                }
            } else {
                data.signals.retain(|signal| signal != &env_signal);
            }
        }
    }

    pub fn trans_change_mode(&mut self, new_mode: Mode) {
        if new_mode != self.current_mode {
            //TODO  -
            info!("Changing mode from {:?} to {:?}", self.current_mode, new_mode);
            self.current_mode = new_mode;
        }
    }

    pub fn handle_signal(&mut self, signal: CtrlSignal, current_time: i64) {
        match (&mut self.state, signal) {
            // Idle state
            (SMState::Idle, CtrlSignal::ChgMode(new_mode)) => self.trans_change_mode(new_mode),
            (SMState::Idle, CtrlSignal::Weather(_)) => {}
            (SMState::Idle, CtrlSignal::StopMachine) => {}
            // Watering State
            (SMState::Watering(_), CtrlSignal::ChgMode(new_mode)) => self.trans_change_mode(new_mode),
            (SMState::Watering(_), CtrlSignal::Weather(env_signal)) => self.trans_pause(env_signal, current_time),
            (SMState::Watering(_), CtrlSignal::StopMachine) => self.trans_change_mode(Mode::Manual),
            // Paused State
            (SMState::Paused(_), CtrlSignal::ChgMode(new_mode)) => self.trans_change_mode(new_mode),
            (SMState::Paused(_), CtrlSignal::Weather(env_signal)) => self.trans_resume(env_signal, current_time),
            (SMState::Paused(_), CtrlSignal::StopMachine) => self.trans_change_mode(Mode::Manual),
            _ => {}
        }
    }

    pub fn do_daily_adjustments(&mut self, current_time: i64, daily_et: f64, daily_rain: f64) {
        let weekday = get_week_day_from_ts(current_time);
        let new_week = weekday == Weekday::Mon;
        if new_week {
            info!("New week.")
        }
        // 1. Adjust progress for each sector
        adjust_daily_sector_progress(
            &mut self.sectors.values_mut().collect::<Vec<_>>(),
            daily_et,
            daily_rain,
            new_week,
        );

        // 2. Recalculate the remaining weekly plan
        let secs_clone = &self.sectors.values().cloned().collect::<Vec<_>>();
        let daily_plan = calc_daily_plan(secs_clone, current_time, self.timeframe);
        trace!("{:?}", daily_plan);
        self.mode_wizard.daily_plan = daily_plan;
    }
}
