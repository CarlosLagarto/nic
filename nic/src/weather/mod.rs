pub mod api;
pub mod mqtt_mon;

// TODO call the right function and math
pub fn calculate_et(temp: f64, humidity: f64, wind_speed: f64, solar_radiation: f64) -> f64 {
    // Example: Use the Penman-Monteith equation or another ET formula.
    // Simplified example:
    let net_radiation = solar_radiation * 0.408; // Convert radiation to mm/day equivalent
    let wind_factor = wind_speed * (1.5 - 0.25 * humidity); // Simplified wind adjustment
    let temp_factor = 0.0023 * temp * (temp + 17.8); // Temperature-driven factor

    net_radiation + wind_factor + temp_factor
}