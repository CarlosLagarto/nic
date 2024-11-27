use async_trait::async_trait;
use reqwest;
use tracing::{debug, error};

#[async_trait]
pub trait SensorController: Send + Sync {
    async fn activate_sector(&self, sector: u32);
    async fn deactivate_sector(&self, sector: u32);
}

pub struct RealSensorController;

#[async_trait]
impl SensorController for RealSensorController {
    async fn activate_sector(&self, sector: u32) {
        let url = format!("http://sensor-system/activate/{}", sector);
        if let Err(e) = reqwest::get(&url).await {
            error!("Failed to activate sector {}: {:?}", sector, e);
        } else {
            debug!("Sector {} activated successfully.", sector);
        }
    }

    async fn deactivate_sector(&self, sector: u32) {
        let url = format!("http://sensor-system/deactivate/{}", sector);
        if let Err(e) = reqwest::get(&url).await {
            error!("Failed to deactivate sector {}: {:?}", sector, e);
        } else {
            debug!("Sector {} deactivated successfully.", sector);
        }
    }
}
