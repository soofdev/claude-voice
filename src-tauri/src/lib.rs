mod bridge;
mod code_extract;
mod fs_util;
mod history;
mod link_extract;
mod server;
mod sessions;
mod settings;
mod text_clean;
mod transcript;
mod tts;

use std::sync::Arc;

use bridge::BridgeQueue;
use history::{HistoryEntry, HistoryStore};
use server::AppState;
use sessions::{SessionInfo, SessionsStore};
use settings::{Settings, SettingsStore};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tts::TtsEngine;

const POPUP_WIDTH: f64 = 680.0;
const POPUP_HEIGHT: f64 = 620.0;
// Low enough to allow the minimized orb (see SIZE_ORB in popup.js) to
// actually shrink to its target — otherwise the OS clamps the resize and
// the click-blocking rect stays at the larger size.
const POPUP_MIN_WIDTH: f64 = 140.0;
const POPUP_MIN_HEIGHT: f64 = 140.0;
const POPUP_MARGIN_RIGHT: f64 = 12.0;
const POPUP_MARGIN_TOP: f64 = 32.0;

struct MenuRefs {
    enabled: CheckMenuItem<tauri::Wry>,
    pin_popup: CheckMenuItem<tauri::Wry>,
}

#[tauri::command]
fn get_settings(store: tauri::State<Arc<SettingsStore>>) -> Settings {
    store.get()
}

#[tauri::command]
fn set_settings(
    new: Settings,
    store: tauri::State<Arc<SettingsStore>>,
    menu_refs: tauri::State<Arc<MenuRefs>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    store.update(new.clone()).map_err(|e| e.to_string())?;
    let _ = menu_refs.enabled.set_checked(new.enabled);
    let _ = menu_refs.pin_popup.set_checked(new.pin_popup);
    let _ = app.emit("settings:changed", new);
    Ok(())
}

#[tauri::command]
fn toggle_pin_popup(
    store: tauri::State<Arc<SettingsStore>>,
    menu_refs: tauri::State<Arc<MenuRefs>>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let mut cfg = store.get();
    cfg.pin_popup = !cfg.pin_popup;
    store.update(cfg.clone()).map_err(|e| e.to_string())?;
    let _ = menu_refs.pin_popup.set_checked(cfg.pin_popup);
    let _ = app.emit("settings:changed", cfg.clone());
    Ok(cfg.pin_popup)
}

#[tauri::command]
fn list_voices() -> Vec<String> {
    tts::list_voices()
}

#[tauri::command]
fn test_speak(
    text: String,
    tts: tauri::State<Arc<TtsEngine>>,
    store: tauri::State<Arc<SettingsStore>>,
) {
    let cfg = store.get();
    tts.speak(text, cfg, None);
}

