use rand::seq::SliceRandom;

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
        let formatted_proxies = ips
            .into_iter()
            .map(|ip| {
                format!(
                    "http://brd-customer-hl_3b0e466f-zone-datacenter_proxy2-ip-{}:xlovjulq950y@brd.superproxy.io:22225",
                    ip
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
