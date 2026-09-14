use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct AppTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub compose_yml: String,
}

pub fn get_templates() -> Vec<AppTemplate> {
    vec![
        AppTemplate {
            id: "nextcloud".to_string(),
            name: "Nextcloud".to_string(),
            description: "Self-hosted productivity platform.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/commons/6/60/Nextcloud_Logo.svg"
                .to_string(),
            compose_yml: r#"
version: '3'
services:
  app:
    image: nextcloud
    restart: always
    ports:
      - 8080:80
    volumes:
      - nextcloud_data:/var/www/html
volumes:
  nextcloud_data:
"#
            .to_string(),
        },
        AppTemplate {
            id: "pihole".to_string(),
            name: "Pi-hole".to_string(),
            description: "Network-wide Ad Blocking.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/en/1/15/Pi-hole_vector_logo.svg"
                .to_string(),
            compose_yml: r#"
version: '3'
services:
  pihole:
    container_name: pihole
    image: pihole/pihole:latest
    ports:
      - "53:53/tcp"
      - "53:53/udp"
      - "8081:80/tcp"
    environment:
      TZ: 'America/Chicago'
      WEBPASSWORD: 'admin'
    volumes:
       - 'pihole_etc:/etc/pihole/'
       - 'pihole_dnsmasq:/etc/dnsmasq.d/'
    restart: unless-stopped
volumes:
  pihole_etc:
  pihole_dnsmasq:
"#
            .to_string(),
        },
        AppTemplate {
            id: "wordpress".to_string(),
            name: "WordPress".to_string(),
            description: "Build a website or blog.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/commons/9/93/Wordpress_Blue_logo.png"
                .to_string(),
            compose_yml: r#"
version: '3'
services:
  wordpress:
    image: wordpress
    restart: always
    ports:
      - 8082:80
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: wp
      WORDPRESS_DB_PASSWORD: wp
      WORDPRESS_DB_NAME: wordpress
    volumes:
      - wordpress_data:/var/www/html
  db:
    image: mariadb
    restart: always
    environment:
      MYSQL_DATABASE: wordpress
      MYSQL_USER: wp
      MYSQL_PASSWORD: wp
      MYSQL_RANDOM_ROOT_PASSWORD: '1'
    volumes:
      - db_data:/var/lib/mysql
volumes:
  wordpress_data:
  db_data:
"#
            .to_string(),
        },
    ]
}

pub async fn list_apps() -> impl Responder {
    HttpResponse::Ok().json(get_templates())
}

#[derive(Deserialize)]
pub struct AppInstallRequest {
    pub id: String,
}

pub async fn install_app(body: web::Json<AppInstallRequest>) -> impl Responder {
    let templates = get_templates();
    let template = match templates.into_iter().find(|t| t.id == body.id) {
        Some(t) => t,
        None => return HttpResponse::NotFound().json("App template not found"),
    };

    let wadm_dir = Path::new("/var/lib/wadm/apps");
    if !wadm_dir.exists() {
        let _ = fs::create_dir_all(wadm_dir);
    }

    let app_dir = wadm_dir.join(&template.id);
    if !app_dir.exists() {
        let _ = fs::create_dir_all(&app_dir);
    }

    let compose_file = app_dir.join("docker-compose.yml");
    if let Err(e) = fs::write(&compose_file, &template.compose_yml) {
        return HttpResponse::InternalServerError()
            .json(format!("Failed to write compose file: {}", e));
    }

    info!("Installing app {} via docker-compose...", template.name);

    actix_web::rt::spawn(async move {
        let _ = Command::new("sudo")
            .args(&["-n", "docker-compose", "up", "-d"])
            .current_dir(&app_dir)
            .output();

        let _ = Command::new("sudo")
            .args(&["-n", "docker", "compose", "up", "-d"])
            .current_dir(&app_dir)
            .output();
    });

    HttpResponse::Ok().json(format!(
        "{} is installing in the background.",
        template.name
    ))
}
