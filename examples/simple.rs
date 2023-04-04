//! Run with
//!
//! ```not_rust
//! cargo run -p example-readme
//! ```

use axum::{
    body::Bytes,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();
    println!("Started server!");
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        // `POST /users` goes to `create_user`
        .route("/handle_email", post(handle_email))
        .route("/email_in", post(handle_email))
        .route("/email_event", post(handle_email));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn handle_email(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    body: Json<serde_json::Value>,
) -> impl IntoResponse {
    println!("Handling email {:?}", body);

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json("OK"))
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    println!("Creating user");
    // insert your application logic here
    let user = User {
        id: 1337,
        username: "Testuser".to_string(),
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
