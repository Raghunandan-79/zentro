use std::sync::{Mutex, MutexGuard};

use actix_web::{
    App, HttpResponse, HttpServer, Responder, post,
    web::{self, Data, Json},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SignupInput {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
struct SignupResponse {
    message: String,
}

struct User {
    id: u32,
    username: String,
    password: String,
}

struct AppState {
    user_index: Mutex<u32>,
    users: Mutex<Vec<User>>,
}

#[post("/signup")]
async fn sign_up(body: Json<SignupInput>, app_state: web::Data<AppState>) -> impl Responder {
    let mut users: MutexGuard<'_, Vec<User>> = app_state.users.lock().unwrap();
    let mut users_index: MutexGuard<'_, u32> = app_state.user_index.lock().unwrap();

    let user_found: Option<&User> = users.iter().find(|u: &&User| u.username == body.username);

    if user_found.is_none() {
        *users_index = *users_index + 1;
        users.push(User {
            id: users_index.clone(),
            username: body.username.clone(),
            password: body.password.clone(),
        });

        println!("{}", users.len());

        drop(users); // unlocking the users

        return HttpResponse::Ok().json(SignupResponse {
            message: String::from("Successfully signed up"),
        });
    }

    HttpResponse::Unauthorized().json(SignupResponse {
        message: String::from("User already exists"),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state: Data<AppState> = web::Data::new(AppState {
        user_index: Mutex::new(0),
        users: Mutex::new(vec![]),
    });

    HttpServer::new(move || 
        App::new().app_data(
            app_state.clone()
        )
        .service(sign_up))
        .bind(("127.0.0.1", 3001))?
        .run()
        .await
}
