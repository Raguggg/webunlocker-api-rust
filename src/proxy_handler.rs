use rand::seq::SliceRandom;

use crate::config::Config;

// Proxy handler implementation
pub trait ProxyHandler: Send + Sync {
    fn get_proxy(&self) -> Option<String>;
    fn remove(&mut self, proxy: &str);
}

pub struct BrightDataRandomProxyHandler {
    proxies: Vec<String>,
}

impl BrightDataRandomProxyHandler {
    pub fn new(ips: Vec<String>) -> Self {
        let config = Config::load();
        let formatted_proxies = ips
            .into_iter()
            .map(|ip| {
                format!(
                    "http://{}-ip-{}:{}@{}:{}",
                    config.proxy_username,
                    ip,
                    config.proxy_password,
                    config.proxy_host,
                    config.proxy_port
                )
            })
            .collect();
        BrightDataRandomProxyHandler {
            proxies: formatted_proxies,
        }
    }
}

impl ProxyHandler for BrightDataRandomProxyHandler {
    fn get_proxy(&self) -> Option<String> {
        self.proxies.choose(&mut rand::thread_rng()).cloned()
    }

    fn remove(&mut self, proxy: &str) {
        if let Some(pos) = self.proxies.iter().position(|x| x == proxy) {
            self.proxies.remove(pos);
        }
    }
}

impl Clone for BrightDataRandomProxyHandler {
    fn clone(&self) -> Self {
        // Clone the vector of proxies
        BrightDataRandomProxyHandler {
            proxies: self.proxies.clone(),
        }
    }
}
