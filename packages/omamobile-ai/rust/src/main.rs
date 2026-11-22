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
    usb_boost: bool,
    usb_idle_timeout_secs: i64,
    boost_idle_timeout_secs: i64,
    boost_profile: String,
    boost_max_temp_mc: i64,
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
            usb_boost: false,
            usb_idle_timeout_secs: 0,
            boost_idle_timeout_secs: 60,
            boost_profile: "performance".into(),
            boost_max_temp_mc: 85000,
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
        "USB_BOOST" => _set_bool(value, path, key, &mut config.usb_boost),
        "USB_IDLE_TIMEOUT" => _set_number(value, path, key, |n| config.usb_idle_timeout_secs = n),
        "BOOST_IDLE_TIMEOUT" => {
            _set_number(value, path, key, |n| config.boost_idle_timeout_secs = n)
        }
        "BOOST_PROFILE" => config.boost_profile = value.into(),
        "BOOST_MAX_TEMP_MC" => _set_number(value, path, key, |n| config.boost_max_temp_mc = n),
        other => warn!("{}: unknown setting {:?}", path.display(), other),
    }
}

/// Parse boolean config values (`1`/`true`/`yes`/`on` vs `0`/`false`/`no`/`off`).
fn _set_bool(value: &str, path: &Path, key: &str, target: &mut bool) {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => *target = true,
        "0" | "false" | "no" | "off" => *target = false,
        _ => warn!("{}: bad boolean for {}: {:?}", path.display(), key, value),
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

/// Runtime state: config, llama-server child, boost hold child, and last request time.
struct AppState {
    config: Config,
    llama: Mutex<Option<tokio::process::Child>>,
    boost: Mutex<Option<tokio::process::Child>>,
    last_request: AtomicI64,
}

impl AppState {
    fn new(config: Config) -> Self {
        Self {
            config,
            llama: Mutex::new(None),
            boost: Mutex::new(None),
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
    _release_boost(&state, "shutdown").await;
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
        // llama.cpp router surface: Pi's /login llama.cpp validates a server
        // by listing it this way, so the phone is also a "router" with one
        // model. Loading and unloading map onto our lazy lifecycle.
        .route("/models", get(_handle_router_models))
        .route("/props", get(_handle_router_props))
        .route("/models/load", post(_handle_router_load))
        .route("/models/unload", post(_handle_router_unload))
        .fallback(_handle_not_found)
        .with_state(state)
}

// ─── Handlers ────────────────────────────────────────────────────────

/// GET /health: liveness plus RAM, battery, boost state, load and model state.
async fn _handle_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let running = state.llama.lock().await.is_some();
    let boost_active = state.boost.lock().await.is_some();
    let usb_connected = _usb_connected();
    let power_profile = _power_profile().await;
    Json(json!({
        "status": "ok",
        "model": _model_id(&state.config),
        "model_loaded": running,
        "idle_timeout_secs": state.config.idle_timeout_secs,
        "usb_boost": state.config.usb_boost,
        "usb_connected": usb_connected,
        "usb_idle_timeout_secs": state.config.usb_idle_timeout_secs,
        "boost_active": boost_active,
        "power_profile": power_profile,
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
    _sync_boost(&state).await;
    let res = _proxy_chat(&state, body).await;
    _record_request(&state);
    _sync_boost(&state).await;
    res
}

/// GET /models: the llama.cpp router catalog, with the one model we serve.
/// `status.value` must be present or clients call the server "not a router".
async fn _handle_router_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = if state.llama.lock().await.is_some() {
        "loaded"
    } else {
        "unloaded"
    };
    Json(json!({
        "object": "list",
        "data": [{
            "id": _model_id(&state.config),
            "status": { "value": status },
            "source": "local",
            "meta": {
                "n_ctx": state.config.context_size,
                "n_ctx_train": state.config.context_size,
            },
            "architecture": { "input_modalities": ["text"] },
        }],
    }))
}

/// GET /props: only the router flag is read; nothing here autoloads.
async fn _handle_router_props(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "models_autoload": false,
        "default_generation_settings": {},
        "model_path": state.config.model_path,
    }))
}

/// POST /models/load: the router's explicit load, which is our lazy spawn.
async fn _handle_router_load(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    _ensure_llama(&state).await.map_err(|e| {
        error!("load failed: {e}");
        StatusCode::SERVICE_UNAVAILABLE
    })?;
    _record_request(&state);
    _sync_boost(&state).await;
    Ok(Json(json!({ "success": true })))
}

/// POST /models/unload: the router's unload, which frees the model now.
async fn _handle_router_unload(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    _stop_llama(&state).await;
    Json(json!({ "success": true }))
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

/// Kill and reap llama-server, if we own one, and drop any performance boost.
async fn _stop_llama(state: &AppState) {
    let mut guard = state.llama.lock().await;
    if let Some(mut child) = guard.take() {
        let _ = child.start_kill();
        let _ = child.wait().await;
        info!("llama-server stopped, model unloaded");
    }
    drop(guard);
    state.last_request.store(0, Ordering::Relaxed);
    _release_boost(state, "model unloaded").await;
}

/// Kill llama-server once the effective idle timeout has passed and sync boost state.
async fn _idle_watchdog(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(WATCHDOG_TICK).await;
        if _idle_expired(&state) {
            info!("idle timeout reached");
            _stop_llama(&state).await;
        } else {
            _sync_boost(&state).await;
        }
    }
}

/// True when a model is loaded and nobody has asked for it in a while.
/// When USB_BOOST=1 and USB is connected, USB_IDLE_TIMEOUT (0 = stay resident)
/// replaces IDLE_TIMEOUT so the model remains warm across sessions.
fn _idle_expired(state: &AppState) -> bool {
    _idle_expired_with(state, _usb_connected(), _unix_now())
}

fn _idle_expired_with(state: &AppState, usb_connected: bool, now: i64) -> bool {
    let last = state.last_request.load(Ordering::Relaxed);
    if last == 0 {
        return false; // Nothing was started through us yet.
    }
    let timeout = if state.config.usb_boost && usb_connected {
        state.config.usb_idle_timeout_secs
    } else {
        state.config.idle_timeout_secs
    };
    if timeout <= 0 {
        return false;
    }
    now.saturating_sub(last) >= timeout
}

fn _boost_expired_with(state: &AppState, now: i64) -> bool {
    let limit = state.config.boost_idle_timeout_secs;
    if limit <= 0 {
        return false;
    }
    let last = state.last_request.load(Ordering::Relaxed);
    if last == 0 {
        return true;
    }
    now.saturating_sub(last) >= limit
}

// ─── USB-powered performance boost ────────────────────────────────────

/// Synchronize the powerprofilesctl performance hold with USB, load, and thermal state.
async fn _sync_boost(state: &AppState) {
    if !state.config.usb_boost {
        _release_boost(state, "disabled").await;
        return;
    }
    let llama_running = state.llama.lock().await.is_some();
    let usb_up = _usb_connected();
    let hot = _thermal_hot(state.config.boost_max_temp_mc);
    let idle = _boost_expired_with(state, _unix_now());

    if llama_running && usb_up && !hot && !idle {
        _acquire_boost(state).await;
    } else {
        let reason = if !llama_running {
            "model idle/unloaded"
        } else if !usb_up {
            "USB unplugged"
        } else if hot {
            "thermal guard"
        } else {
            "boost idle timeout"
        };
        _release_boost(state, reason).await;
    }
}

/// Acquire a temporary powerprofilesctl HoldProfile child kept alive via stdin pipe.
async fn _acquire_boost(state: &AppState) {
    let mut guard = state.boost.lock().await;
    if let Some(child) = guard.as_mut() {
        if child.try_wait().ok().flatten().is_none() {
            return;
        }
        *guard = None;
    }
    let profile = &state.config.boost_profile;
    let mut cmd = tokio::process::Command::new("powerprofilesctl");
    cmd.args([
        "launch",
        "-p",
        profile,
        "-r",
        "USB AI serving",
        "-i",
        "omamobile-ai",
        "--",
        "sh",
        "-c",
        "read _",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .kill_on_drop(true);

    match cmd.spawn() {
        Ok(child) => {
            info!("acquired USB powerprofilesctl boost ({profile})");
            *guard = Some(child);
        }
        Err(e) => warn!("could not acquire powerprofilesctl boost ({profile}): {e}"),
    }
}

/// Close stdin pipe and reap the HoldProfile helper so power-profiles-daemon reverts cleanly.
async fn _release_boost(state: &AppState, reason: &str) {
    let mut guard = state.boost.lock().await;
    if let Some(mut child) = guard.take() {
        drop(child.stdin.take());
        let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
        let _ = child.start_kill();
        let _ = child.wait().await;
        info!("released powerprofilesctl boost ({reason})");
    }
}

/// True when USB power supply is online or USB gadget UDC is configured.
fn _usb_connected() -> bool {
    _usb_connected_at(Path::new("/sys"))
}

fn _usb_connected_at(sys: &Path) -> bool {
    let ps = sys.join("class/power_supply");
    if let Ok(entries) = fs::read_dir(ps) {
        for entry in entries.flatten() {
            let dir = entry.path();
            let ptype = fs::read_to_string(dir.join("type")).unwrap_or_default();
            if ptype.trim().starts_with("USB") {
                let online = fs::read_to_string(dir.join("online")).unwrap_or_default();
                if online.trim() == "1" {
                    return true;
                }
            }
        }
    }
    let udc = sys.join("class/udc");
    if let Ok(entries) = fs::read_dir(udc) {
        for entry in entries.flatten() {
            let state = fs::read_to_string(entry.path().join("state")).unwrap_or_default();
            if state.trim() == "configured" {
                return true;
            }
        }
    }
    false
}

/// True when any thermal zone reads at or above `max_mc` millidegrees C.
fn _thermal_hot(max_mc: i64) -> bool {
    _thermal_hot_at(Path::new("/sys"), max_mc)
}

fn _thermal_hot_at(sys: &Path, max_mc: i64) -> bool {
    if max_mc <= 0 {
        return false;
    }
    let dir = sys.join("class/thermal");
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let Ok(raw) = fs::read_to_string(entry.path().join("temp")) else {
                continue;
            };
            if let Ok(temp) = raw.trim().parse::<i64>() {
                if temp >= max_mc {
                    return true;
                }
            }
        }
    }
    false
}

/// Read the active power-profiles-daemon profile, if available.
async fn _power_profile() -> String {
    match tokio::process::Command::new("powerprofilesctl")
        .arg("get")
        .output()
        .await
    {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => "unknown".into(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn _test_config_boost_flags() {
        let mut cfg = Config::default();
        let p = Path::new("/tmp/test.conf");
        _apply_setting("USB_BOOST", "1", &mut cfg, p);
        _apply_setting("USB_IDLE_TIMEOUT", "0", &mut cfg, p);
        _apply_setting("BOOST_IDLE_TIMEOUT", "45", &mut cfg, p);
        _apply_setting("BOOST_PROFILE", "performance", &mut cfg, p);
        _apply_setting("BOOST_MAX_TEMP_MC", "75000", &mut cfg, p);
        assert!(cfg.usb_boost);
        assert_eq!(cfg.usb_idle_timeout_secs, 0);
        assert_eq!(cfg.boost_idle_timeout_secs, 45);
        assert_eq!(cfg.boost_profile, "performance");
        assert_eq!(cfg.boost_max_temp_mc, 75000);
    }

    #[test]
    fn _test_idle_keeps_resident_on_usb() {
        let mut cfg = Config::default();
        cfg.usb_boost = true;
        cfg.idle_timeout_secs = 180;
        cfg.usb_idle_timeout_secs = 0;
        cfg.boost_idle_timeout_secs = 60;
        let state = AppState::new(cfg);
        state.last_request.store(100, Ordering::Relaxed);
        assert!(!_idle_expired_with(&state, true, 5000));
        assert!(_idle_expired_with(&state, false, 280));
        assert!(!_boost_expired_with(&state, 159));
        assert!(_boost_expired_with(&state, 160));
    }

    #[test]
    fn _test_sysfs_usb_and_thermal() {
        let tmp = std::env::temp_dir().join(format!("omamobile-ai-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let chg = tmp.join("class/power_supply/max77705-charger");
        let tz = tmp.join("class/thermal/thermal_zone0");
        fs::create_dir_all(&chg).unwrap();
        fs::create_dir_all(&tz).unwrap();
        fs::write(chg.join("type"), "USB\n").unwrap();
        fs::write(chg.join("online"), "0\n").unwrap();
        fs::write(tz.join("temp"), "42000\n").unwrap();
        assert!(!_usb_connected_at(&tmp));
        assert!(!_thermal_hot_at(&tmp, 78000));
        fs::write(chg.join("online"), "1\n").unwrap();
        fs::write(tz.join("temp"), "79000\n").unwrap();
        assert!(_usb_connected_at(&tmp));
        assert!(_thermal_hot_at(&tmp, 78000));
        let _ = fs::remove_dir_all(&tmp);
    }
}
