use dotenv::dotenv;
use log::{error, warn};
use std::env;
use std::process;

#[derive(Debug)]
pub struct Config {
    pub proxy_username: String,
    pub proxy_password: String,
    pub proxy_host: String,
    pub proxy_port: String,
    pub zenrows_api_key: String,
    pub api_port: String,
    pub proxies_txt_file: String,
}

impl Config {
    // Function to load and validate environment variables
    pub fn load() -> Config {
        // Load the .env file only once at the start
        dotenv().ok();

        // Load and validate each environment variable
        let proxy_username = match env::var("PROXY_USERNAME") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                error!("PROXY_USERNAME is missing or empty");
                process::exit(1);
            }
        };

        let proxy_password = match env::var("PROXY_PASSWORD") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                error!("PROXY_PASSWORD is missing or empty");
                process::exit(1);
            }
        };

        let proxy_host = match env::var("PROXY_HOST") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                error!("PROXY_HOST is missing or empty");
                process::exit(1);
            }
        };

        let proxy_port = match env::var("PROXY_PORT") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                error!("PROXY_PORT is missing or empty");
                process::exit(1);
            }
        };

        let zenrows_api_key = match env::var("ZENROWS_API_KEY") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                error!("ZENROWS_API_KEY is missing or empty");
                process::exit(1);
            }
        };
        let api_port = match env::var("API_PORT") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                warn!("API start running on default port 5000");
                "5000".to_string()
            }
        };
        let proxies_txt_file = match env::var("PROXY_TEXT_FILE") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                warn!("API start running on default proxy text file");
                "proxies.txt".to_string()
            }
        };

        // Return the Config instance
        Config {
            proxy_username,
            proxy_password,
            proxy_host,
            proxy_port,
            zenrows_api_key,
            api_port,
            proxies_txt_file,
        }
    }
}

// fn main() {
//     // Load the environment variables and perform strict validation
//     let config = Config::load();

//     // Log loaded configuration values
//     info!("Proxy Username: {}", config.proxy_username);
//     info!("Proxy Password: {}", config.proxy_password);
//     info!("Proxy Host: {}", config.proxy_host);
//     info!("Proxy Port: {}", config.proxy_port);
//     info!("ZenRows API Key: {}", config.zenrows_api_key);
// }
