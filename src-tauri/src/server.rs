use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::settings::SettingsStore;
use crate::text_clean::clean_for_speech;
use crate::transcript::last_assistant_text;
use crate::tts::TtsEngine;

#[derive(Clone)]
pub struct AppState {
    pub tts: Arc<TtsEngine>,
    pub settings: Arc<SettingsStore>,
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
}

#[derive(Serialize)]
struct StatusResponse {
    enabled: bool,
    speaking: bool,
    voice: String,
    rate: u32,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/status", get(status))
        .route("/speak", post(speak))
        .route("/stop", post(stop))
        .route("/hook/stop", post(hook_stop))
        .with_state(state)
}

async fn root() -> &'static str {
    "claude-voice bridge running"
}

async fn status(State(s): State<AppState>) -> Json<StatusResponse> {
    let cfg = s.settings.get();
    Json(StatusResponse {
        enabled: cfg.enabled,
        speaking: s.tts.is_speaking(),
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
    let text = clean_for_speech(&req.text);
    s.tts.speak(&text, &cfg.voice, cfg.rate);
    (StatusCode::OK, "ok")
}

async fn stop(State(s): State<AppState>) -> impl IntoResponse {
    s.tts.stop();
    (StatusCode::OK, "stopped")
}

async fn hook_stop(
    State(s): State<AppState>,
    Json(input): Json<StopHookInput>,
) -> impl IntoResponse {
    if input.stop_hook_active {
        return (StatusCode::OK, "skip");
    }
    let cfg = s.settings.get();
    if !cfg.enabled {
        return (StatusCode::OK, "disabled");
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

    let text = clean_for_speech(&raw);
    s.tts.speak(&text, &cfg.voice, cfg.rate);
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
