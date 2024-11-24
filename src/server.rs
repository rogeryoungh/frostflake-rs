use crate::{
    utils::{download, prompt_user},
    windows::{active_window, enable_virtual_terminal_sequences, list_windows, notify_message},
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

use chrono::DateTime;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    env, fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(PartialEq)]
enum YasUpdateState {
    Prechecking,
    Downloading,
    Done,
    NoUpdate,
}

struct AppState {
    authorized_tokens: Mutex<HashSet<Uuid>>,
    yas_update_state: Mutex<YasUpdateState>,
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
    notify_message("frostflake", &format!("收到来自 {} 的新请求", url)).unwrap();
    let message = format!("来自 {} 的请求\n确定要生成新的令牌吗？[Y/N] ", url);
    if prompt_user(&message) == "Y" {
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
        return Json(json!({"message": "Operation cancelled by the user"}));
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

async fn yas_check_update() -> (String, String, String) {
    let github_response: Value = reqwest::Client::new()
        .get("https://api.github.com/repos/wormtql/yas/releases/latest")
        .header("User-Agent", &format!("frostflake/{}", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let latest_version = github_response["tag_name"].as_str().unwrap().to_string();
    let latest_update = github_response["published_at"].as_str().unwrap().to_string();
    let mut latest_url = String::new();
    for asset in github_response["assets"].as_array().unwrap() {
        let name = asset["name"].as_str().unwrap();
        if name == format!("yas_{}.exe", latest_version) {
            latest_url = asset["browser_download_url"].as_str().unwrap().to_owned();
            break;
        }
        if name == format!("yas_artifact_{}.exe", latest_version) {
            latest_url = asset["browser_download_url"].as_str().unwrap().to_owned();
            break;
        }
    }
    return (latest_version, latest_update, latest_url);
}

async fn yas_update(state: Arc<AppState>) {
    let config_path = env::current_dir().unwrap().join("yas_version.json");
    if !&config_path.exists() {
        let init = json!({
            "version": "null",
            "update_at": "2011-08-16T00:00:00Z",
            "url": "https://example.com/"
        });
        let file = fs::File::create(&config_path).unwrap();
        serde_json::to_writer_pretty(file, &init).unwrap();
    }
    let config_content = fs::File::open(Path::new(&config_path)).unwrap();
    let config_content: Value = serde_json::from_reader(config_content).unwrap();
    let config_update = config_content["update_at"].as_str().unwrap();

    let (latest_version, latest_update, latest_url) = yas_check_update().await;

    let config_update_time = DateTime::parse_from_rfc3339(config_update).unwrap();
    let latest_update_time = DateTime::parse_from_rfc3339(&latest_update).unwrap();

    if latest_update_time > config_update_time {
        let update_message = format!(
            "yas 最新版本 {}，当前版本 {}，正在下载更新",
            config_content["version"], latest_version
        );
        println!("{}", update_message);
        notify_message("frostflake", &update_message).unwrap();
        {
            let mut yas_state = state.yas_update_state.lock().unwrap();
            *yas_state = YasUpdateState::Downloading;
        }
        tokio::spawn(async move {
            let config_path = env::current_dir().unwrap().join("yas_version.json");
            let config_path = Path::new(&config_path);
            download(&latest_url, "yas_artifact.exe").await;
            notify_message("frostflake", "更新下载完成").unwrap();
            let version_file = fs::File::create(config_path).unwrap();
            let new_version = json!({
                "version": latest_version,
                "update_at": latest_update,
                "url": latest_url
            });
            serde_json::to_writer_pretty(version_file, &new_version).unwrap();
            let mut yas_state = state.yas_update_state.lock().unwrap();
            *yas_state = YasUpdateState::Done;
        });
    } else {
        println!("yas 最新版本 {}，无需更新", latest_version);
        let mut yas_state = state.yas_update_state.lock().unwrap();
        *yas_state = YasUpdateState::NoUpdate;
    }
}

async fn api_post_upgrade_yas(State(state): State<Arc<AppState>>) -> Response<axum::body::Body> {
    let mut yas_state = state.yas_update_state.lock().unwrap();
    if *yas_state == YasUpdateState::NoUpdate || *yas_state == YasUpdateState::Done {
        *yas_state = YasUpdateState::Prechecking;
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            yas_update(state_clone).await;
        });
        return Response::builder()
            .status(StatusCode::CREATED)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(json!({"msg": "prechecking"}).to_string()))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::CONFLICT)
            .body(axum::body::Body::from(json!({"msg": "failed"}).to_string()))
            .unwrap();
    }
}

