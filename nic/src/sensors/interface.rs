use reqwest;
use reqwest::blocking;
use tracing::debug;

use crate::error::AppError;

pub enum ControlMessage {
    Activate(u32),
    Deactivate(u32),
}

pub trait SensorController: Send + Sync {
    fn activate_sector(&self, sector: u32) -> Result<(), AppError>;
    fn deactivate_sector(&self, sector: u32) -> Result<(), AppError>;
}

pub struct RealSensorController;

impl SensorController for RealSensorController {
    fn activate_sector(&self, sector: u32) -> Result<(), AppError> {
        let url = format!("http://sensor-system/activate/{}", sector);
        let response = blocking::get(&url)?;
        if response.status().is_success() {
            debug!("Sector {} activated successfully.", sector);
            Ok(())
        } else {
            Err(AppError::SensorError(format!("Failed to activate sector {}: {:?}", sector, response.status())))
        }
    }

    fn deactivate_sector(&self, sector: u32) -> Result<(), AppError> {
        let url = format!("http://sensor-system/deactivate/{}", sector);
        let response = blocking::get(&url)?;
        if response.status().is_success() {
            debug!("Sector {} deactivated successfully.", sector);
            Ok(())
        } else {
            Err(AppError::SensorError(
                format!("Failed to deactivate sector {}: {:?}", sector, response.status()),
            ))
        }
    }
}
