use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::TcpListener;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct AppTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub compose_yml: String,
    pub default_ports: Vec<u16>,
}

pub fn get_templates() -> Vec<AppTemplate> {
    vec![
        AppTemplate {
            id: "nextcloud".to_string(),
            name: "Nextcloud".to_string(),
            description: "Self-hosted productivity platform and file storage.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/commons/6/60/Nextcloud_Logo.svg".to_string(),
            default_ports: vec![8080],
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
            description: "Network-wide Ad and Tracker Blocking DNS sinkhole.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/en/1/15/Pi-hole_vector_logo.svg".to_string(),
            default_ports: vec![53, 8081],
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
      TZ: 'UTC'
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
            description: "Build a modern website, blog, or store with MariaDB.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/commons/9/93/Wordpress_Blue_logo.png".to_string(),
            default_ports: vec![8082],
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
        AppTemplate {
            id: "npm".to_string(),
            name: "Nginx Proxy Manager".to_string(),
            description: "Easily manage reverse proxy hosts and free SSL certificates.".to_string(),
            icon: "https://raw.githubusercontent.com/NginxProxyManager/nginx-proxy-manager/master/frontend/images/logo.png".to_string(),
            default_ports: vec![8084, 8085, 8443],
            compose_yml: r#"
version: '3.8'
services:
  app:
    image: 'jc21/nginx-proxy-manager:latest'
    restart: unless-stopped
    ports:
      - '8084:80'
      - '8085:81'
      - '8443:443'
    volumes:
      - npm_data:/data
      - npm_letsencrypt:/etc/letsencrypt
volumes:
  npm_data:
  npm_letsencrypt:
"#
            .to_string(),
        },
        AppTemplate {
            id: "portainer".to_string(),
            name: "Portainer CE".to_string(),
            description: "Web-based container management interface for Docker.".to_string(),
            icon: "https://www.portainer.io/hubfs/Portainer%20Icon%20Colour.svg".to_string(),
            default_ports: vec![9000, 9443],
            compose_yml: r#"
version: '3.8'
services:
  portainer:
    image: portainer/portainer-ce:latest
    restart: always
    ports:
      - "9000:9000"
      - "9443:9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data
volumes:
  portainer_data:
"#
            .to_string(),
        },
    ]
}

fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_ok()
}

fn generate_secure_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..20)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn is_valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && !id.starts_with('-')
        && !id.contains('/')
        && !id.contains('\\')
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn get_apps_dir() -> std::path::PathBuf {
    if let Ok(data_dir) = std::env::var("WADM_DATA_DIR") {
        return std::path::PathBuf::from(data_dir).join("apps");
    }
    let system_dir = std::path::Path::new("/var/lib/wadm/apps");
    if system_dir.exists() || std::fs::create_dir_all(system_dir).is_ok() {
        system_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join("data")
            .join("apps")
    }
}

pub async fn list_apps() -> impl Responder {
    HttpResponse::Ok().json(get_templates())
}

#[derive(Deserialize)]
pub struct AppInstallRequest {
    pub id: String,
}

#[derive(Serialize)]
pub struct AppCredentialsResponse {
    pub id: String,
    pub credentials: Option<String>,
    pub installed: bool,
}

pub async fn get_app_credentials(path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    if !is_valid_app_id(&id) {
        return HttpResponse::BadRequest().json("Invalid app identifier");
    }

    let apps_dir = get_apps_dir();
    let creds_file = apps_dir.join(&id).join("credentials.txt");
    let compose_file = apps_dir.join(&id).join("docker-compose.yml");
    let installed = compose_file.exists();

    if creds_file.exists() {
        if let Ok(content) = fs::read_to_string(&creds_file) {
            return HttpResponse::Ok().json(AppCredentialsResponse {
                id,
                credentials: Some(content),
                installed,
            });
        }
    }

    HttpResponse::Ok().json(AppCredentialsResponse {
        id,
        credentials: None,
        installed,
    })
}

pub async fn uninstall_app(path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    if !is_valid_app_id(&id) {
        return HttpResponse::BadRequest().json("Invalid app identifier");
    }

    let apps_dir = get_apps_dir();
    let app_dir = apps_dir.join(&id);

    if !app_dir.exists() {
        return HttpResponse::NotFound().json("App not found or not installed");
    }

    let dir_clone = app_dir.clone();
    let success = actix_web::web::block(move || {
        let mut cmd1 = Command::new("sudo");
        cmd1.args(["-n", "docker-compose", "down", "-v"])
            .current_dir(&dir_clone);
        if cmd1.output().map(|o| o.status.success()).unwrap_or(false) {
            return true;
        }

        let mut cmd2 = Command::new("sudo");
        cmd2.args(["-n", "docker", "compose", "down", "-v"])
            .current_dir(&dir_clone);
        cmd2.output().map(|o| o.status.success()).unwrap_or(false)
    })
    .await
    .unwrap_or(false);

    let _ = fs::remove_dir_all(&app_dir);

    if success {
        HttpResponse::Ok().json(format!("App {} uninstalled successfully", id))
    } else {
        HttpResponse::Ok().json(format!("App {} removed from filesystem", id))
    }
}

