use super::{
    ds::{AppState, CtrlSignal},
    modes::*,
    state_machine::*,
};
use crate::{
    api::{CycleResponse, WateringStateResponse},
    config::Watering,
    db::DatabaseTrait,
    error::AppError,
    sensors::interface::SensorController,
    time::TimeProvider,
    utils::sod,
};
use std::sync::Arc;
use tokio::sync::{
    broadcast::{Receiver, Sender},
    Mutex,
};
use tracing::info;

#[derive(Debug)]
pub struct WateringSystem {
    pub sm: StateMachine,
    pub controller: Arc<dyn SensorController>, // Sensor controller (mockable)
    pub time_provider: Arc<dyn TimeProvider>,  // Injected time provider
    pub db: Arc<dyn DatabaseTrait>,            // Injected db provider
    pub web_tx: Arc<Sender<CtrlSignal>>,
    pub sm_rx: Arc<Mutex<Receiver<CtrlSignal>>>,
}

impl WateringSystem {
    pub fn new(
        app_state: Arc<AppState>, starting_mode: Option<Mode>, current_time: i64, cfg: Watering,
    ) -> Result<Self, AppError> {
        let sectors = app_state.db.load_sectors()?;
        let state = StateMachine::new(
            app_state.sensors_ctrl.clone(),
            starting_mode,
            sectors,
            current_time,
            app_state.db.clone(),
            cfg,
        )?;
        Ok(WateringSystem {
            sm: state,
            db: app_state.db.clone(),
            controller: app_state.sensors_ctrl.clone(),
            time_provider: app_state.time_provider.clone(),
            web_tx: app_state.web_tx.clone(),
            sm_rx: app_state.sm_rx.clone(),
        })
    }

    async fn handle_control_signals(&mut self, current_time: i64) {
        if let Ok(signal) = self.sm_rx.lock().await.try_recv() {
            match signal {
                CtrlSignal::DevicesState(_x) => {} //TODO
                CtrlSignal::Weather(_) | CtrlSignal::StopMachine | CtrlSignal::ChgMode(_) => {
                    self.sm.handle_signal(signal, current_time)
                }
                CtrlSignal::GetCycle => {
                    let resp = self.get_cycle();
                    let _res = self.web_tx.send(CtrlSignal::GetCycleResponse(resp));
                }
                CtrlSignal::GetState => {
                    let resp = self.get_state();
                    let _res = self.web_tx.send(CtrlSignal::GetStateResponse(resp));
                }
                CtrlSignal::GenWeather(_x) => {} //TODO
                //the next arms are not needed
                _ => (),
                // ControlSignal::GetStateResponse(watering_state_response) => ()
                // ControlSignal::GetCycleResponse(cycle_response) => ()
            }
        }
    }

    fn do_daily_adjustments(&mut self, last_day: &mut i64, now: i64) {
        let day_start = sod(now);
        if *last_day == day_start {
            return; // Skip unnecessary processing if adjustments have already been made for today
        }

        *last_day = day_start;

        // Use default values directly in a single call to reduce redundant operations
        let (daily_et, daily_rain) =
            (self.db.get_daily_et(day_start).unwrap_or(0.0), self.db.get_lastday_rain(day_start).unwrap_or(0.0));

        self.sm.do_daily_adjustments(now, daily_et, daily_rain);
        info!(
            event = "daily_adjustments",
            daily_et = format!("{:.2}", daily_et),
            daily_rain = format!("{:.2}", daily_rain),
        );
    }

    pub fn get_state(&self) -> WateringStateResponse {
        let mode = self.sm.current_mode;

        let state = match &self.sm.state {
            SMState::Idle => "Idle".to_string(),
            SMState::Watering(sec) => {
                format!("Watering sector {} for {:.2} minutes", sec.id, sec.duration_minutes())
            }
            SMState::Paused(data) => match *data.state {
                SMState::Watering(ref sec) => format!("Paused sector {}", sec.id),
                _ => unreachable!(),
            },
        };
        let current_cycle =
            self.sm.cycle.as_ref().map(|cycle| format!("Cycle ID: {}, Instructions: {:?}", cycle.id, cycle.daily_plan));

        WateringStateResponse { error: None, mode: Some(mode.to_string()), state: Some(state), current_cycle }
    }

    pub fn get_cycle(&self) -> CycleResponse {
        CycleResponse {
            error: None,
            id: self.sm.cycle.as_ref().map(|cycle| cycle.id),
            instructions: self.sm.cycle.as_ref().map(|cycle| {
                cycle.daily_plan.0.iter().map(|sec| (sec.id, format!("{} minutes", sec.duration))).collect()
            }),
        }
    }
}

pub async fn run_watering_system(
    app_state: Arc<AppState>,
    starting_mode: Option<Mode>,
    stop_signal: tokio::sync::watch::Receiver<bool>,
    end_time: Option<i64>,           // Optional parameter for simulation
    ws: Option<&mut WateringSystem>, // Optional parameter for simulation
    cfg: Watering,
) -> Result<(), AppError> {
    let mut now = app_state.time_provider.now();
    let ws = if let Some(ws1) = ws { ws1 } else { &mut WateringSystem::new(app_state, starting_mode, now, cfg)? };

    let mut last_day = sod(now);
    let stop_signal = stop_signal; // Clone the receiver for use in the loop
    while end_time.map_or(true, |end| now < end) && !*stop_signal.borrow() {
        now = ws.time_provider.now();

        // in the fn we validate if it is a new day and a new week
        ws.do_daily_adjustments(&mut last_day, now);

        ws.handle_control_signals(now).await;

        ws.sm.update(now);

        ws.time_provider.advance_time(1).await;
    }
    info!("Ending watering system.");
    Ok(())
}
