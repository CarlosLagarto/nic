use super::{
    ds::{AppState, ControlSignal, WaterSector, WateringState},
    modes::ModeIdx,
    water_state::WaterState,
};
use crate::{
    api::{CycleResponse, WateringStateResponse},
    db::DatabaseTrait,
    error::AppError,
    sensors::interface::SensorController,
    time::{advance_time, TimeProvider},
    utils::{display_from_ts, sod},
    watering::ds::WateringEvent,
};
use std::sync::Arc;
use tokio::sync::{
    broadcast::{Receiver, Sender},
    Mutex,
};
use tracing::{info, trace};

#[derive(Debug, Clone)]
pub struct WateringSystem<C, D, T>
where
    C: SensorController,
    D: DatabaseTrait,
    T: TimeProvider,
{
    pub water_state: WaterState,
    pub controller: Arc<C>,    // Sensor controller (mockable)
    pub time_provider: Arc<T>, // Injected time provider
    pub db: Arc<D>,            // Injected time provider
    pub tx: Arc<Sender<ControlSignal>>,
    pub rx: Arc<Mutex<Receiver<ControlSignal>>>,
}

impl<C, D, T> WateringSystem<C, D, T>
where
    C: SensorController + 'static,
    D: DatabaseTrait + 'static,
    T: TimeProvider + 'static,
{
    pub fn new(
        controller: Arc<C>, db: Arc<D>, time_provider: Arc<T>, starting_mode: Option<ModeIdx>,
        tx: Arc<Sender<ControlSignal>>, rx: Arc<Mutex<Receiver<ControlSignal>>>,
    ) -> Result<Self, AppError>
    where
        D: DatabaseTrait + 'static,
    {
        let sectors = db.load_sectors()?;
        let water_state = WaterState::new(starting_mode, sectors);
        Ok(WateringSystem { water_state, db, controller, time_provider, tx, rx })
    }

    async fn handle_control_signals(&mut self) {
        let signal_res = self.rx.lock().await.try_recv();
        if let Ok(signal) = signal_res {
            match signal {
                ControlSignal::SwitchToAuto => self.water_state.switch_mode(ModeIdx::Auto),
                ControlSignal::SwitchToManual => self.water_state.switch_mode(ModeIdx::Manual),
                ControlSignal::SwitchToWizard => self.water_state.switch_mode(ModeIdx::Wizard),
                ControlSignal::Environmental(env_signal) => {
                    if ModeIdx::Wizard == self.water_state.active_mode {
                        self.water_state.handle_environmental_signal(env_signal);
                    }
                }
                ControlSignal::StopMachine => self.water_state.set_idle(),
                ControlSignal::Weather(_weather) => {} //TODO:
                ControlSignal::DevicesState(_state) => {}
                ControlSignal::GetState => {
                    let res = self.get_state();
                    _ = self.tx.send(ControlSignal::GetStateResponse(res));
                }
                ControlSignal::GetCycle => {
                    let res = self.get_cycle();
                    _ = self.tx.send(ControlSignal::GetCycleResponse(res));
                }
                //do nothing for the other msgs
                ControlSignal::GetStateResponse(_) => (),
                ControlSignal::GetCycleResponse(_) => (),
            }
        }
    }

    pub async fn execute_active_mode(&mut self, current_time: i64) {
        match self.water_state.active_mode {
            ModeIdx::Wizard => {
                trace!("Wizard Mode: Executing dynamic schedule.");
                self.water_state.execute_wizard(current_time)
            }
            ModeIdx::Auto => {
                let base_time = sod(current_time); // Start of the current day
                if self.water_state.timeframe.is_within(current_time, base_time) {
                    trace!("Auto Mode: Executing auto schedule.");
                    self.water_state.execute_auto(current_time)
                }
            }
            ModeIdx::Manual => {
                info!("Manual Mode: No automatic scheduling.");
                self.water_state.execute_manual(current_time)
            }
        };

        match self.water_state.state {
            WateringState::Activating(sec) => {
                _ = self.controller.activate_sector(sec.id); // TODO
                self.water_state.state = WateringState::Watering(sec);
                info!("Activated sector {} at {} for {} seconds.", sec.id, display_from_ts(sec.start), sec.duration);
            }
            WateringState::Deactivating(sec) => {
                _ = self.controller.deactivate_sector(sec.id); // TODO
                info!("Deactivated sector {}.", sec.id);
                self.water_state.handle_deactivating(current_time);
            }
            WateringState::Watering(sec) => {
                self.update_active_sector(sec, current_time);
            }
            _ => (),
        }
    }

    fn do_daily_adjustments(&mut self, daily_adjustment_done: &mut Option<i64>, now: i64) {
        // Check if adjustments have already been made for the current day
        let day_start = sod(now); // Calculate start of the current day
        if daily_adjustment_done.map_or(true, |last_day| last_day != day_start) {
            *daily_adjustment_done = Some(day_start);
            let daily_et = self.db.get_daily_et(day_start).unwrap_or(0.); // Default to 0.0 if no ET data available
            let daily_rain = self.db.get_lastday_rain(day_start).unwrap_or(0.);
            self.water_state.do_daily_adjustments(now, daily_et, daily_rain);

            info!("Daily adjustments completed for {:?}", display_from_ts(day_start));
        }
    }

    pub fn get_state(&self) -> WateringStateResponse {
        let mode = self.water_state.active_mode;

        let state = match &self.water_state.state {
            WateringState::Idle => "Idle".to_string(),
            WateringState::Activating(sector) => format!("Activating sector {}", sector.id),
            WateringState::Watering(sec) => {
                format!("Watering sector {} for {:.2} minutes", sec.id, (sec.duration as f64 / 60.))
            }
            WateringState::Deactivating(sec) => format!("Deactivating sector {}", sec.id),
        };

        let current_cycle = self
            .water_state
            .cycle
            .as_ref()
            .map(|cycle| format!("Cycle ID: {}, Instructions: {:?}", cycle.id, cycle.instructions));

        WateringStateResponse { error: None, mode: Some(mode.to_string()), state: Some(state), current_cycle }
    }

    pub fn get_cycle(&self) -> CycleResponse {
        if let Some(cycle) = &self.water_state.cycle {
            let instructions =
                cycle.instructions.iter().map(|sec| (sec.id, format!("{} minutes", sec.duration))).collect();
            CycleResponse { error: None, id: Some(cycle.id), instructions: Some(instructions) }
        } else {
            CycleResponse { error: None, id: None, instructions: None }
        }
    }

    pub fn update_active_sector(&mut self, sec: WaterSector, current_time: i64) {
        let elapsed_secs = (current_time - sec.start) as f64;

        let sector = self.water_state.sectors.get_mut(&sec.id).unwrap();
        if elapsed_secs >= sec.duration as f64 {
            info!("Completed watering for sector {}", sector.id);
            let water_applied = (elapsed_secs / 3600.0) * sector.sprinkler_debit; // Final water applied

            _ = self.db.log_watering_event(WateringEvent::new(None, sec, water_applied, self.water_state.active_mode));
        }
        let incremental_progress = (1.0 / 3600.0) * sector.sprinkler_debit;

        sector.progress += incremental_progress;
        trace!("Sector {} watering progress: {:.2} cm", sector.id, sector.progress);
    }
}

