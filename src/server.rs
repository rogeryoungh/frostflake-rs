use crate::{
    utils::{current_dir_file, prompt_user},
    windows::{active_console_window, active_window, enable_virtual_terminal_sequences, list_windows, notify_message},
};

use axum::{
    body::Body,
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
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    env,
    error::Error,
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(PartialEq, Clone, Copy)]
enum YasUpdateState {
    Prechecking,
    Downloading,
    Done,
    NoUpdate,
}

struct AppState {
    authorized_tokens: Mutex<HashSet<Uuid>>,
    yas_update_state: Mutex<YasUpdateState>,
    yas_download_state: Mutex<(usize, usize)>,
}

impl AppState {
    fn insert_token(&self, token: Uuid) {
        self.authorized_tokens.lock().unwrap().insert(token);
    }
    fn contains_token(&self, token: &Uuid) -> bool {
        return self.authorized_tokens.lock().unwrap().contains(token);
    }
    fn get_yas_update_state(&self) -> YasUpdateState {
        return *self.yas_update_state.lock().unwrap();
    }
    fn set_yas_update_state(&self, state: YasUpdateState) {
        *self.yas_update_state.lock().unwrap() = state;
    }
    fn get_download_state(&self) -> (usize, usize) {
        return *self.yas_download_state.lock().unwrap();
    }
    fn set_download_state(&self, state: (usize, usize)) {
        *self.yas_download_state.lock().unwrap() = state;
    }
}

#[derive(Deserialize, Serialize)]
pub struct YasReleaseInfo {
    pub version: String,
    pub update_at: String,
    pub url: String,
}

impl YasReleaseInfo {
    pub fn read_from_file() -> Result<YasReleaseInfo, Box<dyn Error>> {
        let file = fs::File::open(current_dir_file("yas_version.json"))?;
        let content = serde_json::from_reader(file)?;
        Ok(content)
    }

