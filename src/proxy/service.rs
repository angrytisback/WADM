use actix_web::{http::header::HeaderName, web, HttpRequest, HttpResponse};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use std::str::FromStr;

const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

pub fn is_websocket_request(req: &HttpRequest) -> bool {
    req.headers()
        .get("upgrade")
        .and_then(|h| h.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

pub async fn proxy_http(
    req: HttpRequest,
    body: web::Payload,
    target_base_url: &str,
    strip_prefix: Option<&str>,
) -> Result<HttpResponse, actix_web::Error> {
    let req_path = req.uri().path();
    let forwarded_path = if let Some(prefix) = strip_prefix {
        if let Some(rem) = req_path.strip_prefix(prefix) {
            if rem.is_empty() {
                "/"
            } else {
                rem
            }
        } else {
            req_path
        }
    } else {
        req_path
    };

    let query_str = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();

    let full_target = format!(
        "{}{}{}",
        target_base_url.trim_end_matches('/'),
        forwarded_path,
        query_str
    );

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let method = reqwest::Method::from_bytes(req.method().as_str().as_bytes())
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    let mut forward_req = client.request(method, &full_target);

    // Copy client headers (excluding hop-by-hop)
    for (name, val) in req.headers() {
        let name_str = name.as_str().to_lowercase();
        if !HOP_BY_HOP_HEADERS.contains(&name_str.as_str()) && name_str != "host" {
            if let Ok(v_str) = val.to_str() {
                forward_req = forward_req.header(name.as_str(), v_str);
            }
        }
    }

    // Set standard reverse proxy headers
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("127.0.0.1")
        .to_string();
    let host_hdr = req.connection_info().host().to_string();
    let proto = req.connection_info().scheme().to_string();

    forward_req = forward_req
        .header("X-Forwarded-For", client_ip)
        .header("X-Forwarded-Host", host_hdr.clone())
        .header("X-Forwarded-Proto", proto)
        .header(
            "X-Real-IP",
            req.connection_info()
                .realip_remote_addr()
                .unwrap_or("127.0.0.1"),
        );

    if let Some(prefix) = strip_prefix {
        forward_req = forward_req.header("X-Forwarded-Prefix", prefix);
    }

    // Stream request body across thread boundary using mpsc channel and unfold
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<actix_web::web::Bytes, std::io::Error>>(16);
    let mut payload = body.into_inner();
    actix_web::rt::spawn(async move {
        use futures_util::StreamExt;
        while let Some(chunk) = payload.next().await {
            match chunk {
                Ok(bytes) => {
                    if tx.send(Ok(bytes)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(std::io::Error::other(e))).await;
                    break;
                }
            }
        }
    });

    let body_stream = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    forward_req = forward_req.body(reqwest::Body::wrap_stream(body_stream));

    // Send request to target container
    let backend_resp = match forward_req.send().await {
        Ok(res) => res,
        Err(e) => {
            log::warn!("Reverse proxy target '{}' unreachable: {}", full_target, e);
            return Ok(HttpResponse::BadGateway().json(serde_json::json!({
                "error": "Bad Gateway",
                "message": format!(
                    "The requested service at {} is currently unreachable. The container may still be initializing.",
                    target_base_url
                )
            })));
        }
    };

    // Build client response
    let status_code = actix_web::http::StatusCode::from_u16(backend_resp.status().as_u16())
        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

    let mut response_builder = HttpResponse::build(status_code);

    for (name, val) in backend_resp.headers() {
        let name_str = name.as_str().to_lowercase();
        if !HOP_BY_HOP_HEADERS.contains(&name_str.as_str()) {
            if let (Ok(h_name), Ok(h_val)) = (HeaderName::from_str(name.as_str()), val.to_str()) {
                response_builder.append_header((h_name, h_val));
            }
        }
    }

    let stream = backend_resp
        .bytes_stream()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()));

    Ok(response_builder.streaming(stream))
}

pub async fn proxy_websocket(
    req: HttpRequest,
    body: web::Payload,
    target_base_url: &str,
    strip_prefix: Option<&str>,
) -> Result<HttpResponse, actix_web::Error> {
    let req_path = req.uri().path();
    let forwarded_path = if let Some(prefix) = strip_prefix {
        if let Some(rem) = req_path.strip_prefix(prefix) {
            if rem.is_empty() {
                "/"
            } else {
                rem
            }
        } else {
            req_path
        }
    } else {
        req_path
    };

    let query_str = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();

    let target_ws_base = target_base_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    let target_ws_url = format!(
        "{}{}{}",
        target_ws_base.trim_end_matches('/'),
        forwarded_path,
        query_str
    );

    // Handshake with client
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    // Connect to backend WebSocket server
    actix_web::rt::spawn(async move {
        match tokio_tungstenite::connect_async(&target_ws_url).await {
            Ok((ws_stream, _)) => {
                let (mut ws_write, mut ws_read) = ws_stream.split();

                loop {
                    tokio::select! {
                        client_msg = msg_stream.next() => {
                            match client_msg {
                                Some(Ok(actix_ws::Message::Text(t))) => {
                                    if ws_write.send(tokio_tungstenite::tungstenite::Message::Text(t.to_string().into())).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(actix_ws::Message::Binary(b))) => {
                                    if ws_write.send(tokio_tungstenite::tungstenite::Message::Binary(b)).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(actix_ws::Message::Ping(p))) => {
                                    if ws_write.send(tokio_tungstenite::tungstenite::Message::Ping(p)).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(actix_ws::Message::Pong(p))) => {
                                    if ws_write.send(tokio_tungstenite::tungstenite::Message::Pong(p)).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(actix_ws::Message::Close(c))) => {
                                    let frame = c.map(|cf| tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(u16::from(cf.code)),
                                        reason: cf.description.unwrap_or_default().into(),
                                    });
                                    let _ = ws_write.send(tokio_tungstenite::tungstenite::Message::Close(frame)).await;
                                    break;
                                }
                                _ => break,
                            }
                        }
                        backend_msg = ws_read.next() => {
                            match backend_msg {
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Text(t))) => {
                                    if session.text(t.to_string()).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(b))) => {
                                    if session.binary(b).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(p))) => {
                                    if session.ping(&p).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(p))) => {
                                    if session.pong(&p).await.is_err() {
                                        break;
                                    }
                                }
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Close(c))) => {
                                    let reason = c.map(|cf| actix_ws::CloseReason {
                                        code: actix_ws::CloseCode::from(u16::from(cf.code)),
                                        description: Some(cf.reason.to_string()),
                                    });
                                    let _ = session.close(reason).await;
                                    break;
                                }
                                _ => break,
                            }
                        }
                        else => break,
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to establish backend WebSocket proxy connection to '{}': {}",
                    target_ws_url,
                    e
                );
                let _ = session
                    .close(Some(actix_ws::CloseReason {
                        code: actix_ws::CloseCode::Error,
                        description: Some("Backend WebSocket service unavailable".to_string()),
                    }))
                    .await;
            }
        }
    });

    Ok(response)
}
