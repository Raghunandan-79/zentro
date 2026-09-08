use actix_web::http::header::HeaderValue;
use actix_web::{
    FromRequest, HttpRequest, HttpResponse, ResponseError, dev::Payload, error::Error,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use std::fmt;
use std::future::{Ready, ready};

use crate::config::jwt_secret;
use crate::types::user::Claims;

#[derive(Debug)]
pub struct AuthError;

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid or missing token")
    }
}

impl ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Invalid or missing token"
        }))
    }
}

pub struct AuthUser(pub u32);

impl FromRequest for AuthUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let token: Option<String> = req
            .headers()
            .get("Authorization")
            .and_then(|value: &HeaderValue| value.to_str().ok())
            .map(|value: &str| value.trim_start_matches("Bearer ").trim().to_string());

        let token: String = match token {
            Some(token) if !token.is_empty() => token,
            _ => return ready(Err(AuthError.into())),
        };

        let decoded: Result<jsonwebtoken::TokenData<Claims>, jsonwebtoken::errors::Error> =
            decode::<Claims>(
                &token,
                &DecodingKey::from_secret(jwt_secret().as_bytes()),
                &Validation::default(),
            );

        match decoded {
            Ok(data) => ready(Ok(AuthUser(data.claims.sub))),
            Err(_) => ready(Err(AuthError.into())),
        }
    }
}
