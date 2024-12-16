pub mod ds;
pub mod modes;
pub mod watering_alg;
#[allow(non_snake_case)]
pub mod state_machine;
pub mod watering_system;
pub mod water_window;

pub const SECTOR_TRANSITION_SECS: i64 = 20;
pub const MAX_DURATION_SECS: i64 = 1800; // 30 minutes
pub const DAILY_PERCOLATION_FACTOR: f64 = 0.1 * 24.;