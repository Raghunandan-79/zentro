use actix_web::{App, HttpResponse, HttpServer, Responder, post, web::{self, Data, Json}};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SignupInput {
    pub username: String,
    pub password: String,
}

#[post("/signup")]
async fn sign_up(body: Json<SignupInput>, app_state: web::Data<AppState>) -> impl Responder {
    println!("{}", body.username);
    println!("{}", body.password);
    println!("{}", app_state.users.len());

    HttpResponse::Ok().body("Hello World!")
}

struct User {
    id: u32,
    username: String,
    password: String
}

struct AppState {
    users: Vec<User>
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state: Data<AppState> = web::Data::new(AppState {
        users: vec![]
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(sign_up)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
