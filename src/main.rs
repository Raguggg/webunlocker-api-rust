use std::{collections::HashMap, sync::Arc};
use std::fs;
use rand::seq::SliceRandom;
use async_trait::async_trait;
use serde_derive::Deserialize;
use std::error::Error;
use std::fmt;
use reqwest::{header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, USER_AGENT}, Client, Proxy};
use reqwest::{Error as ReqwestError, Url};
use tokio::sync::Mutex;
use tokio::sync::RwLock; 


use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use env_logger;




#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // Load proxies from file
    let proxies = load_proxies("/home/ragu/Desktop/src/.config/proxies.txt");

    // Set up the ZenrowsCookiesHandler
    let cookie_url = "https://www.property.com.au/".to_string();
    let api_key = "b46ad9c3eaa157dcbc0a884ec2f1be72bce1d7a7".to_string();
    let premium_proxy = false;
    
    let zenrows_handler = ZenrowsCookiesHandler::new(cookie_url, api_key, premium_proxy, Some(Box::new(BrightDataRandomProxyHandler::new(proxies.clone()))));

    let zenrows_handler = Arc::new(zenrows_handler);
    let proxy_handler = Arc::new(Mutex::new(BrightDataRandomProxyHandler::new(proxies)));

    // Create the AsyncRequestHandler
    // let request_handler_api = Arc::new(Mutex::new(AsyncRequestHandler::new(Some(zenrows_handler), Some(proxy_handler))));
    let request_handler_api = Arc::new(RwLock::new(AsyncRequestHandler::new(
        Some(zenrows_handler),
        Some(proxy_handler),
    )));
    // Start the HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(request_handler_api.clone())) // Pass the Arc<Mutex<AsyncRequestHandler>>
            .route("/request", web::get().to(request_handler)) // Route all requests to the same handler
    })
    .bind("0.0.0.0:5000")?
    .workers(1)
    .run()
    .await
}

async fn request_handler(
    request_data: web::Query<RequestData>,
    // property_request_handler: web::Data<Arc<Mutex<AsyncRequestHandler>>>,
    property_request_handler: web::Data<Arc<RwLock<AsyncRequestHandler>>>,
) -> impl Responder {
    println!("{:?}", request_data);

    let allowed_domains = vec!["www.property.com.au"];
    let url = &request_data.url;
    
    // Parse the URL from the query string
    let parsed_url = match Url::parse(&url) {
        Ok(url) => url,
        Err(_) => {
            return HttpResponse::BadRequest().body("Invalid URL format");
        }
    };

    // Check if the domain is allowed
    if !allowed_domains.contains(&parsed_url.host_str().unwrap_or("")) {
        return HttpResponse::BadRequest().body("URL domain not allowed");
    }

    // Handle the request using the shared AsyncRequestHandler
    // let handler = property_request_handler.lock().await;
    let handler = property_request_handler.read().await;
    match handler.make_request(&parsed_url.to_string()).await {
        Ok(body) => HttpResponse::Ok().json(serde_json::json!({ "status_code": 200, "body": body })),
        Err(_) => HttpResponse::TooManyRequests().json(serde_json::json!({ "status_code": 429, "body": "" })),
    }
}

#[derive(Deserialize, Debug)] // Add the Debug derive here
struct RequestData {
    url: String,
}
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
    proxy_handler: Option<Arc<Mutex<dyn ProxyHandler + Send + Sync>>>,  // Mutex inside Arc
    lock: Arc<Mutex<bool>>,
    headers: HeaderMap,
    cookies: Arc<CookieManager>,
}