#[tauri::command]
async fn list_elevenlabs_voices(
    api_key: String,
    tts: tauri::State<'_, Arc<TtsEngine>>,
) -> Result<Vec<(String, String)>, String> {
    let http = tts.http();
    tts::list_elevenlabs_voices(&http, &api_key)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_speaking(tts: tauri::State<Arc<TtsEngine>>) {
    tts.stop();
}

#[tauri::command]
fn toggle_pause(tts: tauri::State<Arc<TtsEngine>>) {
    tts.toggle_pause();
}

#[tauri::command]
fn get_sessions(store: tauri::State<Arc<SessionsStore>>) -> Vec<SessionInfo> {
    store.list()
}

#[tauri::command]
fn set_session_enabled(
    id: String,
    enabled: bool,
    store: tauri::State<Arc<SessionsStore>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    store.set_enabled(&id, enabled);
    let _ = app.emit("sessions:changed", ());
    Ok(())
}

#[tauri::command]
fn rename_session(
    id: String,
    label: String,
    store: tauri::State<Arc<SessionsStore>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    store.rename(&id, &label);
    let _ = app.emit("sessions:changed", ());
    Ok(())
}

#[tauri::command]
fn set_session_voice(
    id: String,
    voice_id: Option<String>,
    store: tauri::State<Arc<SessionsStore>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    store.set_voice(&id, voice_id);
    let _ = app.emit("sessions:changed", ());
    Ok(())
}

#[tauri::command]
fn set_session_color(
    id: String,
    color: Option<String>,
    store: tauri::State<Arc<SessionsStore>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    store.set_color(&id, color);
    let _ = app.emit("sessions:changed", ());
    Ok(())
}

#[tauri::command]
fn remove_session(
    id: String,
    store: tauri::State<Arc<SessionsStore>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    store.remove(&id);
    let _ = app.emit("sessions:changed", ());
    Ok(())
}

#[tauri::command]
fn get_history(store: tauri::State<Arc<HistoryStore>>) -> Vec<HistoryEntry> {
    store.list()
}

#[tauri::command]
fn replay_history(
    id: String,
    tts: tauri::State<Arc<tts::TtsEngine>>,
    history: tauri::State<Arc<HistoryStore>>,
    settings: tauri::State<Arc<SettingsStore>>,
) -> Result<(), String> {
    let entry = history
        .get(&id)
        .ok_or_else(|| "entry not found".to_string())?;
    let cfg = settings.get();
    tts.replay(entry, cfg);
    Ok(())
}

#[tauri::command]
fn delete_history_entry(
    id: String,
    history: tauri::State<Arc<HistoryStore>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    history.remove_entry(&id);
    let _ = app.emit("history:changed", ());
    Ok(())
}

#[tauri::command]
fn clear_history(
    history: tauri::State<Arc<HistoryStore>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    history.clear();
    let _ = app.emit("history:changed", ());
    Ok(())
}

#[tauri::command]
async fn transcribe_audio(
    audio: Vec<u8>,
    mime: String,
    settings: tauri::State<'_, Arc<SettingsStore>>,
    tts: tauri::State<'_, Arc<tts::TtsEngine>>,
) -> Result<String, String> {
    let cfg = settings.get();
    if cfg.elevenlabs_api_key.is_empty() {
        return Err("ElevenLabs API key not set".to_string());
    }
    if audio.is_empty() {
        return Err("empty audio".to_string());
    }
    let http = tts.http();
    let ext = match mime.as_str() {
        m if m.contains("webm") => "webm",
        m if m.contains("mp4") || m.contains("m4a") => "m4a",
        m if m.contains("ogg") => "ogg",
        m if m.contains("wav") => "wav",
        m if m.contains("mpeg") || m.contains("mp3") => "mp3",
        _ => "webm",
    };
    let part = reqwest::multipart::Part::bytes(audio)
        .file_name(format!("recording.{ext}"))
        .mime_str(&mime)
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .text("model_id", "scribe_v1")
        .part("file", part);
    let resp = http
        .post("https://api.elevenlabs.io/v1/speech-to-text")
        .header("xi-api-key", &cfg.elevenlabs_api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("ElevenLabs STT {status}: {err}"));
    }
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let text = v
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    Ok(text)
}

#[tauri::command]
fn send_to_terminal(
    text: String,
    session_id: Option<String>,
    sessions: tauri::State<Arc<SessionsStore>>,
    bridge: tauri::State<Arc<BridgeQueue>>,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("empty prompt".to_string());
    }
    // Web-based agent sessions (Replit today, more later) are served by the
    // Chrome extension, not a local TTY. Queue the text and let the
    // extension inject it into the agent's input on its next poll.
    if let Some(sid) = session_id.as_deref() {
        if bridge::is_bridge_session(sid) {
            bridge.enqueue(sid, text);
            return Ok("Replit (via bridge)".to_string());
        }
    }
    let tty = session_id
        .as_deref()
        .and_then(|sid| sessions.get(sid))
        .and_then(|s| s.tty);
    #[cfg(target_os = "macos")]
    {
        terminal_send::send(&text, tty.as_deref())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (text, tty);
        Err("send-to-terminal is only implemented on macOS".to_string())
    }
}

#[cfg(target_os = "macos")]
mod terminal_send {
    use std::process::Command;

