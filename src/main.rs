use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;
use sysinfo::{Networks, System};

mod api;
mod middleware;
use middleware::Auth;

use api::auth::load_auth_store;
use api::monitor::AppState;

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    api::logs::init();

    let port = 8168;
    log::info!("Starting WADM server on port {}", port);

    if let Ok(cwd) = std::env::current_dir() {
        log::info!("Current Working Directory: {:?}", cwd);
        let dist_path = cwd.join("web/dist");
        if dist_path.exists() {
            log::info!("Found frontend assets at: {:?}", dist_path);
        } else {
            log::error!("CRITICAL: Frontend assets NOT found at: {:?}. Ensure 'web/dist' exists relative to execution path.", dist_path);
        }
    }

    let sys = System::new_all();
    let networks = Networks::new_with_refreshed_list();
    let app_state = web::Data::new(AppState {
        sys: Mutex::new(sys),
        networks: Mutex::new(networks),
    });

    let auth_store = web::Data::new(Mutex::new(load_auth_store()));

    let app_config = web::Data::new(Mutex::new(api::config::load_config()));

    HttpServer::new(move || {
        let allowed_origins = std::env::var("WADM_ALLOWED_ORIGINS").unwrap_or_default();
        let allowed_list: Vec<String> = allowed_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let cors = if !allowed_list.is_empty() {
            let mut c = Cors::default();
            for origin in allowed_list {
                c = c.allowed_origin(&origin);
            }
            c.allowed_methods(vec!["GET", "POST", "DELETE", "OPTIONS"])
                .allowed_headers(vec![
                    actix_web::http::header::AUTHORIZATION,
                    actix_web::http::header::ACCEPT,
                    actix_web::http::header::CONTENT_TYPE,
                    actix_web::http::header::HeaderName::from_static("sec-websocket-protocol"),
                ])
                .max_age(3600)
        } else {
            Cors::default()
                .allowed_origin_fn(|origin, _req_head| {
                    if let Ok(origin_str) = origin.to_str() {
                        origin_str.starts_with("http://localhost")
                            || origin_str.starts_with("https://localhost")
                            || origin_str.starts_with("http://127.0.0.1")
                            || origin_str.starts_with("https://127.0.0.1")
                            || origin_str.contains(":8168")
                    } else {
                        false
                    }
                })
                .allowed_methods(vec!["GET", "POST", "DELETE", "OPTIONS"])
                .allowed_headers(vec![
                    actix_web::http::header::AUTHORIZATION,
                    actix_web::http::header::ACCEPT,
                    actix_web::http::header::CONTENT_TYPE,
                    actix_web::http::header::HeaderName::from_static("sec-websocket-protocol"),
                ])
                .max_age(3600)
        };

        App::new()
            .app_data(app_state.clone())
            .app_data(auth_store.clone())
            .app_data(app_config.clone())
            .wrap(cors)
            .wrap(
                actix_web::middleware::DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add((
                        "Permissions-Policy",
                        "geolocation=(), microphone=(), camera=()",
                    )),
            )
            .wrap(actix_web::middleware::Logger::new("REQ|%m|%U|%s"))
            .route("/api/health", web::get().to(health_check))
            .service(web::scope("/api").wrap(Auth).configure(api::config))
            .service(
                actix_files::Files::new("/", "./web/dist")
                    .index_file("index.html")
                    .default_handler(web::to(|| async {
                        actix_files::NamedFile::open_async("./web/dist/index.html").await
                    }))
                    .prefer_utf8(true),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
