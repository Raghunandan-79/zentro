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
    routes::user::{balance, deposit, onramp, order, sign_in, sign_up},
    types::user::User,
};

pub mod config;
pub mod middleware;
pub mod routes;
pub mod types;

#[derive(Clone)]
struct Balance {
    available: u32,
    locked: u32,
}

enum BalanceMessage {
    Onramp(u32, u32),
    GetBalance(u32, futures::channel::oneshot::Sender<u32>),
    LockFunds(u32, u32),
    UnlockFunds(u32, u32),
    TransferAvailable(u32, u32, u32), // from_user, to_user, amount
    GetAvailable(u32, futures::channel::oneshot::Sender<u32>),
}

enum StockBalanceMessage {
    InitUser(u32),
    Deposit(u32, String, u32),
    GetBalances(u32, futures::channel::oneshot::Sender<HashMap<String, u32>>),
    LockStock(u32, String, u32),
    UnlockStock(u32, String, u32),
    TransferStock(u32, u32, String, u32), // from_user, to_user, asset, qty
    GetAvailable(u32, String, futures::channel::oneshot::Sender<u32>),
}

enum OrderFill {
    Fill {
        buyer: u32,
        seller: u32,
        price: u32,
        qty: u32,
    },
    OrderbookUpdate {
        price: u32,
        qty: u32,
    },
}

enum OrderMessage {
    PlaceOrder {
        user_id: u32,
        side: String,
        price: u32,
        qty: u32,
        asset: String,
        response_tx: futures::channel::oneshot::Sender<Vec<OrderFill>>,
    },
}

struct AppState {
    user_index: Mutex<u32>,
    users: Mutex<Vec<User>>,
    stock_balances_tx: Sender<StockBalanceMessage>,
    balances_tx: Sender<BalanceMessage>,
    order_tx: Sender<OrderMessage>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let (tx, rx) = mpsc::channel();
    let (stock_tx, stock_rx) = mpsc::channel();
    let (order_tx, order_rx) = mpsc::channel();

    let app_state: Data<AppState> = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        stock_balances_tx: stock_tx,
        balances_tx: tx,
        order_tx,
    });

    spawn(move || {
        let mut balances: HashMap<u32, Balance> = HashMap::new();

        while let Ok(message) = rx.recv() {
            match message {
                Onramp(user_id, amount) => {
                    let bal = balances.entry(user_id).or_insert(Balance {
                        available: 0,
                        locked: 0,
                    });
                    bal.available += amount;
                }
                GetBalance(user_id, tx) => {
                    let bal = balances.get(&user_id).cloned().unwrap_or(Balance {
                        available: 0,
                        locked: 0,
                    });
                    let _ = tx.send(bal.available + bal.locked);
                }
                BalanceMessage::LockFunds(user_id, amount) => {
                    if let Some(bal) = balances.get_mut(&user_id) {
                        bal.available -= amount;
                        bal.locked += amount;
                    }
                }
                BalanceMessage::UnlockFunds(user_id, amount) => {
                    if let Some(bal) = balances.get_mut(&user_id) {
                        bal.locked -= amount;
                        bal.available += amount;
                    }
                }
                BalanceMessage::TransferAvailable(from_user, to_user, amount) => {
                    if let Some(from_balance) = balances.get_mut(&from_user) {
                        from_balance.available -= amount;
                    }
                    let to_balance = balances.entry(to_user).or_insert(Balance {
                        available: 0,
                        locked: 0,
                    });
                    to_balance.available += amount;
                }
                BalanceMessage::GetAvailable(user_id, tx) => {
                    let available = balances
                        .get(&user_id)
                        .map(|b| b.available)
                        .unwrap_or(0);
                    let _ = tx.send(available);
                }
            }
        }
    });

    spawn(move || {
        let mut stock_balances: HashMap<u32, HashMap<String, Balance>> = HashMap::new();

        while let Ok(message) = stock_rx.recv() {
            match message {
                StockBalanceMessage::InitUser(user_id) => {
                    stock_balances.insert(user_id, HashMap::new());
                }
                StockBalanceMessage::Deposit(user_id, symbol, qty) => {
                    let user_balances: &mut HashMap<String, Balance> =
                        stock_balances.entry(user_id).or_insert_with(HashMap::new);
                    let bal = user_balances.entry(symbol).or_insert(Balance {
                        available: 0,
                        locked: 0,
                    });
                    bal.available += qty;
                }
                StockBalanceMessage::GetBalances(user_id, tx) => {
                    let user_balances: HashMap<String, u32> = stock_balances
                        .get(&user_id)
                        .map(|balances| {
                            balances
                                .iter()
                                .map(|(k, v)| (k.clone(), v.available + v.locked))
                                .collect()
                        })
                        .unwrap_or_default();
                    let _ = tx.send(user_balances);
                }
                StockBalanceMessage::LockStock(user_id, asset, qty) => {
                    if let Some(user_balances) = stock_balances.get_mut(&user_id) {
                        if let Some(bal) = user_balances.get_mut(&asset) {
                            bal.available -= qty;
                            bal.locked += qty;
                        }
                    }
                }
                StockBalanceMessage::UnlockStock(user_id, asset, qty) => {
                    if let Some(user_balances) = stock_balances.get_mut(&user_id) {
                        if let Some(bal) = user_balances.get_mut(&asset) {
                            bal.locked -= qty;
                            bal.available += qty;
                        }
                    }
                }
                StockBalanceMessage::TransferStock(from_user, to_user, asset, qty) => {
                    if let Some(from_balances) = stock_balances.get_mut(&from_user) {
                        if let Some(bal) = from_balances.get_mut(&asset) {
                            bal.locked -= qty;
                        }
                    }
                    let to_balances = stock_balances
                        .entry(to_user)
                        .or_insert_with(HashMap::new);
                    let to_balance = to_balances.entry(asset).or_insert(Balance {
                        available: 0,
                        locked: 0,
                    });
                    to_balance.available += qty;
                }
                StockBalanceMessage::GetAvailable(user_id, asset, tx) => {
                    let available = stock_balances
                        .get(&user_id)
                        .and_then(|balances| balances.get(&asset))
                        .map(|b| b.available)
                        .unwrap_or(0);
                    let _ = tx.send(available);
                }
            }
        }
    });

    // Simplified orderbook - just returns fills immediately
    spawn(move || {
        // In a real implementation, you'd maintain order books here
        // For now, we'll just simulate immediate fills
        while let Ok(message) = order_rx.recv() {
            match message {
                OrderMessage::PlaceOrder { response_tx, .. } => {
                    // Simplified: return empty fills (no matching orders)
                    // In production, this would match against orderbook
                    let _ = response_tx.send(vec![]);
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
            .service(order)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