    pub fn write_to_file(&self) -> Result<(), Box<dyn Error>> {
        let file = fs::File::create(current_dir_file("yas_version.json"))?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    pub fn newer_than(&self, other: &YasReleaseInfo) -> bool {
        let self_time = DateTime::parse_from_rfc3339(&self.update_at).unwrap();
        let other_time = DateTime::parse_from_rfc3339(&other.update_at).unwrap();
        self_time > other_time
    }
}

impl Default for YasReleaseInfo {
    fn default() -> Self {
        YasReleaseInfo {
            version: "null".to_string(),
            update_at: "2011-08-16T00:00:00Z".to_string(),
            url: "https://example.com/".to_string(),
        }
    }
}

async fn api_root() -> Json<Value> {
    Json(json!({
        "service": "cocogoat-control-rs",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn api_token(header_map: HeaderMap, State(state): State<Arc<AppState>>) -> Response {
    let url = header_map.get("Origin").unwrap().to_str().unwrap();
    notify_message("frostflake", &format!("收到来自 {} 的新请求", url)).unwrap();
    let message = format!("来自 {} 的请求\n确定要生成新的令牌吗？[Y/N] ", url);
    active_console_window().unwrap();
    if prompt_user(&message) == "Y" {
        let id = Uuid::new_v4();
        state.insert_token(id);
        response_json(
            StatusCode::ACCEPTED,
            json!({
                "hwnd": 114514, // TODO!
                "origin": url,
                "swapEffectUpgrade": false, // TODO!
                "token": id.to_string(),
                "winver": 11 // TODO!
            }),
        )
    } else {
        response_json(StatusCode::UNAUTHORIZED, json!({}))
    }
}

async fn api_api_windows() -> Json<Value> {
    let windows = list_windows().unwrap();
    Json(json!(windows))
}

async fn api_patch_windows(uri: axum::http::Uri) -> Json<Value> {
    let hwnd = uri.path().split('/').last().unwrap();
    if hwnd != "null" {
        let hwnd: usize = hwnd.parse().unwrap();
        active_window(hwnd).unwrap()
    }
    Json(json!({}))
}

async fn api_ws(uri: axum::http::Uri, ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    if let Some(uuid_str) = uri.path().split('/').last() {
        if let Ok(uuid) = Uuid::parse_str(uuid_str) {
            if state.contains_token(&uuid) {
                return ws.on_upgrade(handle_ws);
            }
        }
    }
    response_json(StatusCode::UNAUTHORIZED, json!({}))
}

async fn yas_check_update() -> Result<YasReleaseInfo, Box<dyn Error>> {
    let github_response: Value = reqwest::Client::new()
        .get("https://api.github.com/repos/wormtql/yas/releases/latest")
        .header("User-Agent", &format!("frostflake/{}", env!("CARGO_PKG_VERSION")))
        .send()
        .await?
        .json()
        .await?;

    let version = github_response["tag_name"].as_str().unwrap().to_string();
    let update_at = github_response["published_at"].as_str().unwrap().to_string();

    let mut url = String::new();
    let yas_filename_1 = format!("yas_{}.exe", version);
    let yas_filename_2 = format!("yas_artifact_{}.exe", version);
    for asset in github_response["assets"].as_array().unwrap() {
        let name = asset["name"].as_str().unwrap();
        if name == yas_filename_1 || name == yas_filename_2 {
            url = asset["browser_download_url"].as_str().unwrap().to_owned();
            break;
        }
    }
    Ok(YasReleaseInfo {
        version,
        update_at,
        url,
    })
}

async fn yas_update(state: Arc<AppState>) {
    let current_info = YasReleaseInfo::read_from_file().unwrap_or_default();
    let latest_info = yas_check_update().await.unwrap();
    // 更新 yas
    if latest_info.newer_than(&current_info) {
        println!(
            "yas 最新版本 {}，更新时间 {}",
            latest_info.version, latest_info.update_at
        );
        println!(
            "yas 当前版本 {}，更新时间 {}",
            current_info.version, current_info.update_at
        );
        let update_message = format!("正在下载 yas，最新版本 {}", latest_info.version);
        notify_message("frostflake", &update_message).unwrap();

        state.set_yas_update_state(YasUpdateState::Downloading);

        // download(&latest_info.url, "yas_artifact.exe").await.unwrap();
        let mut download_file = fs::File::create(current_dir_file("yas_artifact.exe")).unwrap();
        let response = reqwest::get(&latest_info.url).await.unwrap();
        let total_size = response.content_length().unwrap_or(0) as usize;
        let mut current_size = 0;
        state.set_download_state((current_size, total_size));

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            current_size += chunk.len();
            download_file.write_all(&chunk).unwrap();
            state.set_download_state((current_size, total_size));
        }

        println!("更新下载完成");
        notify_message("frostflake", "更新下载完成").unwrap();
        latest_info.write_to_file().expect("Failed to write to file");
        state.set_yas_update_state(YasUpdateState::Done);
    } else {
        println!("yas 最新版本 {}，无需更新", latest_info.version);
        println!("最近更新时间 {}", latest_info.update_at);
        state.set_yas_update_state(YasUpdateState::NoUpdate);
    }
}

async fn api_post_upgrade_yas(State(state): State<Arc<AppState>>) -> Response<Body> {
    let yas_state = state.get_yas_update_state();
    if yas_state == YasUpdateState::NoUpdate || yas_state == YasUpdateState::Done {
        state.set_yas_update_state(YasUpdateState::Prechecking);

        // 后台检测新版本
        tokio::spawn(async move {
            yas_update(state).await;
        });
        response_json(StatusCode::CREATED, json!({"msg": "prechecking"}))
    } else {
        response_json(StatusCode::CONFLICT, json!({"msg": "failed"}))
    }
}

async fn api_get_upgrade_yas(State(state): State<Arc<AppState>>) -> Response<Body> {
    match state.get_yas_update_state() {
        YasUpdateState::Prechecking => response_json(StatusCode::ACCEPTED, json!({"msg": "prechecking"})),
        YasUpdateState::Downloading => {
            let (downloaded, total) = state.get_download_state();
            response_json(
                StatusCode::ACCEPTED,
                json!({"msg": "downloading", "downloaded": downloaded, "total": total}),
            )
        },
        YasUpdateState::Done => response_json(StatusCode::OK, json!({"msg": "done"})),
        YasUpdateState::NoUpdate => response_json(StatusCode::OK, json!({"msg": "noupdate"})),
    }
}

async fn make_internal_request(method: &str, url: &str, body: Value) -> Value {
    let url = &format!("http://127.0.0.1:32333{}", url);

    let method = {
        let method = method.to_uppercase().into_bytes();
        Method::from_bytes(&method).expect("Unsupport HTTP method")
    };

    let response = reqwest::Client::new()
        .request(method, url)
        .header("Accept", "application/json")
        .body(body.to_string())
        .send()
        .await;

    match response {
        Ok(response) => {
            let status = response.status().as_u16();
            let body: Value = response.json().await.unwrap_or_default();
            json!({ "status": status, "body": body})
        },
        Err(err) => {
            panic!("{}", err)
        },
    }
}

async fn api_yas() -> Json<Value> {
    let mona_json_path = current_dir_file("mona.json");
    match fs::File::open(Path::new(&mona_json_path)) {
        Ok(mona_json) => Json(serde_json::from_reader(&mona_json).unwrap()),
        Err(err) => {
            eprintln!("{}", err);
            Json(json!({}))
        },
    }
}

async fn handle_ws(mut socket: WebSocket) {
    macro_rules! ws_send_json {
        ($json:expr) => {
            socket
                .send(Message::Text(serde_json::to_string($json).unwrap()))
                .await
                .unwrap()
        };
    }
    #[derive(Debug, Deserialize)]
    struct ApiData {
        url: String,
        method: String,
        body: Option<Value>,
    }

    while let Some(Ok(Message::Text(payload))) = socket.recv().await {
        let payload: Value = serde_json::from_str(&payload).unwrap();
        if let Some("api") = payload["action"].as_str() {
            let data: ApiData = serde_json::from_value(payload["data"].clone()).unwrap();
            if data.url != "/api/yas" {
                let response: Value = json!({
                   "action": "api",
                   "data": make_internal_request(&data.method, &data.url, data.body.unwrap_or_default()).await,
                   "id": payload["id"],
                });
                ws_send_json!(&response);
            } else {
                let argv = {
                    let body = data.body.unwrap();
                    let body: Value = serde_json::from_str(body.as_str().unwrap()).unwrap();
                    body["argv"].as_str().unwrap().to_owned()
                };
                let command = current_dir_file("yas_artifact.exe");
                let mut child = std::process::Command::new(&command)
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
                ws_send_json!(&json!({
                    "action": "yas-output",
                    "data": format!("{} {}", command.display(), argv),
                }));
                ws_send_json!(&json!({"action": "yas","data": "load"}));
                for line in reader.lines() {
                    let line = line.unwrap();
                    println!("{}", line);
                    ws_send_json!(&json!({"action": "yas-output", "data": line}));
                }
                ws_send_json!(&json!({"action": "yas","data": "exit"}));
            }
        }
    }
}

fn response_json(code: StatusCode, body: Value) -> Response<Body> {
    Response::builder()
        .status(code)
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("Failed to build response")
}

pub async fn start_server(bind_addr: &str) {
    println!("Server running on http://{}", bind_addr);
    enable_virtual_terminal_sequences().unwrap();

    let shared_state = Arc::new(AppState {
        authorized_tokens: Mutex::new(HashSet::new()),
        yas_update_state: Mutex::new(YasUpdateState::NoUpdate),
        yas_download_state: Mutex::new((0, 114514)),
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

    #[cfg(feature = "tracing")]
    {
        tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();
        let app = app.layer(tower_http::trace::TraceLayer::new_for_http());
    }

    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
