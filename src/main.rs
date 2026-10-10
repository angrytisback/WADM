use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;
use sysinfo::{Networks, System};

mod api;
mod apps;
mod audit;
mod auth;
mod cluster;
mod drivers;
mod middleware;
mod plugins;
mod proxy;
mod ssl;
use middleware::Auth;

use api::auth::load_auth_store;
use api::monitor::AppState;

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    api::logs::init();
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("WADM - Modern Linux Web Administration Panel");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("Usage:");
        println!("  wadm [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --agent               Run in Cluster Node Agent mode (WebSocket tunnel)");
        println!("  --hub-url <URL>       Hub WebSocket tunnel URL (default: ws://localhost:8168/api/cluster/tunnel)");
        println!("  --token <TOKEN>       Cluster node authentication token");
        println!("  --name <NAME>         Friendly name for this cluster node");
        println!("  -v, --version         Print version information");
        println!("  -h, --help            Print this help message");
        return Ok(());
    }

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("wadm {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|a| a == "--agent" || a == "agent") {
        let mut hub_url = std::env::var("WADM_HUB_URL")
            .unwrap_or_else(|_| "ws://localhost:8168/api/cluster/tunnel".to_string());
        let mut token = std::env::var("WADM_AGENT_TOKEN").unwrap_or_default();
        let mut name = std::env::var("WADM_NODE_NAME").ok();

        let mut i = 0;
        while i < args.len() {
            if args[i] == "--hub-url" && i + 1 < args.len() {
                hub_url = args[i + 1].clone();
                i += 1;
            } else if args[i] == "--token" && i + 1 < args.len() {
                token = args[i + 1].clone();
                i += 1;
            } else if args[i] == "--name" && i + 1 < args.len() {
                name = Some(args[i + 1].clone());
                i += 1;
            }
            i += 1;
        }

        cluster::agent::run_agent(hub_url, token, name).await;
        return Ok(());
    }

    let port = 8168;
    log::info!("Starting WADM server on port {}", port);

    log::info!("Serving embedded web UI assets (Single-Binary mode active)");

    let sys = System::new_all();
    let networks = Networks::new_with_refreshed_list();
    let (metrics_sender, _) = tokio::sync::broadcast::channel::<api::monitor::SystemStats>(32);
    let app_state = web::Data::new(AppState {
        sys: Mutex::new(sys),
        networks: Mutex::new(networks),
        metrics_sender,
    });

    api::monitor::start_telemetry_loop(app_state.clone());

    let db_path = api::jobs::get_wadm_db_path();
    log::info!("Initializing SQLite database at: {:?}", db_path);
    let job_db = std::sync::Arc::new(
        api::jobs::JobDatabase::new(db_path.clone()).expect("Failed to initialize SQLite database"),
    );

    let ssl_db = std::sync::Arc::new(
        ssl::SslDatabase::new(db_path.clone()).expect("Failed to initialize SSL database"),
    );
    let ssl_db_data = web::Data::new(ssl_db.clone());

    let challenge_store = ssl::create_challenge_store();
    let challenge_store_data = web::Data::new(challenge_store.clone());

    let job_manager =
        api::jobs::JobManager::new_with_ssl(job_db, challenge_store.clone(), ssl_db.clone());
    let job_manager_data = web::Data::new(job_manager.clone());

    // Start auto-renewal daemon
    ssl::renewal::start_renewal_daemon(ssl_db.clone(), job_manager);

    let user_db = std::sync::Arc::new(
        auth::UserDatabase::new(db_path.clone()).expect("Failed to initialize User database"),
    );
    let user_db_data = web::Data::new(user_db);

    let audit_logger = std::sync::Arc::new(
        audit::AuditLogger::new(db_path.clone()).expect("Failed to initialize Audit database"),
    );
    let audit_logger_data = web::Data::new(audit_logger);

    let cluster_db = std::sync::Arc::new(
        cluster::ClusterDatabase::new(db_path.clone())
            .expect("Failed to initialize Cluster database"),
    );
    let cluster_db_data = web::Data::new(cluster_db.clone());

    let proxy_db = std::sync::Arc::new(
        proxy::ProxyDatabase::new(db_path).expect("Failed to initialize Proxy database"),
    );
    let proxy_db_data = web::Data::new(proxy_db.clone());

    let cluster_manager = cluster::ClusterManager::new(cluster_db.clone());
    let cluster_manager_data = web::Data::new(cluster_manager.clone());

    cluster::hub::start_heartbeat_auditor(cluster_manager, cluster_db);

    let auth_store = web::Data::new(Mutex::new(load_auth_store()));

    let app_config = web::Data::new(Mutex::new(api::config::load_config()));

    let driver_registry = web::Data::new(drivers::DriverRegistry::detect_and_init());

    let plugin_manager = std::sync::Arc::new(plugins::PluginManager::default_instance());
    let plugin_manager_data = web::Data::new(plugin_manager.clone());

    let plugin_store = std::sync::Arc::new(plugins::PluginStore::new());
    let plugin_store_data = web::Data::new(plugin_store.clone());

    let server = HttpServer::new(move || {
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
                .supports_credentials()
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
                            || origin_str.contains(":5173")
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
                .supports_credentials()
                .max_age(3600)
        };

        App::new()
            .app_data(app_state.clone())
            .app_data(auth_store.clone())
            .app_data(app_config.clone())
            .app_data(job_manager_data.clone())
            .app_data(driver_registry.clone())
            .app_data(plugin_manager_data.clone())
            .app_data(plugin_store_data.clone())
            .app_data(user_db_data.clone())
            .app_data(audit_logger_data.clone())
            .app_data(cluster_db_data.clone())
            .app_data(cluster_manager_data.clone())
            .app_data(ssl_db_data.clone())
            .app_data(challenge_store_data.clone())
            .app_data(proxy_db_data.clone())
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
            .route(
                "/.well-known/acme-challenge/{token}",
                web::get().to(api::ssl::acme_challenge_handler),
            )
            .service(web::scope("/api").wrap(Auth).configure(api::config))
            .service(
                web::resource(["/apps/{app_id}", "/apps/{app_id}/{tail:.*}"])
                    .route(web::route().to(proxy::proxy_subpath_handler)),
            )
            .default_service(web::to(proxy::proxy_domain_or_static_handler))
    });

    let ssl_config = ssl_db.get_config().unwrap_or_default();
    let (cert_path, key_path) = ssl::get_cert_paths();
    let ssl_active = ssl_config.enabled && cert_path.exists() && key_path.exists();

    if ssl_active {
        match ssl::tls::build_server_config(&cert_path, &key_path) {
            Ok(tls_config) => {
                log::info!(
                    "TLS enabled: Starting HTTPS server on port {}",
                    ssl_config.https_port
                );

                if ssl_config.force_https {
                    let redirect_https_port = ssl_config.https_port;
                    let chal_store_clone = challenge_store.clone();
                    let http_p = ssl_config.http_port;
                    std::thread::spawn(move || {
                        let sys = actix_web::rt::System::new();
                        sys.block_on(async move {
                            let res = HttpServer::new(move || {
                                let chal_data = web::Data::new(chal_store_clone.clone());
                                App::new()
                                    .app_data(chal_data)
                                    .route(
                                        "/.well-known/acme-challenge/{token}",
                                        web::get().to(api::ssl::acme_challenge_handler),
                                    )
                                    .default_service(web::to(move |req: actix_web::HttpRequest| {
                                        let host_str = req.connection_info().host().to_string();
                                        let host_without_port =
                                            host_str.split(':').next().unwrap_or("localhost");
                                        let uri_str = req
                                            .uri()
                                            .path_and_query()
                                            .map(|pq| pq.as_str())
                                            .unwrap_or("");
                                        let target = format!(
                                            "https://{}:{}{}",
                                            host_without_port, redirect_https_port, uri_str
                                        );
                                        async move {
                                            HttpResponse::MovedPermanently()
                                                .append_header((
                                                    actix_web::http::header::LOCATION,
                                                    target,
                                                ))
                                                .finish()
                                        }
                                    }))
                            })
                            .bind(("0.0.0.0", http_p));

                            match res {
                                Ok(srv) => {
                                    log::info!("Started HTTP redirect listener on port {}", http_p);
                                    let _ = srv.run().await;
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Could not bind HTTP redirect server on port {}: {}. Continuing with HTTPS only.",
                                        http_p,
                                        e
                                    );
                                }
                            }
                        });
                    });
                }

                server
                    .bind_rustls_0_23(("0.0.0.0", ssl_config.https_port), tls_config)?
                    .run()
                    .await
            }
            Err(e) => {
                log::error!(
                    "Failed to initialize TLS config: {}. Falling back to HTTP on port {}",
                    e,
                    port
                );
                server.bind(("0.0.0.0", port))?.run().await
            }
        }
    } else {
        log::info!("Starting WADM in HTTP mode on port {}", port);
        server.bind(("0.0.0.0", port))?.run().await
    }
}
