use nic::watering::ds::SectorInfo;
use chrono::Duration;

pub fn mock_sector(id: u32, weekly_target: f64, sprinkler_debit: f64, max_duration: Duration) -> SectorInfo {
    SectorInfo {
        id,
        weekly_target,
        sprinkler_debit,
        percolation_rate: 0.5, // Mock value
        max_duration,
        progress: 0.,
    }
}