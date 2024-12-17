pub mod ds;
pub mod modes;
pub mod watering_alg;
#[allow(non_snake_case)]
pub mod state_machine;
pub mod watering_system;
pub mod water_window;

pub const DAILY_PERCOLATION_FACTOR: f64 = 0.1 * 24.;
pub const SECS_TO_HOUR_CONV: f64 = 1. / 3600.0;