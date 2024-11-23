use crate::utils::prompt_user;

use axum::{
    extract::State,
    http::Method,
    routing::{get, options, post},
    Json, Router,
};

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

struct AppState {
    authorized_tokens: Mutex<Vec<Uuid>>,
}

async fn api_root() -> Json<Value> {
    return Json(json!({
        "service": "cocogoat-control-rs",
        "version": env!("CARGO_PKG_VERSION")
    }));
}

async fn api_token(header_map: axum::http::HeaderMap, State(state): State<Arc<AppState>>) -> Json<Value> {
    let url = header_map.get("Origin").unwrap().to_str().unwrap();
    let message = format!(
        "Request from {}\nAre you sure you want to generate a new token? [y/N] ",
        url
    );
    if prompt_user(&message) {
        let id = Uuid::new_v4();
        state.authorized_tokens.lock().unwrap().push(id);
        return Json(json!({
            "hwnd": 114514, // TODO!
            "origin": url,
            "swapEffectUpgrade": false, // TODO!
            "token": id.to_string(),
            "winver": 11 // TODO!
        }));
    } else {
        return Json(json!({
            "message": "Operation cancelled by the user"
        }));
    }
}

pub async fn start_server() {
    println!("Server running on http://localhost:32333");

    let shared_state = Arc::new(AppState {
        authorized_tokens: Mutex::new(vec![]),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any) // 允许所有来源
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS]) // 允许的 HTTP 方法
        .allow_headers(Any); // 允许携带身份验证信息（如 Cookie、Authorization）

    let app = Router::new()
        .route("/", get(api_root))
        .route("/token", post(api_token))
        .route("/", options(|| async { "" }))
        .layer(cors)
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:32333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