    pub fn send(text: &str, tty: Option<&str>) -> Result<String, String> {
        if let Some(tty) = tty {
            if let Some(result) = try_iterm2_by_tty(text, tty) {
                return result;
            }
            if let Some(result) = try_terminal_app_by_tty(text, tty) {
                return result;
            }
        }
        if let Some(result) = try_iterm2_current(text) {
            return result;
        }
        try_terminal_app_current(text)
    }

    fn try_iterm2_by_tty(text: &str, tty: &str) -> Option<Result<String, String>> {
        if !is_app_running("iTerm2") {
            return None;
        }
        let escaped_text = applescript_escape(text);
        let escaped_tty = applescript_escape(tty);
        let script = format!(
            r#"tell application "iTerm2"
    set found to false
    repeat with w in windows
        repeat with t in tabs of w
            repeat with s in sessions of t
                if tty of s is "{tty}" then
                    tell s to write text "{text}"
                    set found to true
                end if
            end repeat
        end repeat
    end repeat
    if not found then error "no matching tty"
end tell"#,
            tty = escaped_tty,
            text = escaped_text
        );
        match run_osascript(&script) {
            Ok(_) => Some(Ok(format!("iTerm2 ({tty})"))),
            Err(_) => None,
        }
    }

    fn try_iterm2_current(text: &str) -> Option<Result<String, String>> {
        if !is_app_running("iTerm2") {
            return None;
        }
        let escaped = applescript_escape(text);
        let script = format!(
            r#"tell application "iTerm2" to tell current session of current window to write text "{escaped}""#
        );
        Some(run_osascript(&script).map(|_| "iTerm2 (current)".to_string()))
    }

    fn try_terminal_app_by_tty(text: &str, tty: &str) -> Option<Result<String, String>> {
        if !is_app_running("Terminal") {
            return None;
        }
        let escaped_tty = applescript_escape(tty);
        let escaped_text = applescript_escape(text);
        let script = format!(
            r#"tell application "Terminal"
    set matched to missing value
    repeat with w in windows
        repeat with t in tabs of w
            if tty of t is "{tty}" then set matched to t
        end repeat
    end repeat
    if matched is missing value then error "no matching tty"
    do script "{text}" in matched
end tell"#,
            tty = escaped_tty,
            text = escaped_text
        );
        match run_osascript(&script) {
            Ok(_) => Some(Ok(format!("Terminal.app ({tty})"))),
            Err(_) => None,
        }
    }

    fn try_terminal_app_current(text: &str) -> Result<String, String> {
        if !is_app_running("Terminal") {
            return Err("No supported terminal is running (iTerm2 or Terminal.app)".to_string());
        }
        let escaped = applescript_escape(text);
        let script = format!(
            r#"tell application "Terminal" to activate
delay 0.12
tell application "System Events"
    keystroke "{escaped}"
    key code 36
end tell"#
        );
        run_osascript(&script).map(|_| "Terminal.app (current)".to_string())
    }

    fn is_app_running(name: &str) -> bool {
        let script =
            format!(r#"tell application "System Events" to (name of processes) contains "{name}""#);
        match Command::new("osascript").arg("-e").arg(&script).output() {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "true",
            Err(_) => false,
        }
    }

    fn run_osascript(script: &str) -> Result<(), String> {
        let out = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| format!("osascript failed: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() {
                "osascript returned non-zero".to_string()
            } else {
                err
            });
        }
        Ok(())
    }

    fn applescript_escape(s: &str) -> String {
        // AppleScript string literals are single-line: a raw newline,
        // carriage return, or tab inside the quoted text terminates the
        // string mid-script and produces a syntax error. Replace them
        // with the AppleScript escape forms so a multi-line reply
        // round-trips intact. Backslash must be done first so the
        // backslashes we insert below don't get re-doubled.
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    #[cfg(test)]
    mod tests {
        use super::applescript_escape;

        #[test]
        fn escapes_quote() {
            assert_eq!(applescript_escape("a\"b"), "a\\\"b");
        }

        #[test]
        fn escapes_backslash() {
            assert_eq!(applescript_escape("a\\b"), "a\\\\b");
        }

        #[test]
        fn escapes_newline() {
            assert_eq!(applescript_escape("a\nb"), "a\\nb");
        }

        #[test]
        fn escapes_cr_and_tab() {
            assert_eq!(applescript_escape("a\rb\tc"), "a\\rb\\tc");
        }

        #[test]
        fn backslash_before_quote_does_not_collide() {
            // Input: `a\"b` (backslash followed by quote). We must not
            // produce `a\\"b` (which would let the quote escape the
            // string); the correct output is `a\\\"b`.
            assert_eq!(applescript_escape("a\\\"b"), "a\\\\\\\"b");
        }
    }
}

