use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectPortRequirement {
    pub port: u16,
    pub protocol: String, // "TCP" | "UDP"
    pub reason: String,   // "WireGuard VPN handshake and traffic"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppPortConfig {
    pub internal_web_port: Option<u16>, // Internal loopback web port targeted by reverse proxy
    pub exposed_network_ports: Vec<DirectPortRequirement>, // Network ports requiring direct external exposure (VPN, DNS, etc.)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub ports: AppPortConfig,
    pub compose_template: String,
    pub default_access_mode: String, // "path" | "subdomain" | "none"
}

pub fn get_app_templates() -> Vec<AppTemplate> {
    vec![
        AppTemplate {
            id: "nextcloud".to_string(),
            name: "Nextcloud".to_string(),
            description: "Self-hosted productivity platform, file synchronization, and collaborative office.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/commons/6/60/Nextcloud_Logo.svg".to_string(),
            ports: AppPortConfig {
                internal_web_port: Some(80),
                exposed_network_ports: vec![],
            },
            default_access_mode: "path".to_string(),
            compose_template: r#"version: '3'
services:
  app:
    image: nextcloud
    restart: always
    ports:
      - "127.0.0.1:{{INTERNAL_PORT}}:80"
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
            description: "Network-wide Ad and Tracker Blocking DNS sinkhole with Web Admin Dashboard.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/en/1/15/Pi-hole_vector_logo.svg".to_string(),
            ports: AppPortConfig {
                internal_web_port: Some(80),
                exposed_network_ports: vec![
                    DirectPortRequirement {
                        port: 53,
                        protocol: "UDP".to_string(),
                        reason: "DNS query resolution for network clients".to_string(),
                    },
                    DirectPortRequirement {
                        port: 53,
                        protocol: "TCP".to_string(),
                        reason: "DNS TCP queries and zone transfers".to_string(),
                    },
                ],
            },
            default_access_mode: "path".to_string(),
            compose_template: r#"version: '3'
services:
  pihole:
    container_name: pihole
    image: pihole/pihole:latest
    ports:
      - "127.0.0.1:{{INTERNAL_PORT}}:80/tcp"
      - "53:53/tcp"
      - "53:53/udp"
    environment:
      TZ: 'UTC'
      WEBPASSWORD: '{{SECURE_PASSWORD}}'
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
            id: "wireguard".to_string(),
            name: "WireGuard VPN".to_string(),
            description: "Fast, modern, secure VPN tunnel with state-of-the-art cryptography.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/commons/e/e0/WireGuard_logo.svg".to_string(),
            ports: AppPortConfig {
                internal_web_port: None,
                exposed_network_ports: vec![DirectPortRequirement {
                    port: 51820,
                    protocol: "UDP".to_string(),
                    reason: "WireGuard VPN handshake and encrypted tunnel traffic".to_string(),
                }],
            },
            default_access_mode: "none".to_string(),
            compose_template: r#"version: '3.8'
services:
  wireguard:
    image: linuxserver/wireguard:latest
    container_name: wireguard
    cap_add:
      - NET_ADMIN
      - SYS_MODULE
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=UTC
      - SERVERPORT=51820
    volumes:
      - wireguard_config:/config
    ports:
      - "51820:51820/udp"
    sysctls:
      - net.ipv4.conf.all.src_valid_mark=1
    restart: unless-stopped
volumes:
  wireguard_config:
"#
            .to_string(),
        },
        AppTemplate {
            id: "vaultwarden".to_string(),
            name: "Vaultwarden (Bitwarden)".to_string(),
            description: "Lightweight, secure self-hosted password manager compatible with Bitwarden clients.".to_string(),
            icon: "https://raw.githubusercontent.com/dani-garcia/vaultwarden/main/resources/vaultwarden-icon.svg".to_string(),
            ports: AppPortConfig {
                internal_web_port: Some(80),
                exposed_network_ports: vec![],
            },
            default_access_mode: "path".to_string(),
            compose_template: r#"version: '3'
services:
  vaultwarden:
    image: vaultwarden/server:latest
    container_name: vaultwarden
    restart: always
    ports:
      - "127.0.0.1:{{INTERNAL_PORT}}:80"
    volumes:
      - vaultwarden_data:/data
volumes:
  vaultwarden_data:
"#
            .to_string(),
        },
        AppTemplate {
            id: "jellyfin".to_string(),
            name: "Jellyfin".to_string(),
            description: "Free and open-source media streaming system for movies, shows, and music.".to_string(),
            icon: "https://raw.githubusercontent.com/jellyfin/jellyfin-ux/master/branding/SVG/icon-transparent.svg".to_string(),
            ports: AppPortConfig {
                internal_web_port: Some(8096),
                exposed_network_ports: vec![],
            },
            default_access_mode: "path".to_string(),
            compose_template: r#"version: '3'
services:
  jellyfin:
    image: jellyfin/jellyfin:latest
    container_name: jellyfin
    restart: unless-stopped
    ports:
      - "127.0.0.1:{{INTERNAL_PORT}}:8096"
    volumes:
      - jellyfin_config:/config
      - jellyfin_cache:/cache
volumes:
  jellyfin_config:
  jellyfin_cache:
"#
            .to_string(),
        },
        AppTemplate {
            id: "wordpress".to_string(),
            name: "WordPress".to_string(),
            description: "Build a modern website, blog, or store with integrated MariaDB backend.".to_string(),
            icon: "https://upload.wikimedia.org/wikipedia/commons/9/93/Wordpress_Blue_logo.png".to_string(),
            ports: AppPortConfig {
                internal_web_port: Some(80),
                exposed_network_ports: vec![],
            },
            default_access_mode: "path".to_string(),
            compose_template: r#"version: '3'
services:
  wordpress:
    image: wordpress
    restart: always
    ports:
      - "127.0.0.1:{{INTERNAL_PORT}}:80"
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: wp
      WORDPRESS_DB_PASSWORD: '{{SECURE_PASSWORD}}'
      WORDPRESS_DB_NAME: wordpress
    volumes:
      - wordpress_data:/var/www/html
  db:
    image: mariadb
    restart: always
    environment:
      MYSQL_DATABASE: wordpress
      MYSQL_USER: wp
      MYSQL_PASSWORD: '{{SECURE_PASSWORD}}'
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
            id: "portainer".to_string(),
            name: "Portainer CE".to_string(),
            description: "Web-based container management interface for Docker engines and swarms.".to_string(),
            icon: "https://www.portainer.io/hubfs/Portainer%20Icon%20Colour.svg".to_string(),
            ports: AppPortConfig {
                internal_web_port: Some(9000),
                exposed_network_ports: vec![],
            },
            default_access_mode: "path".to_string(),
            compose_template: r#"version: '3.8'
services:
  portainer:
    image: portainer/portainer-ce:latest
    container_name: portainer
    restart: always
    ports:
      - "127.0.0.1:{{INTERNAL_PORT}}:9000"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_templates_validity() {
        let templates = get_app_templates();
        assert!(!templates.is_empty());

        for t in &templates {
            assert!(!t.id.is_empty());
            assert!(!t.name.is_empty());

            // All web apps must bind strictly to 127.0.0.1
            if t.ports.internal_web_port.is_some() {
                assert!(
                    t.compose_template.contains("127.0.0.1:{{INTERNAL_PORT}}"),
                    "App '{}' must bind internal web port to 127.0.0.1:{{{{INTERNAL_PORT}}}}",
                    t.id
                );
            }
        }
    }
}