impl AsyncRequestHandler {
    pub fn new(
        cookies_handler: Option<Arc<dyn BaseCookiesHandler + Send + Sync>>,
        proxy_handler: Option<Arc<Mutex<dyn ProxyHandler + Send + Sync>>>,  // Updated to Arc<Mutex>
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
            lock:  Arc::new(Mutex::new(false)),
            headers,
            cookies: Arc::new(CookieManager::new()),
        }
    }

    pub async fn refresh(&self) {
        // Acquire the lock
        let mut guard = match self.lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                println!("Task 2 could not acquire the lock, it is already locked.");
                return;
            }
        };
    
        let cookies_handler = self.cookies_handler.clone(); // Clone the handler for async operations
        let current_cookies = self.cookies.get_cookies().await;
        *guard = true;
        if let Some(cookies_handler) = cookies_handler {
            match cookies_handler.validate(&current_cookies).await {
                Ok(_) => {
                    println!("Cookies are valid.");
                }
                Err(_) => {
                    println!("Cookies validation failed. Generating new cookies.");
                    match cookies_handler.generate().await {
                        Ok(new_cookies) => {
                            // Update shared cookies after re-acquiring the lock
                            self.cookies.set_cookies(new_cookies).await;
                            println!("New cookies generated and stored.");
                        }
                        Err(e) => {
                            eprintln!("Failed to generate cookies: {}", e.message);
                        }
                    }
                }
            }
        }
        *guard = false;
      
        
  
        
    }
    
    
    pub async fn make_request(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        println!("Starting request to URL: {}", url);
    
        let mut attempts = 0;
        let max_attempts = 3; // Define max attempts here for easier adjustments
        loop {
            let guard = self.lock.lock().await;
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
    
            let client = Client::builder()
                .proxy(proxy)
                .build()?;

            let mut headers = self.headers.clone();
            let cookie_string: String = self.cookies.get_cookies().await.iter()
                        .filter(|(key, _)| *key == "KP_UIDz-ssn" || *key == "KP_UIDz") // Dereference the `key` here
                        .map(|(key, value)| format!("{}={}", key, value))
                        .collect::<Vec<String>>()
                        .join("; ");


    
            headers.insert(COOKIE, HeaderValue::from_str(&cookie_string).map_err(|e| CookieException {
                message: format!("Failed to set cookie header: {}", e),
            })?);
            
            let response = client
                .get(url)
                .headers(headers)
                .send()
                .await?;
    
            match response.status().as_u16() {
                200 => {
                    let body = response.text().await?;
                    return Ok(body);
                }
                429 => {
                    println!("Received status 429 (Too Many Requests). Retrying...");
                    self.refresh().await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                403 => {
                    println!("Received status 403 (Forbidden). Trying to change proxy or other actions.");
                    // You might want to add proxy removal or change logic here
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                _ => {
                    println!("Request failed with status: {}", response.status());
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
    
            if attempts >= max_attempts {
                return Err("Failed to make successful request after 3 attempts".into());
            }
        }
    }
    


}

// cookie handeler

// Define a custom error type for CookieException
#[derive(Debug)]
pub struct CookieException {
    pub message: String,
}

impl fmt::Display for CookieException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CookieException: {}", self.message)
    }
}

impl Error for CookieException {}

// Implement From<reqwest::Error> for CookieException to convert errors
impl From<ReqwestError> for CookieException {
    fn from(error: ReqwestError) -> Self {
        CookieException {
            message: format!("Request failed: {}", error),
        }
    }
}
// Define the trait equivalent to BaseCookiesHandler
#[async_trait]
pub trait BaseCookiesHandler {
    async fn generate(&self) -> Result<HashMap<String, String>, CookieException>;
    async fn validate(&self, cookies: &HashMap<String, String>) -> Result<(), CookieException>;
}

// Example implementation of BaseCookiesHandler
pub struct ZenrowsCookiesHandler{
    cookie_url: String,
    api_key: String,
    premium_proxy: bool,
    proxy_handler: Option<Box<dyn ProxyHandler>>,
}

impl ZenrowsCookiesHandler {
    pub fn new(
        cookie_url: String,
        api_key: String,
        premium_proxy: bool,
        proxy_handler: Option<Box<dyn ProxyHandler>>,
    )->Self{
        ZenrowsCookiesHandler {
            cookie_url,
            api_key,
            premium_proxy,
            proxy_handler,
        }
    }

    fn parse_cookies(cookie_string: &str) -> HashMap<String, String> {
        let mut cookie_map = HashMap::new();
        for item in cookie_string.split(';') {
            let parts: Vec<&str> = item.splitn(2, '=').collect();
            if parts.len() == 2 {
                cookie_map.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
            }
        }
        cookie_map
    }
}

#[async_trait]
impl BaseCookiesHandler for ZenrowsCookiesHandler {
    
    async fn generate(&self) -> Result<HashMap<String, String>, CookieException> {
        println!("cook gen called");
      
    
        let mut params = vec![
            ("url", self.cookie_url.clone()),
            ("apikey", self.api_key.clone()),
            ("js_render", "true".to_string()),
        ];
        if self.premium_proxy {
            params.push(("premium_proxy", "true".to_string()));
        }
        let client = Client::new();
        let response = client
            .get("https://api.zenrows.com/v1/")
            .query(&params)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| CookieException {
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(CookieException {
                message: format!("HTTP error: {}", response.status()),
            });
        }

        let cookie_header = response
            .headers()
            .get("Zr-Cookies")
            .ok_or(CookieException {
                message: "Missing Zr-Cookies header".to_string(),
            })?;

        let cookie_string = cookie_header.to_str().map_err(|e| CookieException {
            message: format!("Invalid cookie string: {}", e),
        })?;

        let cookies = Self::parse_cookies(cookie_string);
        println!("hey {:?}",cookies);

        Ok(cookies)
    }

    async fn validate(&self, cookies: &HashMap<String, String>) -> Result<(), CookieException> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"));
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"));
        
        // Format cookies into a single string to pass as the Cookie header
        let cookie_string: String = cookies.iter()
                .filter(|(key, _)| *key == "KP_UIDz-ssn" || *key == "KP_UIDz") // Dereference the `key` here
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<String>>()
                .join("; ");


        
        headers.insert(COOKIE, HeaderValue::from_str(&cookie_string).map_err(|e| CookieException {
            message: format!("Failed to set cookie header: {}", e),
        })?);
        println!("{:?}",headers);

        let proxy = Proxy::all(self.proxy_handler.as_ref().unwrap().get_proxy().unwrap())?;

        let client = Client::builder()
                .proxy(proxy)
                .build()?;
        let res = client
            .get(self.cookie_url.clone())
            .headers(headers)
            .send()
            .await?;
        
        if res.status().is_success() {
                let body = res.text().await?;
                
                // Check that the body is not empty
                if !body.is_empty() {
                    println!("{}", body);
                } else {
                    eprintln!("Empty response body received.");
                    return Err(CookieException {
                        message: "Empty response body".to_string(),
                    });
                }
            } else {
                eprintln!("Request failed with status: {}", res.status());
                return Err(CookieException {
                    message: format!("Request failed with status: {}", res.status()),
                });
            }
        Ok(())
    }
}



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