pub async fn install_app(body: web::Json<AppInstallRequest>) -> impl Responder {
    if !is_valid_app_id(&body.id) {
        return HttpResponse::BadRequest().json("Invalid app identifier");
    }

    let templates = get_templates();
    let template = match templates.into_iter().find(|t| t.id == body.id) {
        Some(t) => t,
        None => return HttpResponse::NotFound().json("App template not found"),
    };

    // Check for conflicting ports before starting
    let mut busy_ports = Vec::new();
    for port in &template.default_ports {
        if !is_port_available(*port) {
            busy_ports.push(*port);
        }
    }
    if !busy_ports.is_empty() {
        return HttpResponse::BadRequest().json(format!(
            "Cannot install {}: Port(s) {:?} are already in use on the host system.",
            template.name, busy_ports
        ));
    }

    let wadm_dir = get_apps_dir();
    if !wadm_dir.exists() {
        let _ = fs::create_dir_all(&wadm_dir);
    }

    let app_dir = wadm_dir.join(&template.id);
    if !app_dir.exists() {
        let _ = fs::create_dir_all(&app_dir);
    }

    // Replace hardcoded default passwords with securely generated random passwords
    let secure_pwd = generate_secure_password();
    let customized_compose = template
        .compose_yml
        .replace(
            "WEBPASSWORD: 'admin'",
            &format!("WEBPASSWORD: '{}'", secure_pwd),
        )
        .replace(
            "WORDPRESS_DB_PASSWORD: wp",
            &format!("WORDPRESS_DB_PASSWORD: {}", secure_pwd),
        )
        .replace(
            "MYSQL_PASSWORD: wp",
            &format!("MYSQL_PASSWORD: {}", secure_pwd),
        );

    let compose_file = app_dir.join("docker-compose.yml");
    if let Err(e) = fs::write(&compose_file, &customized_compose) {
        return HttpResponse::InternalServerError()
            .json(format!("Failed to write compose file: {}", e));
    }

    // Save generated credentials securely alongside the app
    let creds_file = app_dir.join("credentials.txt");
    let creds_content = format!(
        "Generated Password: {}\nGenerated At: {}\n",
        secure_pwd,
        chrono::Utc::now()
    );
    let _ = fs::write(&creds_file, creds_content);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&creds_file, fs::Permissions::from_mode(0o600));
    }

    info!(
        "Installing app {} via docker-compose with generated secure credentials...",
        template.name
    );

    let dir_clone = app_dir.clone();
    actix_web::rt::spawn(async move {
        let _ = actix_web::web::block(move || {
            let out1 = Command::new("sudo")
                .args(["-n", "docker-compose", "up", "-d"])
                .current_dir(&dir_clone)
                .output();

            if !out1.map(|o| o.status.success()).unwrap_or(false) {
                let _ = Command::new("sudo")
                    .args(["-n", "docker", "compose", "up", "-d"])
                    .current_dir(&dir_clone)
                    .output();
            }
        })
        .await;
    });

    HttpResponse::Ok().json(format!(
        "{} is installing. Secure password generated: {}. View anytime in credentials.",
        template.name, secure_pwd
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_templates_contains_all_apps() {
        let templates = get_templates();
        let ids: Vec<String> = templates.into_iter().map(|t| t.id).collect();
        assert!(ids.contains(&"nextcloud".to_string()));
        assert!(ids.contains(&"pihole".to_string()));
        assert!(ids.contains(&"wordpress".to_string()));
        assert!(ids.contains(&"npm".to_string()));
        assert!(ids.contains(&"portainer".to_string()));
    }

    #[test]
    fn test_is_port_available_on_ephemeral_port() {
        // Bind to port 0 to get an OS-assigned ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        // Since listener is holding it, is_port_available should return false
        assert!(!is_port_available(port));

        // Dropping the listener frees the port
        drop(listener);
        assert!(is_port_available(port));
    }

    #[test]
    fn test_is_valid_app_id() {
        assert!(is_valid_app_id("nextcloud"));
        assert!(is_valid_app_id("nginx-proxy-manager"));
        assert!(is_valid_app_id("app_123"));
        assert!(!is_valid_app_id(""));
        assert!(!is_valid_app_id("../app"));
        assert!(!is_valid_app_id("/app"));
        assert!(!is_valid_app_id(".hidden"));
        assert!(!is_valid_app_id("-flag"));
        assert!(!is_valid_app_id("app/sub"));
        assert!(!is_valid_app_id("app\\sub"));
    }
}
