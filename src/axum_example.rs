use axum::{
    body::{Bytes, Full},
    handler::post,
    http::{Response, StatusCode},
    Router,
};

async fn receive_post_request(Full(body): Full<Bytes>) -> Response<Full<Bytes>> {
    println!("Received post request: {:?}", body);
    Response::new(Full(body))
}

#[tokio::main]
async fn main() {
    println!("Started server");
    let app = Router::new().route("/", post(receive_post_request));
    println!("Setup universal route");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
