use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::thread;

use anyhow::{Context, anyhow};
use axum::http::{StatusCode, header};
use axum::extract::State;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::sync::oneshot;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::WindowBuilder;
use tracing::{info, warn};
use wry::WebViewBuilder;

const APP_TITLE: &str = "CSCS Key";
const BUNDLED_CLI_NAME: &str = "cscs-key";

#[derive(Parser, Debug)]
#[command(author, version, about = "Local web UI for cscs-key")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 0)]
    port: u16,
    #[arg(long)]
    bin: Option<PathBuf>,
    #[arg(long, default_value_t = false, help = "Open the UI in the system browser instead of the desktop shell")]
    browser: bool,
    #[arg(long, default_value_t = false, help = "Start the local server without opening a window")]
    headless: bool,
    #[arg(long, hide = true, default_value_t = false)]
    no_open_browser: bool,
    #[arg(long, default_value_t = 1320.0, help = "Initial desktop window width")]
    window_width: f64,
    #[arg(long, default_value_t = 900.0, help = "Initial desktop window height")]
    window_height: f64,
}

#[derive(Clone)]
struct AppState {
    binary_path: PathBuf,
    repo_root: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetaResponse {
    binary_path: String,
    repo_root: String,
    default_key_path: String,
    platform: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRequest {
    command: UiCommand,
    env: Option<UiEnvironment>,
    file: Option<String>,
    duration: Option<UiDuration>,
    all: Option<bool>,
    dry: Option<bool>,
    key_ids: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunResponse {
    ok: bool,
    exit_code: Option<i32>,
    command_line: String,
    stdout: String,
    stderr: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum UiCommand {
    Gen,
    Sign,
    List,
    Revoke,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum UiEnvironment {
    Prod,
    Tds,
}

impl UiEnvironment {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Prod => "prod",
            Self::Tds => "tds",
        }
    }
}

#[derive(Deserialize, Clone, Copy)]
enum UiDuration {
    #[serde(rename = "1d")]
    Day,
    #[serde(rename = "1min")]
    Minute,
}

impl UiDuration {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Day => "1d",
            Self::Minute => "1min",
        }
    }
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                message: self.message,
            }),
        )
            .into_response()
    }
}

const INDEX_HTML: &str = include_str!("../static/index.html");
const STYLES_CSS: &str = include_str!("../static/styles.css");
const APP_JS: &str = include_str!("../static/app.js");

fn main() -> anyhow::Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    let mut args = Args::parse();
    if args.no_open_browser {
        args.headless = true;
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("UI manifest is missing a parent directory")?
        .to_path_buf();
    let binary_path = resolve_binary_path(args.bin.as_deref(), &repo_root)?;

    let state = Arc::new(AppState {
        binary_path,
        repo_root,
    });
    let app = build_app(state);

    if args.browser || args.headless {
        return run_foreground_mode(app, &args);
    }

    run_desktop_mode(app, &args)
}

fn build_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/assets/styles.css", get(styles))
        .route("/assets/app.js", get(script))
        .route("/api/meta", get(meta))
        .route("/api/run", post(run_command))
        .with_state(state)
}

fn run_foreground_mode(app: Router, args: &Args) -> anyhow::Result<()> {
    let runtime = build_runtime()?;

    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind((args.host.as_str(), args.port))
            .await
            .with_context(|| format!("failed to bind {}:{}", args.host, args.port))?;
        let addr: SocketAddr = listener.local_addr()?;
        let url = format!("http://{}", addr);
        announce_url(&url);

        if args.browser {
            if let Err(error) = webbrowser::open(&url) {
                warn!("failed to open browser: {error}");
            }
        }

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    })
}

fn run_desktop_mode(app: Router, args: &Args) -> anyhow::Result<()> {
    let listener = TcpListener::bind((args.host.as_str(), args.port))
        .with_context(|| format!("failed to bind {}:{}", args.host, args.port))?;
    listener
        .set_nonblocking(true)
        .context("failed to set desktop listener to non-blocking mode")?;
    let addr: SocketAddr = listener.local_addr()?;
    let url = format!("http://{}", addr);
    announce_url(&url);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server_handle = thread::spawn(move || -> anyhow::Result<()> {
        let runtime = build_runtime()?;
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)
                .context("failed to transfer desktop listener to async runtime")?;

            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await?;

            Ok(())
        })
    });

    let window_result = run_webview_window(args, &url);
    let _ = shutdown_tx.send(());

    let server_result = match server_handle.join() {
        Ok(result) => result,
        Err(_) => Err(anyhow!("desktop UI server thread panicked")),
    };

    window_result?;
    server_result?;

    Ok(())
}

fn run_webview_window(args: &Args, url: &str) -> anyhow::Result<()> {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(APP_TITLE)
        .with_inner_size(LogicalSize::new(args.window_width, args.window_height))
        .with_min_inner_size(LogicalSize::new(960.0, 720.0))
        .build(&event_loop)
        .context("failed to create desktop window")?;

    let _webview = WebViewBuilder::new()
        .with_url(url)
        .build(&window)
        .context("failed to build desktop webview")?;

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });

    Ok(())
}

