// SPDX-License-Identifier: MIT
// omamobile-ai: on-device AI supervisor
//
// Single OpenAI-compatible entry point on 127.0.0.1:8080. llama-server is
// spawned lazily on the first chat request, proxied to, and killed after an
// idle timeout, so a 6 GB phone does not keep ~1.5 GB of model resident.
//
// Public surface is deliberately tiny: `main` starts the daemon and the three
// axum handlers delegate to private helpers named with a leading underscore.

use std::{
    collections::HashSet,
    fs,
    path::Path,
    process::Stdio,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tokio::{net::TcpListener, sync::Mutex};
use tracing::{error, info, warn};

// How often the idle watchdog looks at the clock.
const WATCHDOG_TICK: Duration = Duration::from_secs(5);
// How long llama-server gets to open its port before we give up.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);

// ─── Configuration ───────────────────────────────────────────────────

/// Everything the supervisor needs, from `/etc/omamobile-ai.conf` and drop-ins.
struct Config {
    model_path: String,
    model_id: String,
    threads: usize,
    cpu_affinity: Option<Vec<usize>>,
    context_size: u32,
    gpu_layers: u32,
    idle_timeout_secs: i64,
    listen_host: String,
    listen_port: u16,
    llama_port: u16,
    llama_bin: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_path: "/usr/share/omamobile-ai/models/LFM3-1.7B-Q4_K_M.gguf".into(),
            model_id: "LFM3-1.7B".into(),
            threads: 3,
            cpu_affinity: Some(vec![4, 5, 6]),
            context_size: 32768,
            gpu_layers: 0,
            idle_timeout_secs: 180,
            listen_host: "127.0.0.1".into(),
            listen_port: 8080,
            llama_port: 18080,
            llama_bin: "llama-server".into(),
        }
    }
}

/// Load the base config, then every drop-in in `/etc/omamobile-ai.conf.d`.
fn _load_config() -> Config {
    let mut config = Config::default();
    let base = std::env::var("OMAMOBILE_AI_CONFIG")
        .unwrap_or_else(|_| "/etc/omamobile-ai.conf".into());

    _apply_config_file(Path::new(&base), &mut config);

    let dropins = Path::new("/etc/omamobile-ai.conf.d");
    if let Ok(entries) = fs::read_dir(dropins) {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "conf"))
            .collect();
        paths.sort();
        for path in paths {
            _apply_config_file(&path, &mut config);
        }
    }

    config
}

/// Apply `KEY=VALUE` lines from one file; unknown keys and bad values only warn.
fn _apply_config_file(path: &Path, config: &mut Config) {
    let Ok(text) = fs::read_to_string(path) else {
        return; // A missing base file is fine: defaults stand.
    };

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            warn!("{}: ignoring {:?}", path.display(), line);
            continue;
        };
        _apply_setting(key.trim(), value.trim(), config, path);
    }
}

/// Apply a single setting, reporting anything unusable.
fn _apply_setting(key: &str, value: &str, config: &mut Config, path: &Path) {
    match key {
        "MODEL_PATH" => config.model_path = value.into(),
        "MODEL_ID" => config.model_id = value.into(),
        "LLAMA_BIN" => config.llama_bin = value.into(),
        "LISTEN_HOST" => config.listen_host = value.into(),
        "CPU_AFFINITY" => config.cpu_affinity = _parse_affinity(value),
        "THREADS" => _set_number(value, path, key, |n| config.threads = n),
        "CTX_SIZE" => _set_number(value, path, key, |n| config.context_size = n),
        "GPU_LAYERS" => _set_number(value, path, key, |n| config.gpu_layers = n),
        "IDLE_TIMEOUT" => _set_number(value, path, key, |n| config.idle_timeout_secs = n),
        "LISTEN_PORT" => _set_number(value, path, key, |n| config.listen_port = n),
        "LLAMA_PORT" => _set_number(value, path, key, |n| config.llama_port = n),
        other => warn!("{}: unknown setting {:?}", path.display(), other),
    }
}

/// Parse `value` as N and hand it to `store`; a bad value only warns.
fn _set_number<N: std::str::FromStr>(
    value: &str,
    path: &Path,
    key: &str,
    store: impl FnOnce(N),
) {
    match value.parse() {
        Ok(n) => store(n),
        Err(_) => warn!("{}: bad value for {}: {:?}", path.display(), key, value),
    }
}

/// Parse "4-6", "4,5,6" or "4-5,7" into a core list; empty means "any CPU".
fn _parse_affinity(value: &str) -> Option<Vec<usize>> {
    let mut cores = HashSet::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((a, b)) => {
                let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) else {
                    return None;
                };
                if a > b {
                    return None;
                }
                cores.extend(a..=b);
            }
            None => match part.parse::<usize>() {
                Ok(n) => {
                    cores.insert(n);
                }
                Err(_) => return None,
            },
        }
    }
    if cores.is_empty() {
        None
    } else {
        let mut cores: Vec<_> = cores.into_iter().collect();
        cores.sort_unstable();
        Some(cores)
    }
}

