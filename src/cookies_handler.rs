use async_trait::async_trait;
use reqwest::Error as ReqwestError;
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, USER_AGENT},
    Client, Proxy,
};

use std::error::Error;
use std::fmt;

use std::collections::HashMap;

use crate::proxy_handler::ProxyHandler;

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

#[async_trait]
pub trait BaseCookiesHandler {
    async fn generate(&self) -> Result<HashMap<String, String>, CookieException>;
    async fn validate(&self, cookies: &HashMap<String, String>) -> Result<(), CookieException>;
}

// Example implementation of BaseCookiesHandler
pub struct ZenrowsCookiesHandler {
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
    ) -> Self {
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
        println!("hey {:?}", cookies);

        Ok(cookies)
    }

    async fn validate(&self, cookies: &HashMap<String, String>) -> Result<(), CookieException> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"));
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"));

        // Format cookies into a single string to pass as the Cookie header
        let cookie_string: String = cookies
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
        println!("{:?}", headers);

        let proxy = Proxy::all(self.proxy_handler.as_ref().unwrap().get_proxy().unwrap())?;

        let client = Client::builder().proxy(proxy).build()?;
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