fn announce_url(url: &str) {
    info!("{APP_TITLE} listening on {url}");
    println!("Open {url}");
}

fn build_runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    RuntimeBuilder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build async runtime")
}

async fn index() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn styles() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], STYLES_CSS)
}

async fn script() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        APP_JS,
    )
}

async fn meta(State(state): State<Arc<AppState>>) -> Json<MetaResponse> {
    Json(MetaResponse {
        binary_path: state.binary_path.display().to_string(),
        repo_root: state.repo_root.display().to_string(),
        default_key_path: default_key_path().display().to_string(),
        platform: std::env::consts::OS.to_string(),
    })
}

async fn run_command(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    let request = ValidatedRequest::from_payload(payload)?;
    let mut command = Command::new(&state.binary_path);
    command.args(&request.args);
    command.stdin(Stdio::null());

    let output = command
        .output()
        .await
        .map_err(|error| ApiError::internal(format!("failed to run cscs-key: {error}")))?;

    Ok(Json(RunResponse {
        ok: output.status.success(),
        exit_code: output.status.code(),
        command_line: format_command_line(&state.binary_path, &request.args),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }))
}

struct ValidatedRequest {
    args: Vec<String>,
}

impl ValidatedRequest {
    fn from_payload(payload: RunRequest) -> Result<Self, ApiError> {
        let mut args = Vec::new();

        if let Some(env) = payload.env {
            args.push("--env".to_string());
            args.push(env.as_arg().to_string());
        }

        match payload.command {
            UiCommand::Gen => {
                args.push("gen".to_string());
                push_file_arg(&mut args, payload.file);
                push_duration_arg(&mut args, payload.duration);
            }
            UiCommand::Sign => {
                args.push("sign".to_string());
                push_file_arg(&mut args, payload.file);
                push_duration_arg(&mut args, payload.duration);
            }
            UiCommand::List => {
                args.push("list".to_string());
                if payload.all.unwrap_or(false) {
                    args.push("--all".to_string());
                }
            }
            UiCommand::Revoke => {
                args.push("revoke".to_string());

                let revoke_all = payload.all.unwrap_or(false);
                let dry_run = payload.dry.unwrap_or(false);
                let key_ids = payload
                    .key_ids
                    .unwrap_or_default()
                    .into_iter()
                    .map(|key_id| key_id.trim().to_string())
                    .filter(|key_id| !key_id.is_empty())
                    .collect::<Vec<_>>();

                if revoke_all {
                    args.push("--all".to_string());
                } else if key_ids.is_empty() {
                    return Err(ApiError::bad_request(
                        "Enter at least one certificate serial number or enable revoke all.",
                    ));
                } else {
                    args.extend(key_ids);
                }

                if dry_run {
                    args.push("--dry".to_string());
                }
            }
        }

        Ok(Self { args })
    }
}

fn push_file_arg(args: &mut Vec<String>, file: Option<String>) {
    if let Some(path) = normalized_file_path(file) {
        args.push("--file".to_string());
        args.push(path);
    }
}

fn push_duration_arg(args: &mut Vec<String>, duration: Option<UiDuration>) {
    if let Some(duration) = duration {
        args.push("--duration".to_string());
        args.push(duration.as_arg().to_string());
    }
}

fn normalized_file_path(file: Option<String>) -> Option<String> {
    let file = file?;
    let trimmed = file.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(expand_home(trimmed).display().to_string())
}

fn expand_home(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(suffix) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(suffix);
        }
    }

    PathBuf::from(path)
}

fn format_command_line(binary_path: &Path, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_quote(&binary_path.display().to_string()));
    parts.extend(args.iter().map(|arg| shell_quote(arg)));
    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    let simple = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':' | '='));

    if simple {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn resolve_binary_path(explicit: Option<&Path>, repo_root: &Path) -> anyhow::Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }

    if let Ok(path) = std::env::var("CSCS_KEY_BIN") {
        return Ok(PathBuf::from(path));
    }

    if let Some(path) = bundled_binary_path() {
        return Ok(path);
    }

    let candidates = [
        repo_root.join("target/release/cscs-key"),
        repo_root.join("target/debug/cscs-key"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    which::which("cscs-key").map_err(|_| {
        anyhow!(
            "could not locate cscs-key. Build the project first, use the packaged app bundle, or pass --bin /path/to/cscs-key"
        )
    })
}

fn bundled_binary_path() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let contents_dir = executable.parent()?.parent()?;
    let candidate = contents_dir.join("Resources/bin").join(BUNDLED_CLI_NAME);
    candidate.exists().then_some(candidate)
}

fn default_key_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".ssh/cscs-key")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        if let Ok(mut stream) = signal(SignalKind::terminate()) {
            let _ = stream.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}