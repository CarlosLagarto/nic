pub mod run_options;

use run_options::Args;
use serde::Deserialize;
use std::fs;

pub const CONFIG_FILE: &str = "./nic.toml";

#[derive(Debug, Deserialize)]
pub struct Database {
    pub name: String,
}

impl Default for Database {
    fn default() -> Self {
        Self { name: "nic.db".to_owned() }
    }
}

#[derive(Debug, Deserialize)]
pub struct WebServer {
    pub address: String,
}

impl Default for WebServer {
    fn default() -> Self {
        Self { address: "0.0.0.0:8080".to_owned() }
    }
}

#[derive(Debug, Deserialize)]
pub struct MQTT {
    pub address: String,
}

impl Default for MQTT {
    fn default() -> Self {
        Self { address: "0.0.0.0:8080".to_owned() }
    }
}

#[derive(Debug, Deserialize)]
pub struct WeatherStation {
    pub address: String,
}

impl Default for WeatherStation {
    fn default() -> Self {
        Self { address: "0.0.0.0:8080".to_owned() }
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct Watering {
    pub sector_transation_secs: i64,
    pub max_duration_secs: i64,
    pub min_watering_secs: i64,
}

impl Default for Watering {
    fn default() -> Self {
        Self { sector_transation_secs: 20, max_duration_secs: 1800, min_watering_secs: 300 }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: Database,
    pub web_server: WebServer,
    pub mqtt: MQTT,
    pub weather_station: WeatherStation,
    pub watering: Watering
}

impl Config {
    pub fn load(args: Args) -> Self {
        let config_content = fs::read_to_string(args.cfg_file).expect("Unable to read config file");
        let config: Config = toml::from_str(&config_content).expect("Unable to parse config");
        config
    }

    // test helper
    pub fn load_from_str(config_str: &str) -> Self {
        let config: Config = toml::from_str(config_str).expect("Unable to parse config");
        config
    }
}

#[cfg(test)]
pub mod tests {
    use crate::config::{
        run_options::{default_cfg_file, Args},
        Config,
    };

    #[test]
    fn load() {
        let cfg = default_cfg_file();
        println!("{:?}", Config::load(Args { cfg_file: cfg, cfg_str: None }));
    }
}
