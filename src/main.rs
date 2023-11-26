use axum::{extract::Query, response::IntoResponse, routing::get, Json, Router};
use reqwest::StatusCode;
use web3::{Downloader, Instagram, Utils};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(get_post));

    println!("Listening on 127.0.0.1:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_post(insta: Option<Query<Instagram>>) -> impl IntoResponse {
    let Query(insta) = insta.unwrap();
    println!("{}", insta.url);
    let insta = Instagram::new(&insta.url);
    println!("{}", insta.url);
    let post = insta.download().await.unwrap();
    (StatusCode::OK, Json(post))
}