/// Refuse to start when the model is missing; the port conflicts surface at bind.
fn _validate_config(config: &Config) -> Result<(), String> {
    if !Path::new(&config.model_path).is_file() {
        return Err(format!("model not found: {}", config.model_path));
    }
    Ok(())
}

// ─── Shared state ────────────────────────────────────────────────────

/// Runtime state: the config, the child we own, and the last request time.
struct AppState {
    config: Config,
    llama: Mutex<Option<tokio::process::Child>>,
    last_request: AtomicI64,
}

impl AppState {
    fn new(config: Config) -> Self {
        Self {
            config,
            llama: Mutex::new(None),
            last_request: AtomicI64::new(0),
        }
    }
}

// ─── Entry point ─────────────────────────────────────────────────────

/// Start the daemon: load config, check it, serve, and clean up on exit.
pub fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omamobile_ai=info".into()),
        )
        .init();

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(e) = runtime.block_on(_serve()) {
        error!("fatal: {e}");
        std::process::exit(1);
    }
}

/// Bind, run the server until a signal, then stop llama-server.
async fn _serve() -> Result<(), Box<dyn std::error::Error>> {
    let config = _load_config();
    _validate_config(&config)?;

    let host = config.listen_host.clone();
    let port = config.listen_port;
    let state = Arc::new(AppState::new(config));
    let router = _build_router(state.clone());
    let listener = TcpListener::bind((host.as_str(), port)).await?;

    info!("listening on http://{host}:{port}");

    tokio::spawn(_idle_watchdog(state.clone()));

    axum::serve(listener, router)
        .with_graceful_shutdown(_shutdown_signal())
        .await?;

    _stop_llama(&state).await;
    Ok(())
}

/// Resolve on SIGTERM or Ctrl-C so the child is always reaped.
async fn _shutdown_signal() {
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
    match sigterm.as_mut() {
        Some(sig) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sig.recv() => {}
            }
        }
        None => {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
    info!("shutting down");
}

/// All routes the supervisor answers.
fn _build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(_handle_health))
        .route("/v1/models", get(_handle_models))
        .route("/v1/chat/completions", post(_handle_chat))
        .route("/v1/unload", post(_handle_unload))
        .fallback(_handle_not_found)
        .with_state(state)
}

// ─── Handlers ────────────────────────────────────────────────────────

/// GET /health: liveness plus RAM, battery, load and whether the model is up.
async fn _handle_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let running = state.llama.lock().await.is_some();
    Json(json!({
        "status": "ok",
        "model": _model_id(&state.config),
        "model_loaded": running,
        "idle_timeout_secs": state.config.idle_timeout_secs,
        "threads": state.config.threads,
        "stats": _system_stats(),
    }))
}

/// GET /v1/models: the one model this device serves.
async fn _handle_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [{
            "id": _model_id(&state.config),
            "object": "model",
            "owned_by": "omamobile",
        }],
    }))
}

/// POST /v1/unload: stop llama-server now and free the model's RAM.
async fn _handle_unload(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    _stop_llama(&state).await;
    Json(json!({ "status": "unloaded" }))
}

