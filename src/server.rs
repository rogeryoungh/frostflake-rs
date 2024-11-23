use crate::utils::prompt_user;

use axum::{
    extract::{ws::{Message, WebSocket}, MatchedPath, State, WebSocketUpgrade},
    http::{HeaderMap, Method, StatusCode},
    response::Response,
    routing::{get, options, post},
    Json, Router,
};

use serde_json::{json, Value};
use std::{
    collections::HashSet, os::windows, sync::{Arc, Mutex}
};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

struct AppState {
    authorized_tokens: Mutex<HashSet<Uuid>>,
}

impl AppState {
    fn insert(&self, token: Uuid) {
        self.authorized_tokens.lock().unwrap().insert(token);
    }
    fn contains(&self, token: &Uuid) -> bool {
        return self.authorized_tokens.lock().unwrap().contains(&token);
    }
}

async fn api_root() -> Json<Value> {
    return Json(json!({
        "service": "cocogoat-control-rs",
        "version": env!("CARGO_PKG_VERSION")
    }));
}

async fn api_token(header_map: HeaderMap, State(state): State<Arc<AppState>>) -> Json<Value> {
    let url = header_map.get("Origin").unwrap().to_str().unwrap();
    let message = format!(
        "Request from {}\nAre you sure you want to generate a new token? [y/N] ",
        url
    );
    if prompt_user(&message) {
        let id = Uuid::new_v4();
        state.insert(id);
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

async fn api_ws(path: MatchedPath, ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    let path = path.as_str().to_string();
    let uuid_str = path.split('/').last().unwrap();
    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
        if state.contains(&uuid) {
            return ws.on_upgrade(handle_ws);
        }
    }
    return Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(axum::body::Body::empty())
        .unwrap();
}

async fn handle_ws(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let ret = if let Ok(msg) = msg {
            // let data = Value::from(msg.into_text().unwrap());
            "Hello, World!"
        } else {
            // client disconnected
            return;
        };

        if socket.send(Message::Text(ret.to_string())).await.is_err() {
            // client disconnected
            return;
        }
    }
}

pub async fn start_server() {
    println!("Server running on http://localhost:32333");

    let shared_state = Arc::new(AppState {
        authorized_tokens: Mutex::new(HashSet::new()),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any) // 允许所有来源
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS]) // 允许的 HTTP 方法
        .allow_headers(Any); // 允许携带身份验证信息（如 Cookie、Authorization）

    let app = Router::new()
        .route("/", get(api_root))
        .route("/token", post(api_token))
        .route("/", options(|| async { "" }))
        .route("/ws/:uuid", get(api_ws))
        .layer(cors)
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:32333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
