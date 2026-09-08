use std::{collections::HashMap, sync::{Mutex, mpsc::{self, Sender}}};

use actix_web::{App, HttpServer, web::{self, Data}};
use dotenvy::dotenv;

use crate::{routes::user::{balance, deposit, onramp, sign_in, sign_up}, types::user::User};

pub mod types;
pub mod routes;
pub mod middleware;
pub mod config;

enum BalanceMessage {
    Onramp(u32, u32),
    GetBalance(u32, futures::channel::oneshot::Sender<u32>)
}

struct AppState {
    user_index: Mutex<u32>,
    users: Mutex<Vec<User>>,
    stock_balances: Mutex<HashMap<u32, HashMap<String, u32>>>,
    balances_tx: Sender<BalanceMessage>
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let (tx, rx) = mpsc::channel();
    
    let app_state: Data<AppState> = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        stock_balances: Mutex::new(HashMap::new()),
        balances_tx: tx
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(sign_up)
            .service(sign_in)
            .service(balance)
            .service(onramp)
            .service(deposit)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}