// Utils function
pub fn load_proxies(path: &str) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(contents) => contents.lines().map(|s| s.to_string()).collect(),
        Err(_) => vec![],
    }
}


// #[tokio::main]
// async fn main() {
//     println!("Hello, world!");
    
//     // Load proxies from file
//     let proxies = load_proxies("/home/ragu/Desktop/src/.config/proxies.txt");
    

    
//     // Set up the ZenrowsCookiesHandler
//     let cookie_url = "https://www.property.com.au/".to_string();
//     let api_key = "b46ad9c3eaa157dcbc0a884ec2f1be72bce1d7a7".to_string();
//     let premium_proxy = false;
    
//     let zenrows_handler = ZenrowsCookiesHandler::new(cookie_url, api_key, premium_proxy, Some(Box::new( BrightDataRandomProxyHandler::new(proxies.clone()))));


//     let zenrows_handler = Arc::new(zenrows_handler);
//     let proxy_handler = Arc::new(Mutex::new( BrightDataRandomProxyHandler::new(proxies)));

// // Now call the function:
//     let mut request_api = AsyncRequestHandler::new(Some(zenrows_handler), Some(proxy_handler));
//     // let request_api = AsyncRequestHandler::new(zenrows_handler, proxy_handler);
//     let res = request_api.make_request("https://www.property.com.au/search/?locations=Advancetown%2C+QLD+4211&propertyStatus=FOR_SALE&pageNumber=1&surroundingSuburbs=false").await;
//     println!("{:?}",res)


// }

