use actix_web::web;

pub mod apps;
pub mod audit;
pub mod auth;
pub mod cluster;
pub mod config;
pub mod db;
pub mod dependencies;
pub mod docker;
pub mod files;
pub mod firewall;
pub mod jobs;
pub mod logs;
pub mod monitor;
pub mod pkgmgr;
pub mod plugins;
pub mod services;
pub mod ssl;
pub mod system;
pub mod terminal;
pub mod users;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/auth/status").route(web::get().to(auth::get_auth_status)));
    cfg.service(web::resource("/auth/setup/init").route(web::post().to(auth::init_setup)));
    cfg.service(web::resource("/auth/setup/confirm").route(web::post().to(auth::confirm_setup)));
    cfg.service(web::resource("/auth/login").route(web::post().to(auth::login)));
    cfg.service(web::resource("/auth/logout").route(web::post().to(auth::logout)));
    cfg.service(web::resource("/auth/me").route(web::get().to(auth::get_me)));

    cfg.service(web::resource("/stats").route(web::get().to(monitor::get_system_stats)));
    cfg.service(web::resource("/stats/stream").route(web::get().to(monitor::get_stats_stream)));
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
    cfg.service(
        web::resource("/apps/{id}/credentials").route(web::get().to(apps::get_app_credentials)),
    );
    cfg.service(web::resource("/apps/{id}/uninstall").route(web::post().to(apps::uninstall_app)));

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

    cfg.service(web::resource("/jobs").route(web::get().to(jobs::list_jobs)));
    cfg.service(web::resource("/jobs/{id}").route(web::get().to(jobs::get_job)));
    cfg.service(web::resource("/jobs/{id}/stream").route(web::get().to(jobs::stream_job_logs)));

    cfg.service(web::resource("/plugins/store").route(web::get().to(plugins::list_store_plugins)));
    cfg.service(
        web::resource("/plugins/store/refresh")
            .route(web::post().to(plugins::refresh_store_catalog)),
    );
    cfg.service(
        web::resource("/plugins/store/{id}/install")
            .route(web::post().to(plugins::install_store_plugin)),
    );
    cfg.service(
        web::resource("/plugins/store/{id}/update")
            .route(web::post().to(plugins::update_store_plugin)),
    );
    cfg.service(
        web::resource("/plugins/store/{id}")
            .route(web::delete().to(plugins::uninstall_store_plugin)),
    );

    cfg.service(web::resource("/plugins").route(web::get().to(plugins::list_plugins)));
    cfg.service(web::resource("/plugins/{id}").route(web::get().to(plugins::get_plugin)));
    cfg.service(
        web::resource("/plugins/{id}/enable").route(web::post().to(plugins::enable_plugin)),
    );
    cfg.service(
        web::resource("/plugins/{id}/disable").route(web::post().to(plugins::disable_plugin)),
    );
    cfg.service(web::resource("/plugins/{id}/rpc").route(web::post().to(plugins::forward_rpc)));
    cfg.service(web::resource("/plugins/{id}/ping").route(web::post().to(plugins::ping_plugin)));
    cfg.service(web::resource("/audit-logs").route(web::get().to(audit::get_audit_logs)));
    cfg.service(
        web::resource("/users")
            .route(web::get().to(users::list_users))
            .route(web::post().to(users::create_user)),
    );
    cfg.service(web::resource("/users/{id}/role").route(web::post().to(users::update_user_role)));
    cfg.service(
        web::resource("/users/{id}/password").route(web::post().to(users::update_user_password)),
    );
    cfg.service(web::resource("/users/{id}").route(web::delete().to(users::delete_user)));
    cfg.service(web::resource("/ssl/config").route(web::get().to(ssl::get_ssl_config)));
    cfg.service(
        web::resource("/ssl/self-signed").route(web::post().to(ssl::generate_self_signed_cert)),
    );
    cfg.service(web::resource("/ssl/custom").route(web::post().to(ssl::upload_custom_cert)));
    cfg.service(
        web::resource("/ssl/letsencrypt").route(web::post().to(ssl::request_letsencrypt_cert)),
    );
    cfg.service(web::resource("/ssl/toggle").route(web::post().to(ssl::toggle_ssl)));

    // Reverse Proxy management endpoints
    cfg.service(
        web::resource("/proxy/routes")
            .route(web::get().to(crate::proxy::list_proxy_routes))
            .route(web::post().to(crate::proxy::create_proxy_route)),
    );
    cfg.service(
        web::resource("/proxy/routes/{app_id}")
            .route(web::delete().to(crate::proxy::delete_proxy_route)),
    );

    // Cluster Federation endpoints
    cfg.service(web::resource("/cluster/nodes").route(web::get().to(cluster::list_nodes)));
    cfg.service(
        web::resource("/cluster/nodes/generate-token")
            .route(web::post().to(cluster::generate_join_token)),
    );
    cfg.service(
        web::resource("/cluster/nodes/{id}")
            .route(web::get().to(cluster::get_node))
            .route(web::delete().to(cluster::delete_node)),
    );
    cfg.service(web::resource("/cluster/tunnel").route(web::get().to(cluster::tunnel_handler)));
    cfg.service(
        web::resource("/cluster/nodes/{id}/stats").route(web::get().to(cluster::proxy_node_stats)),
    );
    cfg.service(
        web::resource("/cluster/nodes/{id}/system")
            .route(web::get().to(cluster::proxy_node_system)),
    );
    cfg.service(
        web::resource("/cluster/nodes/{id}/services")
            .route(web::get().to(cluster::proxy_node_services)),
    );
    cfg.service(
        web::resource("/cluster/nodes/{id}/services/{name}")
            .route(web::post().to(cluster::proxy_node_service_action)),
    );
    cfg.service(
        web::resource("/cluster/nodes/{id}/jobs")
            .route(web::get().to(cluster::proxy_node_jobs))
            .route(web::post().to(cluster::proxy_node_execute_job)),
    );
    cfg.service(
        web::resource("/cluster/nodes/{id}/command")
            .route(web::post().to(cluster::proxy_node_command)),
    );
}
