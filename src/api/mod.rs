use actix_web::web;

pub mod apps;
pub mod auth;
pub mod config;
pub mod db;
pub mod dependencies;
pub mod docker;
pub mod files;
pub mod firewall;
pub mod logs;
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
    cfg.service(
        web::resource("/system/dependencies")
            .route(web::get().to(dependencies::check_dependencies)),
    );
    cfg.service(
        web::resource("/system/dependencies/install")
            .route(web::post().to(dependencies::install_dependency)),
    );

    cfg.service(web::resource("/system/reboot").route(web::post().to(system::reboot_system)));
    cfg.service(web::resource("/system/power").route(web::post().to(system::handle_power_action)));
    cfg.service(
        web::resource("/system/power/status").route(web::get().to(system::get_power_status)),
    );
    cfg.service(
        web::resource("/system/maintenance")
            .route(web::post().to(system::handle_maintenance_action)),
    );
    cfg.service(web::resource("/system/dns").route(web::get().to(system::get_dns_info)));
    cfg.service(web::resource("/system/dns/flush").route(web::post().to(system::flush_dns)));
    cfg.service(web::resource("/system/speedtest").route(web::post().to(system::run_speedtest)));

    cfg.service(web::resource("/apps").route(web::get().to(apps::list_apps)));
    cfg.service(web::resource("/apps/install").route(web::post().to(apps::install_app)));

    cfg.service(web::resource("/files/list").route(web::get().to(files::list_files)));
    cfg.service(web::resource("/files/read").route(web::get().to(files::read_file)));
    cfg.service(web::resource("/files/write").route(web::post().to(files::write_file)));
    cfg.service(web::resource("/files/create").route(web::post().to(files::create_item)));
    cfg.service(web::resource("/files/delete").route(web::delete().to(files::delete_item)));
    cfg.service(web::resource("/files/download").route(web::get().to(files::download_file)));
    cfg.service(web::resource("/files/upload").route(web::post().to(files::upload_file)));

    cfg.service(web::resource("/processes").route(web::get().to(monitor::get_processes)));
    cfg.service(web::resource("/processes/kill").route(web::post().to(monitor::kill_process)));
    cfg.service(web::resource("/packages").route(web::get().to(pkgmgr::list_packages)));
    cfg.service(
        web::resource("/packages/installed").route(web::get().to(pkgmgr::list_installed_packages)),
    );
    cfg.service(web::resource("/packages/upgrade").route(web::post().to(pkgmgr::upgrade_package)));
    cfg.service(web::resource("/packages/install").route(web::post().to(pkgmgr::install_package)));
    cfg.service(
        web::resource("/packages/update-all").route(web::post().to(pkgmgr::update_all_packages)),
    );
    cfg.service(web::resource("/packages/remove").route(web::post().to(pkgmgr::remove_package)));
    cfg.service(
        web::resource("/packages/remove-dry-run")
            .route(web::post().to(pkgmgr::remove_package_dry_run)),
    );
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
        web::resource("/db/{engine}/{db_name}/tables").route(web::get().to(db::list_tables)),
    );
    cfg.service(
        web::resource("/db/{engine}/{db_name}/{table_name}/data")
            .route(web::get().to(db::get_table_data)),
    );
    cfg.service(
        web::resource("/db/{engine}/{db_name}/query").route(web::post().to(db::execute_query)),
    );
    cfg.service(
        web::resource("/db/{engine}/{db_name}/backups").route(web::get().to(db::list_backups)),
    );
    cfg.service(
        web::resource("/db/{engine}/{db_name}/backup").route(web::post().to(db::create_backup)),
    );
    cfg.service(
        web::resource("/db/{engine}/{db_name}/upload").route(web::post().to(db::upload_backup)),
    );
    cfg.service(
        web::resource("/db/{engine}/{db_name}/backups/{filename}")
            .route(web::post().to(db::restore_backup))
            .route(web::delete().to(db::delete_backup)),
    );
    cfg.service(
        web::resource("/db/{engine}/{db_name}/backups/{filename}/download")
            .route(web::get().to(db::download_backup)),
    );

    cfg.service(
        web::resource("/config")
            .route(web::get().to(config::get_config))
            .route(web::post().to(config::update_config)),
    );

    cfg.service(web::resource("/terminal/ws").to(terminal::ws_terminal));

    cfg.service(web::resource("/logs").route(web::get().to(logs::get_logs)));
    cfg.service(web::resource("/logs/clear").route(web::post().to(logs::clear_logs)));
}
