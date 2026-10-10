use crate::api::auth::Claims;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{body::EitherBody, Error, HttpMessage, HttpResponse};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use std::rc::Rc;
use std::task::{Context, Poll};

pub struct Auth;

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct AuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path();

        if path == "/api/auth/status"
            || path == "/api/auth/login"
            || path == "/api/auth/setup/init"
            || path == "/api/auth/setup/confirm"
            || path == "/api/auth/logout"
            || path == "/api/health"
            || path == "/api/cluster/tunnel"
        {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        // 1. Check HttpOnly cookie first
        let token = if let Some(cookie) = req.cookie("wadm_token") {
            let val = cookie.value().trim().to_string();
            if !val.is_empty() {
                Some(val)
            } else {
                None
            }
        } else {
            None
        };

        // 2. Check Authorization Bearer header
        let token = token.or_else(|| {
            req.headers().get("Authorization").and_then(|value| {
                let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
                if parts.len() == 2 && parts[0].eq_ignore_ascii_case("bearer") {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            })
        });

        // 3. Fallback for WebSocket & SSE
        let token = token.or_else(|| {
            if path == "/api/terminal/ws"
                || path == "/api/stats/stream"
                || (path.starts_with("/api/jobs/") && path.ends_with("/stream"))
            {
                if let Some(proto) = req.headers().get("Sec-WebSocket-Protocol") {
                    let p = proto.to_str().unwrap_or("").trim().to_string();
                    if !p.is_empty() {
                        return Some(p);
                    }
                }
                let query = req.query_string();
                for pair in query.split('&') {
                    let mut kv = pair.split('=');
                    if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                        if k == "token" && !v.is_empty() {
                            return Some(v.to_string());
                        }
                    }
                }
            }
            None
        });

        let token = match token {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Box::pin(async move {
                    let res = HttpResponse::Unauthorized().json(serde_json::json!({
                        "error": "Missing or invalid token"
                    }));
                    Ok(ServiceResponse::new(req.into_parts().0, res).map_into_right_body())
                });
            }
        };

        let secret = crate::api::auth::JWT_SECRET.as_slice();

        match decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(token_data) => {
                // Check if token has been revoked
                if let Some(user_db) = req
                    .app_data::<actix_web::web::Data<std::sync::Arc<crate::auth::UserDatabase>>>()
                {
                    let token_hash = crate::auth::hash_token(&token);
                    if user_db.is_token_revoked(
                        &token_hash,
                        &token_data.claims.sub,
                        token_data.claims.iat,
                    ) {
                        return Box::pin(async move {
                            let res = HttpResponse::Unauthorized().json(serde_json::json!({
                                "error": "Token has been revoked"
                            }));
                            Ok(ServiceResponse::new(req.into_parts().0, res).map_into_right_body())
                        });
                    }
                }

                req.extensions_mut().insert(token_data.claims);
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            Err(_) => Box::pin(async move {
                let res = HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Invalid token"
                }));
                Ok(ServiceResponse::new(req.into_parts().0, res).map_into_right_body())
            }),
        }
    }
}
