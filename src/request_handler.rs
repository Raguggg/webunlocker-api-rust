use log::{error, info, warn};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, USER_AGENT},
    Client, Proxy,
};

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use crate::{
    cookies_handler::{BaseCookiesHandler, CookieException},
    proxy_handler::ProxyHandler,
};

pub struct CookieManager {
    cookies: Arc<RwLock<HashMap<String, String>>>, // Using RwLock for cookies
}

impl CookieManager {
    // Constructor for initializing CookieManager
    pub fn new() -> Self {
        CookieManager {
            cookies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Method to get cookies (Read Access)
    pub async fn get_cookies(&self) -> HashMap<String, String> {
        let cookies = self.cookies.read().await; // Acquire read lock
        cookies.clone() // Return a clone of the cookies (safe)
    }

    // Method to set cookies (Write Access)
    pub async fn set_cookies(&self, new_cookies: HashMap<String, String>) {
        let mut cookies = self.cookies.write().await; // Acquire write lock
        *cookies = new_cookies; // Update cookies
    }
}
pub struct AsyncRequestHandler {
    cookies_handler: Option<Arc<dyn BaseCookiesHandler + Send + Sync>>,
    proxy_handler: Option<Arc<Mutex<dyn ProxyHandler + Send + Sync>>>, // Mutex inside Arc
    lock: Arc<Mutex<bool>>,
    headers: HeaderMap,
    cookies: Arc<CookieManager>,
}

impl AsyncRequestHandler {
    pub fn new(
        cookies_handler: Option<Arc<dyn BaseCookiesHandler + Send + Sync>>,
        proxy_handler: Option<Arc<Mutex<dyn ProxyHandler + Send + Sync>>>, // Updated to Arc<Mutex>
    ) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"),
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"),
        );

        AsyncRequestHandler {
            cookies_handler,
            proxy_handler,
            lock: Arc::new(Mutex::new(false)),
            headers,
            cookies: Arc::new(CookieManager::new()),
        }
    }

    pub async fn refresh(&self, url: &str) {
        // Acquire the lock
        let mut guard = match self.lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                println!(
                    "Task could not acquire the lock, it is already locked. {:?}",
                    url
                );
                return;
            }
        };
        println!("The task locked by {:?}", url);
        info!("Refreshing cookies.");
        let cookies_handler = self.cookies_handler.clone(); // Clone the handler for async operations
        let current_cookies = self.cookies.get_cookies().await;
        *guard = true;
        if let Some(cookies_handler) = cookies_handler {
            match cookies_handler.validate(&current_cookies).await {
                Ok(_) => {
                    println!("Cookies are valid.");
                    info!("Cookies validated successfully.");
                }
                Err(e) => {
                    warn!(
                        "Cookie validation failed, generating new cookies: {}",
                        e.message
                    );
                    println!("Cookies validation failed. Generating new cookies.");
                    match cookies_handler.generate().await {
                        Ok(new_cookies) => {
                            // Update shared cookies after re-acquiring the lock
                            self.cookies.set_cookies(new_cookies).await;
                            println!("New cookies generated and stored.");
                            info!("New cookies generated successfully.");
                        }
                        Err(e) => {
                            eprintln!("Failed to generate cookies: {}", e.message);
                            error!("Failed to generate cookies: {}", e.message);
                        }
                    }
                }
            }
        }
        *guard = false;
    }

    pub async fn make_request(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        println!("Starting request to URL: {}", url);
        info!("Starting request to URL: {}", url);

        let mut attempts = 0;
        let max_attempts = 3; // Define max attempts here for easier adjustments
        loop {
            let guard = self.lock.lock().await;
            // println!("processing after lock url: {:?}", url);
            drop(guard);
            attempts += 1;

            // Proxy setup logic
            let proxy_url = if let Some(ref handler) = self.proxy_handler {
                let handler = handler.lock().await; // Await the lock
                handler.get_proxy()
            } else {
                None
            };

            let proxy = match proxy_url {
                Some(ref url) => Proxy::all(url)?,
                None => Proxy::all("")?, // Default proxy (could be handled better)
            };

            let client = Client::builder().proxy(proxy).build()?;

            let mut headers = self.headers.clone();
            let cookie_string: String = self
                .cookies
                .get_cookies()
                .await
                .iter()
                .filter(|(key, _)| *key == "KP_UIDz-ssn" || *key == "KP_UIDz") // Dereference the `key` here
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<String>>()
                .join("; ");

            headers.insert(
                COOKIE,
                HeaderValue::from_str(&cookie_string).map_err(|e| CookieException {
                    message: format!("Failed to set cookie header: {}", e),
                })?,
            );

            let response = client.get(url).headers(headers).send().await?;

            match response.status().as_u16() {
                200 => {
                    let body = response.text().await?;
                    if body.is_empty() {
                        println!("The response is empty");
                        warn!("The response is empty");
                        self.refresh(&url).await;
                        continue;
                    }
                    return Ok(body);
                }
                429 => {
                    println!(
                        "Received status 429 (Too Many Requests). Retrying... {:?}",
                        url
                    );
                    warn!(
                        "Received status 429 (Too Many Requests). Retrying... {:?}",
                        url
                    );
                    self.refresh(&url).await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                403 => {
                    println!(
                        "Received status 403 (Forbidden). Trying to change proxy or other actions."
                    );
                    error!(
                        "Received status 403 (Forbidden). Trying to change proxy or other actions."
                    );

                    // You might want to add proxy removal or change logic here
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                _ => {
                    println!("Request failed with status: {}", response.status());
                    error!("Request failed with status: {}", response.status());
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }

            if attempts >= max_attempts {
                return Err("Failed to make successful request after 3 attempts".into());
            }
        }
    }
}
