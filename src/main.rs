use cookies_handler::ZenrowsCookiesHandler;
use proxy_handler::BrightDataRandomProxyHandler;
use request_handler::AsyncRequestHandler;
use reqwest::Url;
use serde_derive::Deserialize;
use utils::load_proxies;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use env_logger;

mod cookies_handler;
mod proxy_handler;
mod request_handler;
mod utils;




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

    let zenrows_handler = ZenrowsCookiesHandler::new(
        cookie_url,
        api_key,
        premium_proxy,
        Some(Box::new(BrightDataRandomProxyHandler::new(proxies.clone()))),
    );

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
        Ok(body) => {
            HttpResponse::Ok().json(serde_json::json!({ "status_code": 200, "body": body }))
        }
        Err(_) => HttpResponse::TooManyRequests()
            .json(serde_json::json!({ "status_code": 429, "body": "" })),
    }
}

#[derive(Deserialize, Debug)] // Add the Debug derive here
struct RequestData {
    url: String,
}
