use reqwest;

/// Activate a sector via HTTP call
pub async fn activate_sector(sector: u32) {
    let url = format!("http://sensor-system/activate/{}", sector);
    if let Err(e) = reqwest::get(&url).await {
        eprintln!("Failed to activate sector {}: {:?}", sector, e);
    } else {
        println!("Sector {} activated successfully.", sector);
    }
}

/// Deactivate a sector via HTTP call
pub async fn deactivate_sector(sector: u32) {
    let url = format!("http://sensor-system/deactivate/{}", sector);
    if let Err(e) = reqwest::get(&url).await {
        eprintln!("Failed to deactivate sector {}: {:?}", sector, e);
    } else {
        println!("Sector {} deactivated successfully.", sector);
    }
}