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
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

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
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(app_state.clone())
            .app_data(auth_store.clone())
            .app_data(app_config.clone())
            .wrap(cors)
            .wrap(actix_web::middleware::Logger::default())
            .route("/api/health", web::get().to(health_check))
            .route("/api/packages", web::get().to(api::pkgmgr::list_packages))
            .route(
                "/api/packages/installed",
                web::get().to(api::pkgmgr::list_installed_packages),
            )
            .route(
                "/api/packages/upgrade",
                web::post().to(api::pkgmgr::upgrade_package),
            )
            .route(
                "/api/packages/install",
                web::post().to(api::pkgmgr::install_package),
            )
            .route(
                "/api/packages/update-all",
                web::post().to(api::pkgmgr::update_all_packages),
            )
            .route(
                "/api/packages/remove",
                web::post().to(api::pkgmgr::remove_package),
            )
            .route(
                "/api/packages/remove-dry-run",
                web::post().to(api::pkgmgr::remove_package_dry_run),
            )
            .service(
                web::scope("/api")
                    .wrap(Auth) 
                    .configure(api::config),
            )
            .service(actix_files::Files::new("/", "./web/dist").index_file("index.html"))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