/// POST /v1/chat/completions: ensure the model is up, then proxy both modes.
async fn _handle_chat(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Response, StatusCode> {
    _ensure_llama(&state)
        .await
        .map_err(|e| {
            error!("llama-server unavailable: {e}");
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    _record_request(&state);
    _proxy_chat(&state, body).await
}

/// Anything else: a JSON 404 so clients see a readable error.
async fn _handle_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" })))
}

// ─── llama-server lifecycle ──────────────────────────────────────────

/// Start llama-server unless it is already running, and wait for its port.
async fn _ensure_llama(state: &AppState) -> Result<(), String> {
    let mut guard = state.llama.lock().await;
    if guard.is_some() {
        return Ok(());
    }

    let child = _spawn_llama(&state.config)?;
    *guard = Some(child);

    match _wait_until_ready(state.config.llama_port).await {
        Ok(()) => {
            info!("llama-server ready on port {}", state.config.llama_port);
            Ok(())
        }
        Err(e) => {
            drop(guard);
            _stop_llama(state).await;
            Err(e)
        }
    }
}

/// Build and spawn the llama-server process with the LFM3 agent flags.
fn _spawn_llama(config: &Config) -> Result<tokio::process::Child, String> {
    info!(
        "starting llama-server: {} (threads {}, cores {:?})",
        config.model_path, config.threads, config.cpu_affinity
    );

    let mut cmd = tokio::process::Command::new(&config.llama_bin);
    cmd.args(_llama_args(config))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    if let Some(cores) = config.cpu_affinity.clone() {
        let cores = Arc::new(cores);
        // SAFETY: sched_setaffinity only touches this child's own CPU mask.
        unsafe {
            cmd.pre_exec(move || _set_affinity(&cores));
        }
    }

    cmd.spawn()
        .map_err(|e| format!("cannot run {}: {e}", config.llama_bin))
}

/// The argument list shared by every start, matching Liquid's LFM3 recipe.
fn _llama_args(config: &Config) -> Vec<String> {
    let mut args = vec![
        "--model".into(),
        config.model_path.clone(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        config.llama_port.to_string(),
        "--threads".into(),
        config.threads.to_string(),
        "-c".into(),
        config.context_size.to_string(),
        // Tool calling through the model's own template.
        "--jinja".into(),
        "--temp".into(),
        "0.1".into(),
        "--top-k".into(),
        "50".into(),
        "--repeat-penalty".into(),
        "1.1".into(),
    ];

    if config.gpu_layers > 0 {
        args.push("-ngl".into());
        args.push(config.gpu_layers.to_string());
    }

    args
}

/// Pin the calling thread to `cores`; used from the child's pre_exec hook.
fn _set_affinity(cores: &[usize]) -> std::io::Result<()> {
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::CPU_ZERO(&mut set);
        for &core in cores {
            libc::CPU_SET(core, &mut set);
        }
        if libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Poll llama-server's /health until it reports ready (200), not merely until
/// its port answers: it opens the port while the GGUF is still loading and
/// answers 503 to requests until the model is in memory.
async fn _wait_until_ready(port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/health");
    let client = _http_client();
    let deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;

    while tokio::time::Instant::now() < deadline {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Err(format!("llama-server did not become ready on port {port}"))
}

/// Kill and reap llama-server, if we own one.
async fn _stop_llama(state: &AppState) {
    let mut guard = state.llama.lock().await;
    if let Some(mut child) = guard.take() {
        let _ = child.start_kill();
        let _ = child.wait().await;
        info!("llama-server stopped, model unloaded");
    }
    state.last_request.store(0, Ordering::Relaxed);
}

/// Kill llama-server once the idle timeout has passed with no chat request.
async fn _idle_watchdog(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(WATCHDOG_TICK).await;
        if _idle_expired(&state) {
            info!("idle timeout reached");
            _stop_llama(&state).await;
        }
    }
}

/// True when a model is loaded and nobody has asked for it in a while.
fn _idle_expired(state: &AppState) -> bool {
    let last = state.last_request.load(Ordering::Relaxed);
    if last == 0 {
        return false; // Nothing was started through us yet.
    }
    let now = _unix_now();
    now.saturating_sub(last) >= state.config.idle_timeout_secs
}

/// Remember when a request last arrived.
fn _record_request(state: &AppState) {
    state.last_request.store(_unix_now(), Ordering::Relaxed);
}

/// Seconds since the epoch.
fn _unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ─── Proxy ───────────────────────────────────────────────────────────

/// Forward the body to llama-server and stream its answer back untouched,
/// so both `stream: true` (SSE) and plain JSON work without parsing here.
async fn _proxy_chat(state: &AppState, body: String) -> Result<Response, StatusCode> {
    let url = format!(
        "http://127.0.0.1:{}/v1/chat/completions",
        state.config.llama_port
    );

    let upstream = _http_client()
        .post(url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            error!("proxy failed: {e}");
            StatusCode::BAD_GATEWAY
        })?;

    let status = upstream.status();
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let body = Body::from_stream(upstream.bytes_stream());
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// A client that never times out mid-generation and does not buffer streams.
fn _http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .pool_max_idle_per_host(2)
        .build()
        .expect("http client")
}

// ─── Identity and stats ──────────────────────────────────────────────

/// The model id clients ask for, e.g. LFM3-1.7B.
fn _model_id(config: &Config) -> String {
    config.model_id.clone()
}

/// RAM, swap, battery and load, as one JSON object.
fn _system_stats() -> serde_json::Value {
    json!({
        "mem_total_kb": _meminfo("MemTotal"),
        "mem_available_kb": _meminfo("MemAvailable"),
        "swap_total_kb": _meminfo("SwapTotal"),
        "swap_free_kb": _meminfo("SwapFree"),
        "battery": _battery(),
        "loadavg": _loadavg(),
    })
}

/// One numeric field from /proc/meminfo, in kB.
fn _meminfo(key: &str) -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    text.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?;
        rest.trim_start_matches(':')
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

/// Battery percentage and status, chosen from the first supply that answers.
fn _battery() -> String {
    const SUPPLIES: [&str; 3] = ["max170xx_battery", "battery", "max77705-charger"];
    for name in SUPPLIES {
        let base = format!("/sys/class/power_supply/{name}");
        let Ok(status) = fs::read_to_string(format!("{base}/status")) else {
            continue;
        };
        let capacity = fs::read_to_string(format!("{base}/capacity"))
            .map(|c| format!("{}% ", c.trim()))
            .unwrap_or_default();
        return format!("{capacity}({})", status.trim());
    }
    "unknown".into()
}

/// The 1/5/15 minute load averages.
fn _loadavg() -> String {
    fs::read_to_string("/proc/loadavg")
        .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        .unwrap_or_else(|_| "unknown".into())
}
