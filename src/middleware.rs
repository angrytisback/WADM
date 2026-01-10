use crate::api::auth::Claims;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{body::EitherBody, Error, HttpResponse};
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
            || path == "/api/health"
            || path == "/api/terminal/ws"
        {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        
        let auth_header = req.headers().get("Authorization");
        let token = match auth_header {
            Some(value) => {
                let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
                if parts.len() == 2 && parts[0] == "Bearer" {
                    parts[1]
                } else {
                    ""
                }
            }
            None => "",
        };

        if token.is_empty() {
            return Box::pin(async move {
                let res = HttpResponse::Unauthorized().body("Missing or invalid token");
                Ok(ServiceResponse::new(req.into_parts().0, res).map_into_right_body())
            });
        }

        
        
        let secret = b"super_secret_key_change_this_in_prod";

        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(_token_data) => {
                
                
                let fut = self.service.call(req);
                return Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                });
            }
            Err(_) => {
                return Box::pin(async move {
                    let res = HttpResponse::Unauthorized().body("Invalid token");
                    Ok(ServiceResponse::new(req.into_parts().0, res).map_into_right_body())
                });
            }
        }
    }
}