async fn api_get_upgrade_yas(State(state): State<Arc<AppState>>) -> Response<axum::body::Body> {
    let state = state.yas_update_state.lock().unwrap();
    let response = |code, msg| -> Response<axum::body::Body> {
        return Response::builder()
            .status(code)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(json!({"msg": msg}).to_string()))
            .unwrap();
    };
    match *state {
        YasUpdateState::Prechecking => {
            return response(StatusCode::ACCEPTED, "prechecking");
        },
        YasUpdateState::Downloading => {
            return response(StatusCode::ACCEPTED, "downloading");
        },
        YasUpdateState::Done => {
            return response(StatusCode::OK, "done");
        },
        YasUpdateState::NoUpdate => {
            return response(StatusCode::OK, "noupdate");
        },
    }
}

async fn make_internal_request(method: &str, url: &str, body: String) -> Value {
    let url = &format!("http://127.0.0.1:32333{}", url);
    let method = method.to_uppercase();

    let client = reqwest::Client::new();

    let request = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url).body(body),
        "PUT" => client.put(url).body(body),
        "PATCH" => client.patch(url).body(body),
        "DELETE" => client.delete(url),
        _ => panic!("Unsupported HTTP method"),
    };
    let request = request.header("Accept", "application/json");

    match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let body: Value = response.json().await.unwrap_or_default();
            return json!({
                "status": status,
                "body": body,
            });
        },
        Err(err) => {
            panic!("{}", err)
        },
    }
}

async fn api_yas() -> Json<Value> {
    let mona_json_path = env::current_dir().unwrap().join("mona.json");
    let mona_json = Path::new(&mona_json_path);
    if mona_json.exists() {
        let content = fs::read_to_string(mona_json).unwrap();
        return Json(serde_json::from_str::<Value>(&content).unwrap());
    } else {
        return Json(json!({}));
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
            let url = payload["data"]["url"].as_str().unwrap();
            let method = payload["data"]["method"].as_str().unwrap();
            let body = payload["data"]["body"].as_str().unwrap_or_default();
            if url != "/api/yas" {
                let response: Value = json!({
                   "action": "api",
                   "data": make_internal_request(method, url, String::from(body)).await,
                   "id": payload["id"],
                });
                send_json!(&response);
            } else {
                let argv = serde_json::from_str::<Value>(&body).unwrap();
                let argv = argv["argv"].as_str().unwrap();
                let command = Path::new("yas_artifact.exe");
                let mut child = std::process::Command::new(command)
                    .args(argv.split_whitespace())
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();
                if let Some(mut stdin) = child.stdin.take() {
                    if let Err(e) = stdin.write(b"114514") {
                        eprintln!("Failed to write to stdin: {}", e);
                    }
                }
                let reader = BufReader::new(child.stderr.take().unwrap());
                send_json!(&json!({
                    "action": "yas-output",
                    "data": format!("{} {}", command.display(), argv),
                }));
                send_json!(&json!({"action": "yas","data": "load"}));
                for line in reader.lines() {
                    let line = line.unwrap();
                    println!("{}", line);
                    send_json!(&json!({"action": "yas-output", "data": line}));
                }
                send_json!(&json!({"action": "yas","data": "exit"}));
            }
        }
    }
}

pub async fn start_server() {
    println!("Server running on http://localhost:32333");
    enable_virtual_terminal_sequences().unwrap();

    let shared_state = Arc::new(AppState {
        authorized_tokens: Mutex::new(HashSet::new()),
        yas_update_state: Mutex::new(YasUpdateState::NoUpdate),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(api_root))
        .route("/token", post(api_token))
        .route("/", options(|| async { "" }))
        .route("/api/windows", get(api_api_windows))
        .route("/api/windows/:hwnd", patch(api_patch_windows))
        .route("/ws/:uuid", get(api_ws))
        .route("/api/upgrade/yas", post(api_post_upgrade_yas).get(api_get_upgrade_yas))
        .route("/api/yas", get(api_yas))
        .layer(cors)
        .with_state(shared_state);

    #[cfg(feature = "tracing-log")]
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();
    #[cfg(feature = "tracing-log")]
    let app = app.layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:32333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