#[tauri::command]
fn hook_command(store: tauri::State<Arc<SettingsStore>>) -> String {
    let port = store.get().port;
    build_hook_command(port)
}

fn build_hook_command(port: u16) -> String {
    // Fire-and-forget: --max-time keeps us from hanging on a dead server,
    // -o /dev/null drops the response body, and `|| true` guarantees a
    // zero exit so Claude Code never reports "Failed with non-blocking
    // status code" just because Claude Voice was restarting.
    //
    // The TTY rides along as an HTTP header rather than being spliced
    // into the JSON body. The previous form used `cat | sed 's/^{//'`
    // to strip Claude Code's leading `{` and prepend `"tty":"/dev/X",`
    // — which broke if the JSON ever started with whitespace and
    // produced `"tty":"/dev/"` when no TTY was attached. The header
    // form also avoids any shell-quoting concerns around the JSON.
    format!(
        "TTY=$(ps -o tty= -p $$ | tr -d ' '); [ -z \"$TTY\" ] && TTY=\"?\"; \
         curl -s --max-time 5 -o /dev/null -X POST \
         http://127.0.0.1:{port}/hook/stop \
         -H 'Content-Type: application/json' \
         -H \"X-Claude-Voice-Tty: /dev/$TTY\" \
         --data-binary @- || true"
    )
}

#[cfg(test)]
mod hook_command_tests {
    use super::build_hook_command;

    #[test]
    fn includes_port_in_url() {
        let cmd = build_hook_command(9000);
        assert!(cmd.contains("http://127.0.0.1:9000/hook/stop"), "{cmd}");
    }

    #[test]
    fn carries_tty_via_header_not_body() {
        let cmd = build_hook_command(8765);
        assert!(cmd.contains("X-Claude-Voice-Tty: /dev/$TTY"), "{cmd}");
        // The old form spliced JSON in the shell — make sure we don't
        // regress to it.
        assert!(!cmd.contains("sed "), "{cmd}");
        assert!(!cmd.contains("printf "), "{cmd}");
    }

    #[test]
    fn is_fire_and_forget() {
        let cmd = build_hook_command(8765);
        assert!(cmd.contains("--max-time 5"), "{cmd}");
        assert!(cmd.ends_with("|| true"), "{cmd}");
    }
}

#[tauri::command]
fn set_history_panel_width(
    width: f64,
    store: tauri::State<Arc<SettingsStore>>,
) -> Result<(), String> {
    let mut cfg = store.get();
    cfg.history_panel_width = Some(width);
    store.update(cfg).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn install_hook(store: tauri::State<Arc<SettingsStore>>) -> Result<String, String> {
    let port = store.get().port;
    install_hook_impl(port)
}

fn show_settings_window(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.unminimize();
        // The popup is always-on-top (Floating level), so the settings
        // window — opened at the normal level — would appear behind it.
        // Temporarily flip always-on-top, raise focus, then unflag so
        // the window obeys normal stacking once the user clicks away.
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.set_always_on_top(false);
    }
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_settings_window(&app);
}

#[tauri::command]
fn toggle_speaking(app: tauri::AppHandle) {
    let store = app.state::<Arc<SettingsStore>>();
    let mut cfg = store.get();
    cfg.enabled = !cfg.enabled;
    let _ = store.update(cfg.clone());
    if let Some(refs) = app.try_state::<Arc<MenuRefs>>() {
        let _ = refs.enabled.set_checked(cfg.enabled);
    }
    if !cfg.enabled {
        let tts = app.state::<Arc<TtsEngine>>();
        tts.stop();
    }
    let _ = app.emit("settings:changed", cfg);
}

