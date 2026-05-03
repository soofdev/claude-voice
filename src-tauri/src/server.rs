use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::bridge::BridgeQueue;
use crate::history::now_ms;
use crate::sessions::SessionsStore;
use crate::settings::SettingsStore;
use crate::transcript::last_assistant_text;
use crate::tts::TtsEngine;

#[derive(Clone)]
pub struct AppState {
    pub tts: Arc<TtsEngine>,
    pub settings: Arc<SettingsStore>,
    pub sessions: Arc<SessionsStore>,
    pub bridge: Arc<BridgeQueue>,
    pub app: tauri::AppHandle,
}

#[derive(Deserialize)]
struct SpeakRequest {
    text: String,
}

#[derive(Deserialize)]
struct StopHookInput {
    transcript_path: Option<String>,
    #[serde(default)]
    stop_hook_active: bool,
    #[serde(default)]
    last_assistant_message: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    tty: Option<String>,
}

#[derive(Serialize)]
struct StatusResponse {
    enabled: bool,
    speaking: bool,
    paused: bool,
    voice: String,
    rate: u32,
}

// Axum's default is 2 MiB. The endpoints that accept a body all carry a
// short JSON payload — pin a tight per-route ceiling so a misbehaving
// local process can't enqueue gigabytes of "speak this" or sit on the
// socket dribbling a slow body.
const SPEAK_BODY_LIMIT: usize = 64 * 1024;
// last_assistant_message can be a full agent response — keep this
// generous but bounded.
const HOOK_BODY_LIMIT: usize = 512 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/status", get(status))
        .route(
            "/speak",
            post(speak).layer(DefaultBodyLimit::max(SPEAK_BODY_LIMIT)),
        )
        .route("/stop", post(stop))
        .route("/pause", post(pause))
        .route("/resume", post(resume))
        .route("/toggle", post(toggle))
        .route(
            "/hook/stop",
            post(hook_stop).layer(DefaultBodyLimit::max(HOOK_BODY_LIMIT)),
        )
        .route("/bridge/pending/:sid", get(bridge_pending))
        .with_state(state)
}

async fn bridge_pending(
    State(s): State<AppState>,
    Path(sid): Path<String>,
) -> Json<Vec<String>> {
    Json(s.bridge.drain(&sid))
}

async fn root() -> &'static str {
    "claude-voice bridge running"
}

async fn status(State(s): State<AppState>) -> Json<StatusResponse> {
    let cfg = s.settings.get();
    Json(StatusResponse {
        enabled: cfg.enabled,
        speaking: s.tts.is_speaking(),
        paused: s.tts.is_paused(),
        voice: cfg.voice,
        rate: cfg.rate,
    })
}

async fn speak(
    State(s): State<AppState>,
    Json(req): Json<SpeakRequest>,
) -> impl IntoResponse {
    let cfg = s.settings.get();
    if !cfg.enabled {
        return (StatusCode::OK, "disabled");
    }
    s.tts.speak(req.text, cfg, None);
    (StatusCode::OK, "ok")
}

async fn stop(State(s): State<AppState>) -> impl IntoResponse {
    s.tts.stop();
    (StatusCode::OK, "stopped")
}

async fn pause(State(s): State<AppState>) -> impl IntoResponse {
    s.tts.pause();
    (StatusCode::OK, "paused")
}

async fn resume(State(s): State<AppState>) -> impl IntoResponse {
    s.tts.resume();
    (StatusCode::OK, "resumed")
}

async fn toggle(State(s): State<AppState>) -> impl IntoResponse {
    s.tts.toggle_pause();
    (StatusCode::OK, "ok")
}

async fn hook_stop(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(mut input): Json<StopHookInput>,
) -> impl IntoResponse {
    if input.stop_hook_active {
        return (StatusCode::OK, "skip");
    }
    // The hook command pipeline sets X-Claude-Voice-Tty so we don't have
    // to splice JSON in the shell. Trust the header only when the body
    // didn't already provide a tty (forward-compat with future Claude
    // Code versions that may include it natively), and ignore the "?"
    // sentinel that the script emits when no controlling terminal is
    // attached.
    if input.tty.as_deref().map_or(true, str::is_empty) {
        if let Some(t) = headers
            .get("x-claude-voice-tty")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|t| !t.is_empty() && *t != "/dev/?")
        {
            input.tty = Some(t.to_string());
        }
    }
    let cfg = s.settings.get();
    if !cfg.enabled {
        return (StatusCode::OK, "disabled");
    }

    let mut cfg = cfg;
    let mut session_tag: Option<crate::tts::SessionTag> = None;
    if let Some(sid) = input.session_id.as_deref() {
        let cwd = input.cwd.as_deref().unwrap_or("");
        let tty = input.tty.as_deref();
        s.sessions.upsert_active(sid, cwd, tty, now_ms());
        use tauri::Emitter;
        let _ = s.app.emit("sessions:changed", ());
        if !s.sessions.is_enabled(sid) {
            return (StatusCode::OK, "session-muted");
        }
        if let Some(info) = s.sessions.get(sid) {
            if let Some(voice) = info.voice_override.clone() {
                if cfg.backend == "elevenlabs" {
                    cfg.elevenlabs_voice_id = voice;
                } else {
                    cfg.voice = voice;
                }
            }
            session_tag = Some(crate::tts::SessionTag {
                id: info.session_id,
                label: info.label,
                color: info.color,
            });
        }
    }

    let raw = if let Some(msg) = input.last_assistant_message.filter(|m| !m.trim().is_empty()) {
        msg
    } else if let Some(path) = input.transcript_path {
        match last_assistant_text(&PathBuf::from(path)) {
            Some(t) => t,
            None => return (StatusCode::OK, "no-message"),
        }
    } else {
        return (StatusCode::OK, "no-input");
    };

    s.tts.speak(raw, cfg, session_tag);
    (StatusCode::OK, "ok")
}

pub async fn serve(state: AppState, port: u16) -> anyhow::Result<()> {
    let app = router(state);
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("[claude-voice] listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
