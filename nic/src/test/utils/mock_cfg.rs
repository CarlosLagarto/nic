use crate::config::Config;

pub fn mock_cfg() -> Config {
    let config_str = r#"[database]
                name = "watering_system.db"

                [web_server]
                address = "0.0.0.0:8080"

                [mqtt]
                address = ""

                [weather_station]
                address = ""

                [watering]
                sector_transation_secs = 20
                max_duration_secs = 1800
                min_watering_secs = 300
                "#;

    Config::load_from_str(config_str)
}