#[tauri::command]
fn hook_installed() -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let path = home.join(".claude").join("settings.json");
    match std::fs::read_to_string(&path) {
        Ok(s) => s.contains("/hook/stop"),
        Err(_) => false,
    }
}

fn install_hook_impl(port: u16) -> Result<String, String> {
    let home = dirs::home_dir().ok_or_else(|| "no home dir".to_string())?;
    let path = home.join(".claude").join("settings.json");
    let marker = "/hook/stop".to_string();

    let existing = std::fs::read_to_string(&path).ok();
    let mut root: serde_json::Value = match existing.as_deref() {
        Some(s) if !s.trim().is_empty() => serde_json::from_str(s)
            .map_err(|e| format!("failed to parse {}: {e}", path.display()))?,
        _ => serde_json::json!({}),
    };

    if !root.is_object() {
        return Err(format!("{} is not a JSON object", path.display()));
    }

    let hooks = root
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    if !hooks.is_object() {
        return Err("hooks is not a JSON object".to_string());
    }
    let stop = hooks
        .as_object_mut()
        .unwrap()
        .entry("Stop")
        .or_insert_with(|| serde_json::json!([]));
    if !stop.is_array() {
        return Err("hooks.Stop is not an array".to_string());
    }

    let new_cmd = build_hook_command(port);
    let mut updated = false;

    for entry in stop.as_array_mut().unwrap().iter_mut() {
        let Some(inner) = entry.get_mut("hooks") else {
            continue;
        };
        let Some(inner_arr) = inner.as_array_mut() else {
            continue;
        };
        for h in inner_arr.iter_mut() {
            let is_ours = h
                .get("command")
                .and_then(|c| c.as_str())
                .map(|c| c.contains(&marker))
                .unwrap_or(false);
            if is_ours {
                h.as_object_mut().unwrap().insert(
                    "command".to_string(),
                    serde_json::Value::String(new_cmd.clone()),
                );
                h.as_object_mut().unwrap().insert(
                    "type".to_string(),
                    serde_json::Value::String("command".to_string()),
                );
                updated = true;
            }
        }
    }

    let status = if !updated {
        stop.as_array_mut().unwrap().push(serde_json::json!({
            "hooks": [{
                "type": "command",
                "command": new_cmd
            }]
        }));
        "Installed"
    } else {
        "Updated"
    };

    // First-time install: drop a one-shot .bak alongside the original so
    // the user can recover from a bad parse or an unwanted edit. We don't
    // overwrite an existing .bak — that would clobber a known-good copy
    // with a possibly-already-modified file.
    if status == "Installed" {
        if let Some(prev) = existing.as_deref() {
            let bak = path.with_extension("json.bak");
            if !bak.exists() {
                let _ = fs_util::write_atomic(&bak, prev.as_bytes(), 0o600);
            }
        }
    }

    let pretty = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    fs_util::write_atomic(&path, pretty.as_bytes(), 0o600).map_err(|e| e.to_string())?;
    Ok(format!("{status} hook in {}", path.display()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings_store = Arc::new(SettingsStore::new());
    let history_store = Arc::new(HistoryStore::new());
    let sessions_store = Arc::new(SessionsStore::new());
    let bridge_queue = Arc::new(BridgeQueue::new());
    let tts = Arc::new(TtsEngine::new());
    tts.set_history(history_store.clone());

    let initial = settings_store.get();
    let port = initial.port;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    toggle_popup_visibility(app);
                })
                .build(),
        )
        .manage(settings_store.clone())
        .manage(history_store.clone())
        .manage(sessions_store.clone())
        .manage(bridge_queue.clone())
        .manage(tts.clone())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            list_voices,
            list_elevenlabs_voices,
            test_speak,
            stop_speaking,
            toggle_pause,
            toggle_pin_popup,
            get_history,
            replay_history,
            delete_history_entry,
            clear_history,
            send_to_terminal,
            transcribe_audio,
            install_hook,
            hook_installed,
            open_settings,
            toggle_speaking,
            set_history_panel_width,
            get_sessions,
            set_session_enabled,
            rename_session,
            set_session_voice,
            set_session_color,
            remove_session,
            hook_command,
        ])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();

            let tts_state = app.state::<Arc<TtsEngine>>();
            tts_state.set_app_handle(handle.clone());

            let popup =
                WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("popup.html".into()))
                    .decorations(false)
                    .transparent(true)
                    .always_on_top(true)
                    .resizable(true)
                    .focused(false)
                    .skip_taskbar(true)
                    .visible(false)
                    .inner_size(POPUP_WIDTH, POPUP_HEIGHT)
                    .min_inner_size(POPUP_MIN_WIDTH, POPUP_MIN_HEIGHT)
                    .shadow(false)
                    .accept_first_mouse(true)
                    .build()?;

            let saved_cfg = settings_store.get();
            // Saved geometry is only persisted for the expanded popup
            // (see persist_popup_geometry), so restore with the expanded
            // minimums, not the smaller orb-mode minimum.
            let w = saved_cfg.popup_width.unwrap_or(POPUP_WIDTH).max(240.0);
            let h = saved_cfg.popup_height.unwrap_or(POPUP_HEIGHT).max(240.0);
            let _ = popup.set_size(LogicalSize::new(w, h));
            match saved_cfg.popup_x.zip(saved_cfg.popup_y) {
                Some((x, y)) if position_on_any_monitor(&popup, x, y, w, h) => {
                    let _ = popup.set_position(LogicalPosition::new(x, y));
                }
                _ => position_popup(&popup),
            }
            #[cfg(target_os = "macos")]
            {
                let _ = popup.set_visible_on_all_workspaces(true);
            }

            let popup_for_start = popup.clone();
            let store_for_start = settings_store.clone();
            handle.listen("voice:start", move |_| {
                if !store_for_start.get().show_popup {
                    return;
                }
                let _ = popup_for_start.show();
            });

            let popup_for_wake = popup.clone();
            let store_for_wake = settings_store.clone();
            handle.listen("voice:waking", move |_| {
                if !store_for_wake.get().show_popup {
                    return;
                }
                let _ = popup_for_wake.show();
            });

            // popup owns its own hide logic (respects pin)

            let settings_window =
                WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
                    .title("Claude Voice Settings")
                    .inner_size(480.0, 820.0)
                    .min_inner_size(440.0, 560.0)
                    .resizable(true)
                    .visible(false)
                    .build()?;
            let _ = settings_window.hide();

            let enabled_item = CheckMenuItem::with_id(
                &handle,
                "toggle_enabled",
                "Speaking Enabled",
                true,
                initial.enabled,
                None::<&str>,
            )?;
            let pause_item = MenuItem::with_id(
                &handle,
                "toggle_pause",
                "Pause / Resume",
                true,
                None::<&str>,
            )?;
            let stop_item = MenuItem::with_id(
                &handle,
                "stop_speaking",
                "Stop Speaking",
                true,
                None::<&str>,
            )?;
            let pin_item = CheckMenuItem::with_id(
                &handle,
                "toggle_pin_popup",
                "Pin Popup",
                true,
                initial.pin_popup,
                None::<&str>,
            )?;
            let reset_popup_item = MenuItem::with_id(
                &handle,
                "reset_popup_position",
                "Reset Popup Position",
                true,
                None::<&str>,
            )?;
            let sessions_submenu =
                Submenu::with_id_and_items(&handle, "sessions_submenu", "Sessions", true, &[])?;
            rebuild_sessions_submenu(&sessions_submenu, &sessions_store.list(), &handle)?;
            let settings_item =
                MenuItem::with_id(&handle, "open_settings", "Settings…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(&handle, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                &handle,
                &[
                    &enabled_item,
                    &pause_item,
                    &stop_item,
                    &PredefinedMenuItem::separator(&handle)?,
                    &pin_item,
                    &reset_popup_item,
                    &sessions_submenu,
                    &settings_item,
                    &PredefinedMenuItem::separator(&handle)?,
                    &quit_item,
                ],
            )?;

            let menu_refs = Arc::new(MenuRefs {
                enabled: enabled_item.clone(),
                pin_popup: pin_item.clone(),
            });
            app.manage(menu_refs);

            let submenu_listen = sessions_submenu.clone();
            let sessions_for_listen = sessions_store.clone();
            let handle_for_listen = handle.clone();
            handle.listen("sessions:changed", move |_| {
                let list = sessions_for_listen.list();
                if let Err(e) = rebuild_sessions_submenu(&submenu_listen, &list, &handle_for_listen)
                {
                    eprintln!("[claude-voice] sessions submenu rebuild failed: {e}");
                }
            });

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

            TrayIconBuilder::with_id("main")
                .icon(icon)
                .icon_as_template(true)
                .tooltip("Claude Voice")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "toggle_enabled" => {
                        let store = app.state::<Arc<SettingsStore>>();
                        let mut cfg = store.get();
                        cfg.enabled = !cfg.enabled;
                        let _ = store.update(cfg.clone());
                        let refs = app.state::<Arc<MenuRefs>>();
                        let _ = refs.enabled.set_checked(cfg.enabled);
                        if !cfg.enabled {
                            let tts = app.state::<Arc<TtsEngine>>();
                            tts.stop();
                        }
                    }
                    "stop_speaking" => {
                        let tts = app.state::<Arc<TtsEngine>>();
                        tts.stop();
                    }
                    "toggle_pause" => {
                        let tts = app.state::<Arc<TtsEngine>>();
                        tts.toggle_pause();
                    }
                    "reset_popup_position" => {
                        if let Some(win) = app.get_webview_window("popup") {
                            position_popup(&win);
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "toggle_pin_popup" => {
                        let store = app.state::<Arc<SettingsStore>>();
                        let mut cfg = store.get();
                        cfg.pin_popup = !cfg.pin_popup;
                        let _ = store.update(cfg.clone());
                        let refs = app.state::<Arc<MenuRefs>>();
                        let _ = refs.pin_popup.set_checked(cfg.pin_popup);
                        let _ = app.emit("settings:changed", cfg);
                    }
                    other if other.starts_with("session_enable_") => {
                        let id = &other["session_enable_".len()..];
                        let sessions = app.state::<Arc<SessionsStore>>();
                        let current = sessions
                            .list()
                            .iter()
                            .find(|s| s.session_id == id)
                            .map(|s| s.enabled)
                            .unwrap_or(true);
                        sessions.set_enabled(id, !current);
                        let _ = app.emit("sessions:changed", ());
                    }
                    other if other.starts_with("session_settings_") => {
                        let id = other["session_settings_".len()..].to_string();
                        #[cfg(target_os = "macos")]
                        {
                            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
                        }
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.unminimize();
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                        let _ = app.emit("session:focus", id);
                    }
                    "open_settings" => {
                        show_settings_window(app);
                    }
                    "quit" => {
                        let tts = app.state::<Arc<TtsEngine>>();
                        tts.stop();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|_tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        // menu opens on left click via show_menu_on_left_click
                    }
                })
                .build(app)?;

            {
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
                let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    eprintln!("[claude-voice] global shortcut register failed: {e}");
                }
            }

            let state_for_server = AppState {
                tts: tts.clone(),
                settings: settings_store.clone(),
                sessions: sessions_store.clone(),
                bridge: bridge_queue.clone(),
                app: handle.clone(),
            };
            tauri::async_runtime::spawn(async move {
                if let Err(e) = server::serve(state_for_server, port).await {
                    eprintln!("[claude-voice] server error: {e}");
                }
            });

            Ok(())
        })
        .on_window_event(|win, event| {
            if win.label() == "popup" {
                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                        persist_popup_geometry(win);
                    }
                    _ => {}
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if win.label() == "settings" {
                    api.prevent_close();
                    let _ = win.hide();
                    #[cfg(target_os = "macos")]
                    {
                        let _ = win
                            .app_handle()
                            .set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn rebuild_sessions_submenu(
    submenu: &Submenu<tauri::Wry>,
    sessions: &[SessionInfo],
    handle: &tauri::AppHandle,
) -> tauri::Result<()> {
    let existing = submenu.items()?;
    for item in existing {
        use tauri::menu::MenuItemKind::*;
        match &item {
            MenuItem(i) => submenu.remove(i)?,
            Check(i) => submenu.remove(i)?,
            Submenu(i) => submenu.remove(i)?,
            Predefined(i) => submenu.remove(i)?,
            Icon(i) => submenu.remove(i)?,
        }
    }
    if sessions.is_empty() {
        let placeholder = MenuItem::with_id(
            handle,
            "sessions_none",
            "(no active sessions)",
            false,
            None::<&str>,
        )?;
        submenu.append(&placeholder)?;
        return Ok(());
    }
    for s in sessions {
        let enable_id = format!("session_enable_{}", s.session_id);
        let settings_id = format!("session_settings_{}", s.session_id);
        let enable_item =
            CheckMenuItem::with_id(handle, &enable_id, "Enabled", true, s.enabled, None::<&str>)?;
        let settings_item =
            MenuItem::with_id(handle, &settings_id, "Settings…", true, None::<&str>)?;
        let session_sub =
            Submenu::with_items(handle, &s.label, true, &[&enable_item, &settings_item])?;
        submenu.append(&session_sub)?;
    }
    Ok(())
}

fn persist_popup_geometry(win: &tauri::Window) {
    let scale = win.scale_factor().unwrap_or(1.0);
    let Ok(size) = win.inner_size() else { return };
    let Ok(pos) = win.outer_position() else {
        return;
    };
    let lw = size.width as f64 / scale;
    let lh = size.height as f64 / scale;
    // Skip the minimized orb state so we don't persist the 240x240 geometry.
    if lw < 320.0 || lh < 320.0 {
        return;
    }
    let store = win.app_handle().state::<Arc<SettingsStore>>();
    let mut cfg = store.get();
    cfg.popup_width = Some(lw);
    cfg.popup_height = Some(lh);
    cfg.popup_x = Some(pos.x as f64 / scale);
    cfg.popup_y = Some(pos.y as f64 / scale);
    let _ = store.update(cfg);
}

fn toggle_popup_visibility(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("popup") else {
        return;
    };
    let visible = win.is_visible().unwrap_or(false);
    if visible {
        let _ = win.hide();
    } else {
        let mut on_screen = false;
        if let (Ok(pos), Ok(size), Ok(scale)) =
            (win.outer_position(), win.outer_size(), win.scale_factor())
        {
            let x = pos.x as f64 / scale;
            let y = pos.y as f64 / scale;
            let w = size.width as f64 / scale;
            let h = size.height as f64 / scale;
            on_screen = position_on_any_monitor(&win, x, y, w, h);
        }
        if !on_screen {
            position_popup(&win);
        }
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn position_on_any_monitor(popup: &tauri::WebviewWindow, x: f64, y: f64, w: f64, h: f64) -> bool {
    let Ok(monitors) = popup.available_monitors() else {
        return false;
    };
    const VISIBLE_MARGIN: f64 = 80.0;
    for m in monitors {
        let scale = m.scale_factor();
        let pos = m.position();
        let size = m.size();
        let mx = pos.x as f64 / scale;
        let my = pos.y as f64 / scale;
        let mw = size.width as f64 / scale;
        let mh = size.height as f64 / scale;
        let overlap_x = (x + w).min(mx + mw) - x.max(mx);
        let overlap_y = (y + h).min(my + mh) - y.max(my);
        if overlap_x >= VISIBLE_MARGIN && overlap_y >= VISIBLE_MARGIN {
            return true;
        }
    }
    false
}

fn position_popup(popup: &tauri::WebviewWindow) {
    let monitor = match popup.primary_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let scale = monitor.scale_factor();
    let logical_w = monitor.size().width as f64 / scale;
    let x = logical_w - POPUP_WIDTH - POPUP_MARGIN_RIGHT;
    let y = POPUP_MARGIN_TOP;
    let _ = popup.set_size(LogicalSize::new(POPUP_WIDTH, POPUP_HEIGHT));
    let _ = popup.set_position(LogicalPosition::new(x, y));
}
