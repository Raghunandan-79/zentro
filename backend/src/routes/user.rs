use std::{collections::HashMap, sync::{MutexGuard}};

use actix_web::{HttpResponse, Responder, get, post, web::{self, Json}};
use chrono::{Duration, Utc};
use futures::channel::oneshot;
use jsonwebtoken::{EncodingKey, Header, encode};

use crate::{AppState, BalanceMessage::Onramp, StockBalanceMessage, config::jwt_secret, middleware::AuthUser, types::user::{BalanceResponse, Claims, DepositRequest, DespositResponse, OnRampRequest, SigninInput, SigninResponse, SignupInput, SignupResponse, User}};

#[post("/signup")]
async fn sign_up(body: Json<SignupInput>, app_state: web::Data<AppState>) -> impl Responder {
    let mut users: MutexGuard<'_, Vec<User>> = app_state.users.lock().unwrap();
    let mut user_index: MutexGuard<'_, u32> = app_state.user_index.lock().unwrap();

    let user_found: Option<&User> = users.iter().find(|u: &&User| u.username == body.username);

    if user_found.is_none() {
        *user_index = *user_index + 1;
        users.push(User {
            id: user_index.clone(),
            username: body.username.clone(),
            password: body.password.clone()
        });

        app_state.balances_tx.send(Onramp(user_index.clone(), 0));
        app_state.stock_balances_tx.send(StockBalanceMessage::InitUser(*user_index));

        HttpResponse::Ok().json(SignupResponse {
            message: String::from("Successfully signed up")
        })
    } else {
        HttpResponse::Unauthorized().json(SignupResponse {
            message: String::from("User already")
        })
    }
}

#[post("/signin")]
pub async fn sign_in(app_state: web::Data<AppState>, body: Json<SigninInput>) -> impl Responder {
    let users: MutexGuard<'_, Vec<User>> = app_state.users.lock().unwrap();
    let user_found: Option<&User> = users.iter().find(|u: &&User| u.username == body.username && u.password == body.password);

    if user_found.is_none() {
        return HttpResponse::Unauthorized().json(SignupResponse {
            message: String::from("Incorrect credentials")
        });
    }

    let user: &User = user_found.unwrap();

    let exp: usize = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims: Claims = Claims {
        sub: user.id,
        exp
    };

    let token: String = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes())
    ).unwrap();
    
    HttpResponse::Ok().json(SigninResponse {
        token
    })
}


#[get("/balance")]
pub async fn balance(app_state: web::Data<AppState>, user: AuthUser) -> impl Responder {
    let user_id: u32 = user.0;
    let (tx, rx) = oneshot::channel::<u32>();
    app_state.balances_tx.send(crate::BalanceMessage::GetBalance(user_id, tx));

    let usd_balance: u32 = rx.await.unwrap();

    let (stock_tx, stock_rx) = oneshot::channel::<HashMap<String, u32>>();
    app_state.stock_balances_tx.send(StockBalanceMessage::GetBalances(user_id, stock_tx));
    let stock_balances: HashMap<String, u32> = stock_rx.await.unwrap();

    HttpResponse::Ok().json(BalanceResponse {
        usd_balance: usd_balance,
        stock_balances: stock_balances
    })
}

#[post("/onramp")]
pub async fn onramp(app_state: web::Data<AppState>, user: AuthUser, body: Json<OnRampRequest>) -> impl Responder {
    let user_id: u32 = user.0;
    app_state.balances_tx.send(crate::BalanceMessage::Onramp(user_id, body.qty));

    HttpResponse::Ok()
}

#[post("/deposit/{asset_symbol}")]
pub async fn deposit(app_state: web::Data<AppState>, user: AuthUser, symbol: web::Path<String>, body: Json<DepositRequest>) -> impl Responder {
    let user_id: u32 = user.0;
    let symbol: String = symbol.into_inner();

    app_state.stock_balances_tx.send(StockBalanceMessage::Deposit(user_id, symbol, body.qty));

    HttpResponse::Ok().json(DespositResponse {
        message: String::from("Successfully deposited")
    })
}

#[post("/order")]
pub async fn order() -> impl Responder {
    HttpResponse::Ok()
}

#[post("/cancel")]
pub async fn cancel() -> impl Responder {
    HttpResponse::Ok()
}
