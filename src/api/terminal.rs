use crate::api::auth::{Claims, JWT_SECRET};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::Mutex;
use std::thread;
use tokio::sync::mpsc;

use crate::api::config::AppConfig;

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

pub async fn ws_terminal(
    req: HttpRequest,
    stream: web::Payload,
    config_data: web::Data<Mutex<AppConfig>>,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    _query: web::Query<WsQuery>,
) -> Result<HttpResponse, Error> {
    let client_ip = crate::auth::extract_client_ip(&req);

    let token = match crate::auth::extract_token(&req) {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(HttpResponse::Unauthorized().body("Missing token")),
    };

    let validation = Validation::new(Algorithm::HS256);
    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET.as_slice()),
        &validation,
    ) {
        Ok(c) => c,
        Err(_) => return Ok(HttpResponse::Unauthorized().body("Invalid token")),
    };

    let claims = token_data.claims;

    if let Some(user_db) = req.app_data::<web::Data<std::sync::Arc<crate::auth::UserDatabase>>>() {
        let token_hash = crate::auth::hash_token(&token);
        if user_db.is_token_revoked(&token_hash, &claims.sub, claims.iat) {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Token has been revoked"
            })));
        }
    }
    if claims.role != crate::auth::UserRole::Admin {
        log::warn!(
            "Unauthorized terminal access attempt by user '{}' with role '{}'",
            claims.sub,
            claims.role
        );
        audit.log(
            &claims.sub,
            claims.role.as_str(),
            "TERMINAL_CONNECT",
            None,
            &client_ip,
            "DENIED",
            Some("Terminal access requires Admin role"),
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        })));
    }

    {
        let config = config_data.lock().unwrap_or_else(|e| e.into_inner());
        if !config.developer_mode {
            log::warn!("Attempted terminal access without Developer Mode enabled.");
            audit.log(
                &claims.sub,
                claims.role.as_str(),
                "TERMINAL_CONNECT",
                None,
                &client_ip,
                "DENIED",
                Some("Developer Mode is disabled"),
            );
            return Ok(HttpResponse::Forbidden().body("Developer Mode is disabled"));
        }
    }

    audit.log(
        &claims.sub,
        claims.role.as_str(),
        "TERMINAL_CONNECT",
        None,
        &client_ip,
        "SUCCESS",
        Some("Interactive PTY session started"),
    );

    let (res, mut session, stream) = actix_ws::handle(&req, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        .max_continuation_size(2 * 1024 * 1024);

    actix_web::rt::spawn(async move {
        let pty_system = NativePtySystem::default();

        let pair = match pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to create PTY: {}", e);
                let _ = session
                    .text("\r\n\x1b[31mError: Failed to allocate pseudo-terminal\x1b[0m\r\n")
                    .await;
                let _ = session.close(None).await;
                return;
            }
        };

        let shell = if std::path::Path::new("/bin/bash").exists()
            || std::path::Path::new("/usr/bin/bash").exists()
        {
            "bash"
        } else {
            "sh"
        };
        let cmd = CommandBuilder::new(shell);
        let mut child = match pair.slave.spawn_command(cmd) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to spawn shell: {}", e);
                let _ = session
                    .text("\r\n\x1b[31mError: Failed to spawn shell process\x1b[0m\r\n")
                    .await;
                let _ = session.close(None).await;
                return;
            }
        };

        let mut reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to clone PTY reader: {}", e);
                let _ = session
                    .text("\r\n\x1b[31mError: Failed to initialize terminal reader\x1b[0m\r\n")
                    .await;
                let _ = session.close(None).await;
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        };

        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        });

        let mut writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to take PTY writer: {}", e);
                let _ = session
                    .text("\r\n\x1b[31mError: Failed to initialize terminal writer\x1b[0m\r\n")
                    .await;
                let _ = session.close(None).await;
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        };

        loop {
            tokio::select! {
                Some(chunk) = rx.recv() => {

                    if session.binary(chunk).await.is_err() {
                        break;
                    }
                }

                Some(msg) = stream.next() => {
                    match msg {
                        Ok(AggregatedMessage::Binary(bin)) => {
                             if writer.write_all(&bin).is_err() {
                                 break;
                             }
                        }
                        Ok(AggregatedMessage::Text(text)) => {
                            if text.starts_with("RESIZE:") {
                                if let Some(dims) = text.strip_prefix("RESIZE:") {
                                    let parts: Vec<&str> = dims.split('x').collect();
                                    if parts.len() == 2 {
                                         if let (Ok(cols), Ok(rows)) = (parts[0].parse(), parts[1].parse()) {
                                             let _ = pair.master.resize(PtySize {
                                                 rows,
                                                 cols,
                                                 pixel_width: 0,
                                                 pixel_height: 0,
                                             });
                                         }
                                    }
                                }
                            } else {
                                if writer.write_all(text.as_bytes()).is_err() {
                                    break;
                                }
                            }
                        }
                        Ok(AggregatedMessage::Ping(msg)) => {
                            let _ = session.pong(&msg).await;
                        }
                        Ok(AggregatedMessage::Close(_)) => break,
                        _ => {}
                    }
                }
                else => break,
            }
        }

        // Clean up child process to prevent zombie/orphan processes
        let _ = child.kill();
        let _ = child.wait();
    });

    Ok(res)
}
