use crate::{
    utils::prompt_user,
    windows::{active_window, list_windows},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{HeaderMap, Method, StatusCode},
    response::Response,
    routing::{get, options, patch, post},
    Json, Router,
};

use serde_json::{json, Value};
use std::{
    collections::HashSet,
    io::{BufRead, BufReader},
    sync::{Arc, Mutex},
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use uuid::Uuid;

struct AppState {
    authorized_tokens: Mutex<HashSet<Uuid>>,
    yas_update: Mutex<i32>,
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

async fn api_api_windows() -> Json<Value> {
    let windows = list_windows().unwrap();
    return Json(json!(windows));
}

async fn api_patch_windows(uri: axum::http::Uri) -> Json<Value> {
    let hwnd = uri.path().split('/').last().unwrap();
    if hwnd != "null" {
        let hwnd = hwnd.parse::<usize>().unwrap();
        active_window(hwnd).unwrap()
    }
    return Json(json!({}));
}

async fn api_ws(uri: axum::http::Uri, ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    println!("path: {}", &uri);
    let uuid_str = uri.path().split('/').last().unwrap();
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

async fn api_upgrade_yas(State(state): State<Arc<AppState>>) -> Response<axum::body::Body> {
    let mut count = state.yas_update.lock().unwrap();
    *count += 1;
    if count.eq(&1) {
        return Response::builder()
            .status(StatusCode::CREATED)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(json!({"msg": "done"}).to_string()))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(json!({"msg": "done"}).to_string()))
            .unwrap();
    }
}

async fn make_internal_request(method: String, url: String, body: String) -> Value {
    let url = format!("http://127.0.0.1:32333{}", url);
    let method = method.to_uppercase();

    println!("method: {}, url: {}", method, url);
    let client = reqwest::Client::new();

    let request = match method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url).body(body),
        "PUT" => client.put(&url).body(body),
        "PATCH" => client.patch(&url).body(body),
        "DELETE" => client.delete(&url),
        _ => panic!("Unsupported HTTP method"),
    };
    let request = request.header("Accept", "application/json");
    // debug request
    println!("request: {:?}", request);

    match request.header("Accept", "application/json").send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap();
            println!("status: {}, body: {}", status, body);
            let json = serde_json::from_str::<Value>(&body).unwrap_or_default();

            return json!({
                "status": status,
                "body": json,
            });
        },
        Err(err) => {
            panic!("{}", err)
        },
    }
}

async fn handle_ws(mut socket: WebSocket) {
    macro_rules! send_json {
        ($json:expr) => {
            socket
                .send(Message::Text(serde_json::to_string($json).unwrap()))
                .await
                .unwrap()
        };
    }
    while let Some(Ok(Message::Text(payload))) = socket.recv().await {
        let payload = serde_json::from_str::<Value>(&payload).unwrap();
        if let Some("api") = payload["action"].as_str() {
            let url = payload["data"]["url"].as_str().unwrap().to_string();
            let method = payload["data"]["method"].as_str().unwrap().to_string();
            let body: String = payload["data"]["body"].as_str().unwrap_or_default().to_string();
            if url != "/api/yas" {
                let response: Value = json!({
                   "action": "api",
                   "data": make_internal_request(method, url, body).await,
                   "id": payload["id"],
                });
                send_json!(&response);
            } else {
                let argv = serde_json::from_str::<Value>(&body).unwrap();
                let argv = argv["argv"].as_str().unwrap();
                let command = String::from("C:\\Users\\YOUNG\\Downloads\\yas_artifact_v0.1.18.exe");
                let child = std::process::Command::new(&command)
                    .args(argv.split_whitespace())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();
                let mut reader = BufReader::new(child.stdout.unwrap());
                send_json!(&json!({
                    "action": "yas-output",
                    "data": format!("{} {}", &command, argv),
                }));
                send_json!(&json!({
                    "action": "yas",
                    "data": "load",
                }));
                let mut buf = String::new();
                while let Ok(_) = reader.read_line(&mut buf) {
                    send_json!(&json!({
                        "action": "yas-output",
                        "data": buf,
                    }));
                }
                send_json!(&json!({
                    "action": "yas",
                    "data": "exit",
                }));
            }
        }
    }
}

pub async fn start_server() {
    println!("Server running on http://localhost:32333");

    let shared_state = Arc::new(AppState {
        authorized_tokens: Mutex::new(HashSet::new()),
        yas_update: Mutex::new(0),
    });

    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    let cors = CorsLayer::new()
        .allow_origin(Any) // 允许所有来源
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS]) // 允许的 HTTP 方法
        .allow_headers(Any); // 允许携带身份验证信息（如 Cookie、Authorization）

    let app = Router::new()
        .route("/", get(api_root))
        .route("/token", post(api_token))
        .route("/", options(|| async { "" }))
        .route("/api/windows", get(api_api_windows))
        .route("/api/windows/:hwnd", patch(api_patch_windows))
        .route("/ws/:uuid", get(api_ws))
        .route("/api/upgrade/yas", post(api_upgrade_yas).get(api_upgrade_yas))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:32333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