pub async fn run_watering_system<C, D, T>(
    app_state: Arc<AppState<C, D, T>>,
    starting_mode: Option<ModeIdx>,
    end_time: Option<i64>,                    // Optional parameter for simulation
    ws: Option<&mut WateringSystem<C, D, T>>, // Optional parameter for simulation
) -> Result<(), AppError>
where
    C: SensorController + 'static,
    D: DatabaseTrait + 'static,
    T: TimeProvider + 'static,
{
    let ws = if let Some(ws1) = ws {
        ws1
    } else {
        &mut WateringSystem::new(
            app_state.sensors_ctrl.clone(),
            app_state.db.clone(),
            app_state.time_provider.clone(),
            starting_mode,
            app_state.tx.clone(),
            app_state.rx.clone(),
        )?
    };

    let mut daily_adjustment_done: Option<i64> = None;
    let mut now;

    loop {
        now = ws.time_provider.now();

        // Exit the loop if `end_time` is specified and reached
        if let Some(end_time) = end_time {
            if now >= end_time {
                info!("Ending watering system simulation at {:?}", now);
                break;
            }
        }
        // Handle daily adjustments at midnight
        if now >= sod(now) && daily_adjustment_done.map_or(true, |last_day| last_day != sod(now)) {
            daily_adjustment_done = Some(sod(now));
            ws.do_daily_adjustments(&mut daily_adjustment_done, now);
            info!("Performed daily adjustments.");
        }

        ws.handle_control_signals().await;

        ws.execute_active_mode(now).await;

        advance_time(&ws.time_provider).await;
    }
    Ok(())
}

#[cfg(test)]
mod test {}
