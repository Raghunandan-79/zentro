use std::{
    collections::HashMap,
    sync::{
        Mutex,
        mpsc::{self, Sender},
    },
    thread::spawn,
};

use actix_web::{
    App, HttpServer,
    web::{self, Data},
};
use dotenvy::dotenv;

use crate::{
    BalanceMessage::{GetBalance, Onramp},
    routes::user::{balance, deposit, onramp, sign_in, sign_up},
    types::user::User,
};

pub mod config;
pub mod middleware;
pub mod routes;
pub mod types;

enum BalanceMessage {
    Onramp(u32, u32),
    GetBalance(u32, futures::channel::oneshot::Sender<u32>),
}

enum StockBalanceMessage {
    InitUser(u32),
    Deposit(u32, String, u32),
    GetBalances(u32, futures::channel::oneshot::Sender<HashMap<String, u32>>),
}

struct AppState {
    user_index: Mutex<u32>,
    users: Mutex<Vec<User>>,
    stock_balances_tx: Sender<StockBalanceMessage>,
    balances_tx: Sender<BalanceMessage>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let (tx, rx) = mpsc::channel();
    let (stock_tx, stock_rx) = mpsc::channel();

    let app_state: Data<AppState> = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        stock_balances_tx: stock_tx,
        balances_tx: tx,
    });

    spawn(move || {
        let mut balances: HashMap<u32, u32> = HashMap::new();

        while let Ok(message) = rx.recv() {
            match message {
                Onramp(user_id, amount) => {
                    let existing_amount: &u32 = balances.get(&user_id).unwrap_or(&0);
                    balances.insert(user_id, amount + existing_amount);
                }
                GetBalance(user_id, tx) => {
                    let user_balance: &u32 = balances.get(&user_id).unwrap_or(&0);
                    let _ = tx.send(*user_balance);
                }
            }
        }
    });

    spawn(move || {
        let mut stock_balances: HashMap<u32, HashMap<String, u32>> = HashMap::new();

        while let Ok(message) = stock_rx.recv() {
            match message {
                StockBalanceMessage::InitUser(user_id) => {
                    stock_balances.insert(user_id, HashMap::new());
                }
                StockBalanceMessage::Deposit(user_id, symbol, qty) => {
                    let user_balances: &mut HashMap<String, u32> = stock_balances.entry(user_id).or_insert_with(HashMap::new);
                    let existing_balance = *user_balances.get(&symbol).unwrap_or(&0);
                    user_balances.insert(symbol, existing_balance + qty);
                }
                StockBalanceMessage::GetBalances(user_id, tx) => {
                    let user_balances: HashMap<String, u32> = stock_balances.get(&user_id).cloned().unwrap_or_default();
                    let _ = tx.send(user_balances);
                }
            }
        }
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
