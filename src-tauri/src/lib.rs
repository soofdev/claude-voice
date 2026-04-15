mod history;
mod link_extract;
mod server;
mod sessions;
mod settings;
mod text_clean;
mod transcript;
mod tts;

use std::sync::Arc;

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

const POPUP_WIDTH: f64 = 380.0;
const POPUP_HEIGHT: f64 = 560.0;
const POPUP_MIN_HEIGHT: f64 = 240.0;
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
    let entry = history.get(&id).ok_or_else(|| "entry not found".to_string())?;
    let cfg = settings.get();
    tts.replay(entry, cfg);
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
fn send_to_terminal(text: String) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("empty prompt".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        terminal_send::send(&text)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
        Err("send-to-terminal is only implemented on macOS".to_string())
    }
}

#[cfg(target_os = "macos")]
mod terminal_send {
    use std::process::Command;

    pub fn send(text: &str) -> Result<String, String> {
        if let Some(target) = try_iterm2(text) {
            return target;
        }
        try_terminal_app(text)
    }

    fn try_iterm2(text: &str) -> Option<Result<String, String>> {
        if !is_app_running("iTerm2") {
            return None;
        }
        let escaped = applescript_escape(text);
        let script = format!(
            r#"tell application "iTerm2" to tell current session of current window to write text "{}""#,
            escaped
        );
        Some(run_osascript(&script).map(|_| "iTerm2".to_string()))
    }

    fn try_terminal_app(text: &str) -> Result<String, String> {
        if !is_app_running("Terminal") {
            return Err("No supported terminal is running (iTerm2 or Terminal.app)".to_string());
        }
        let escaped = applescript_escape(text);
        let script = format!(
            r#"
tell application "Terminal" to activate
delay 0.12
tell application "System Events"
    keystroke "{}"
    key code 36
end tell
"#,
            escaped
        );
        run_osascript(&script).map(|_| "Terminal.app".to_string())
    }

    fn is_app_running(name: &str) -> bool {
        let script = format!(
            r#"tell application "System Events" to (name of processes) contains "{}""#,
            name
        );
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
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

#[tauri::command]
fn hook_command(store: tauri::State<Arc<SettingsStore>>) -> String {
    let port = store.get().port;
    format!(
        "curl -s -X POST http://127.0.0.1:{port}/hook/stop -H 'Content-Type: application/json' --data-binary @-"
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings_store = Arc::new(SettingsStore::new());
    let history_store = Arc::new(HistoryStore::new());
    let sessions_store = Arc::new(SessionsStore::new());
    let tts = Arc::new(TtsEngine::new());
    tts.set_history(history_store.clone());

    let initial = settings_store.get();
    let port = initial.port;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(settings_store.clone())
        .manage(history_store.clone())
        .manage(sessions_store.clone())
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
            clear_history,
            send_to_terminal,
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

            let popup = WebviewWindowBuilder::new(
                app,
                "popup",
                WebviewUrl::App("popup.html".into()),
            )
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .resizable(true)
            .focused(false)
            .skip_taskbar(true)
            .visible(false)
            .inner_size(POPUP_WIDTH, POPUP_HEIGHT)
            .min_inner_size(POPUP_WIDTH, POPUP_MIN_HEIGHT)
            .shadow(false)
            .accept_first_mouse(true)
            .build()?;

            position_popup(&popup);
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

            // popup owns its own hide logic (respects pin)

            let settings_window = WebviewWindowBuilder::new(
                app,
                "settings",
                WebviewUrl::App("index.html".into()),
            )
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
            let stop_item =
                MenuItem::with_id(&handle, "stop_speaking", "Stop Speaking", true, None::<&str>)?;
            let pin_item = CheckMenuItem::with_id(
                &handle,
                "toggle_pin_popup",
                "Pin Popup",
                true,
                initial.pin_popup,
                None::<&str>,
            )?;
            let sessions_submenu = Submenu::with_id_and_items(
                &handle,
                "sessions_submenu",
                "Sessions",
                true,
                &[],
            )?;
            rebuild_sessions_submenu(
                &sessions_submenu,
                &sessions_store.list(),
                &handle,
            )?;
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
                if let Err(e) =
                    rebuild_sessions_submenu(&submenu_listen, &list, &handle_for_listen)
                {
                    eprintln!("[claude-voice] sessions submenu rebuild failed: {e}");
                }
            });

            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("no default icon")?;

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
                        #[cfg(target_os = "macos")]
                        {
                            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
                        }
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.unminimize();
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
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

            let state_for_server = AppState {
                tts: tts.clone(),
                settings: settings_store.clone(),
                sessions: sessions_store.clone(),
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
        let enable_item = CheckMenuItem::with_id(
            handle,
            &enable_id,
            "Enabled",
            true,
            s.enabled,
            None::<&str>,
        )?;
        let settings_item =
            MenuItem::with_id(handle, &settings_id, "Settings…", true, None::<&str>)?;
        let session_sub = Submenu::with_items(handle, &s.label, true, &[
            &enable_item,
            &settings_item,
        ])?;
        submenu.append(&session_sub)?;
    }
    Ok(())
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
