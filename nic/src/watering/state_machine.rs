use super::{
    ds::{CtrlSignal, Cycle, DailyPlan, SectorInfo, WaterSector, WeatherSignal},
    modes::*,
    water_window::WaterWin,
    watering_alg::*,
};
use crate::{
    config::Watering,
    db::DatabaseTrait,
    error::AppError,
    sensors::interface::SensorController,
    utils::{get_week_day_from_ts, load_sectors_into_hashmap, sod, ux_ts_to_string},
    watering::{ds::WateringEvent, SECS_TO_HOUR_CONV},
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

    // pub fn is_idle(&self) -> bool {
    //     *self == SMState::Idle
    // }

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

    pub auto_schedule: Schedule,

    pub mode_manual: ModeManual,
    pub mode_auto: ModeAuto,
    pub mode_wizard: ModeWizard,

    pub cfg: Watering,
}

impl StateMachine {
    pub fn new(
        controller: Arc<dyn SensorController>, starting_mode: Option<Mode>, sectors: Vec<SectorInfo>,
        current_time: i64, db: Arc<dyn DatabaseTrait>, cfg: Watering,
    ) -> Result<Self, AppError> {
        let auto_schedule = db.load_auto_schedule()?;
        let mode_auto = ModeAuto { daily_plan: load_auto_schedule(&auto_schedule, current_time) };
        Ok(Self {
            state: SMState::Idle,
            sectors: load_sectors_into_hashmap(sectors),
            current_mode: starting_mode.unwrap_or(Mode::Auto),
            timeframe: WaterWin::new(current_time, 22, 8),
            controller,
            db,
            auto_schedule,
            mode_manual: ModeManual,
            mode_auto,
            mode_wizard: ModeWizard { daily_plan: Vec::with_capacity(2) },
            cycle: None,
            cfg,
        })
    }

    // Update the machine on every time tick
    pub fn update(&mut self, current_time: i64) {
        self.timeframe.roll_window(current_time);
        match self.state {
            SMState::Watering(sec) => {
                trace!(sector_id = sec.id, "Watering sector.");
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
            SMState::Idle if self.is_auto_or_wizard() => self.trans_watering(current_time),
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
                    mode = ?self.current_mode,
                    cycle_start = ux_ts_to_string(cycle.get_start_unchecked()),
                    "Starting watering cycle.",
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
            info!(sector = sec.id, "Moving to sector.");
        }
    }

    fn deactivate_sector(&mut self, current_time: i64, sec: WaterSector) {
        self.sectors.get_mut(&sec.id).unwrap().last_water = current_time;
        if let Err(e) = self.controller.deactivate_sector(sec.id) {
            error!(sector_id=sec.id, error=?e,"Failed to deactivate sector");
        };
    }

    fn update_active_sector(&mut self, sec: WaterSector, current_time: i64) {
        let elapsed_secs = (current_time - sec.start) as f64;

        let sector = self.sectors.get_mut(&sec.id).unwrap();
        let sprinkler_debit_per_sec = SECS_TO_HOUR_CONV * sector.sprinkler_debit;
        if elapsed_secs >= sec.duration as f64 {
            info!(sector = sector.id, "Completed watering for sector.");
            let water_applied = elapsed_secs * sprinkler_debit_per_sec; // Final water applied

            _ = self.db.log_watering_event(WateringEvent::new(None, sec, water_applied, self.current_mode));
            return;
        }
        sector.progress += sprinkler_debit_per_sec;
        trace!("Sector {} watering progress: {:.2} cm", sector.id, sector.progress);
    }

    pub fn trans_pause(&mut self, signal: WeatherSignal, current_time: i64) {
        if self.current_mode != Mode::Wizard {
            trace!(mode=?self.current_mode,"Pause not applicable.");
            return;
        }
        match &mut self.state {
            SMState::Watering(sec) => {
                let sec_clone = *sec;
                self.deactivate_sector(current_time, sec_clone);
                info!(sector = sec_clone.id, signal = ?signal, "Sector deactivated due to pause signal");
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

    /// panics if mode daily plan don't have secs, or if called more times than the number of sectors
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
        if !matches!(env_signal, WeatherSignal::WindLow | WeatherSignal::RainStop) {
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
            info!(current_mode = ?self.current_mode, new_mode = ?new_mode, "Changing mode.");
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

        // 2. Recalculate the next day plan for wizard_mode, so we can switch at any time and the info is up to date
        let secs_clone = &self.sectors.values().cloned().collect::<Vec<_>>();
        self.mode_wizard.daily_plan = calc_wizard_daily_plan(
            secs_clone,
            current_time,
            self.timeframe,
            self.cfg.sector_transation_secs,
            self.cfg.min_watering_secs,
        );

        // 3. Recalculate the next day plan for auto_mode, so we can switch at any time and the info is up to date
        self.mode_auto.daily_plan = load_auto_schedule(&self.auto_schedule, current_time);
    }

    pub fn is_auto_or_wizard(&self) -> bool {
        matches!(self.current_mode, Mode::Auto | Mode::Wizard)
    }
}

fn load_auto_schedule(schedule: &Schedule, current_time: i64) -> Vec<DailyPlan> {
    let mut plans: Vec<DailyPlan> = Vec::with_capacity(2);

    let current_weekday = get_week_day_from_ts(current_time);
    let day_start = sod(current_time);

    for schedule in schedule.entries.iter() {
        if let ScheduleType::Weekday(weekday) = schedule.schedule_type {
            if weekday == current_weekday {
                let mut daily_plan = Vec::new();
                for sec in schedule.start_times.0.iter() {
                    daily_plan.push(WaterSector::new(sec.id, day_start + sec.start, sec.duration));
                }
                daily_plan.sort_by_key(|sector| sector.start); // Sort by start time
                plans.push(DailyPlan(daily_plan));
            }
        }
    }
    plans
}
