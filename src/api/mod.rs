use actix_web::web;

pub mod auth;
pub mod config;
pub mod db;
pub mod docker;
pub mod firewall;
pub mod monitor;
pub mod pkgmgr;
pub mod services;
pub mod system;
pub mod terminal;

pub fn config(cfg: &mut web::ServiceConfig) {
    
    cfg.service(web::resource("/auth/status").route(web::get().to(auth::get_auth_status)));
    cfg.service(web::resource("/auth/setup/init").route(web::post().to(auth::init_setup)));
    cfg.service(web::resource("/auth/setup/confirm").route(web::post().to(auth::confirm_setup)));
    cfg.service(web::resource("/auth/login").route(web::post().to(auth::login)));

    cfg.service(web::resource("/stats").route(web::get().to(monitor::get_system_stats)));
    cfg.service(web::resource("/system").route(web::get().to(system::get_detailed_info)));
    cfg.service(web::resource("/system/reboot").route(web::post().to(system::reboot_system)));
    cfg.service(web::resource("/processes").route(web::get().to(monitor::get_processes)));
    cfg.service(web::resource("/processes/kill").route(web::post().to(monitor::kill_process)));
    cfg.service(web::resource("/packages").route(web::get().to(pkgmgr::list_packages)));
    cfg.service(web::resource("/packages/upgrade").route(web::post().to(pkgmgr::upgrade_package)));
    cfg.service(web::resource("/packages/install").route(web::post().to(pkgmgr::install_package)));
    cfg.service(web::resource("/services").route(web::get().to(services::list_services)));
    cfg.service(web::resource("/services/{name}").route(web::post().to(services::control_service)));
    cfg.service(
        web::resource("/services/{name}/logs").route(web::get().to(services::get_service_logs)),
    );
    cfg.service(web::resource("/docker").route(web::get().to(docker::list_containers)));
    cfg.service(web::resource("/docker/status").route(web::get().to(docker::get_status)));
    cfg.service(web::resource("/docker/start").route(web::post().to(docker::start_service)));
    cfg.service(web::resource("/docker/{id}").route(web::post().to(docker::control_container)));
    cfg.service(
        web::resource("/docker/{id}/stats").route(web::get().to(docker::get_container_stats)),
    );

    cfg.service(web::resource("/firewall").route(web::get().to(firewall::get_status)));
    cfg.service(web::resource("/firewall/action").route(web::post().to(firewall::set_status)));
    cfg.service(web::resource("/firewall/install").route(web::post().to(firewall::install_ufw)));
    cfg.service(
        web::resource("/firewall/rules")
            .route(web::post().to(firewall::add_rule))
            .route(web::delete().to(firewall::delete_rule)),
    );

    cfg.service(web::resource("/db").route(web::get().to(db::list_dbs)));

    
    cfg.service(
        web::resource("/config")
            .route(web::get().to(config::get_config))
            .route(web::post().to(config::update_config)),
    );
    
    cfg.service(web::resource("/terminal/ws").to(terminal::ws_terminal));
}
