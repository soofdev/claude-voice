mod server;
mod settings;
mod text_clean;
mod transcript;
mod tts;

use std::sync::Arc;

use server::AppState;
use settings::{Settings, SettingsStore};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tts::TtsEngine;

struct MenuRefs {
    enabled: CheckMenuItem<tauri::Wry>,
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
) -> Result<(), String> {
    store.update(new.clone()).map_err(|e| e.to_string())?;
    let _ = menu_refs.enabled.set_checked(new.enabled);
    Ok(())
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
    let cleaned = text_clean::clean_for_speech(&text);
    tts.speak(&cleaned, &cfg.voice, cfg.rate);
}

#[tauri::command]
fn stop_speaking(tts: tauri::State<Arc<TtsEngine>>) {
    tts.stop();
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
    let tts = Arc::new(TtsEngine::new());

    let initial = settings_store.get();
    let port = initial.port;

    let state_for_server = AppState {
        tts: tts.clone(),
        settings: settings_store.clone(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(settings_store.clone())
        .manage(tts.clone())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            list_voices,
            test_speak,
            stop_speaking,
            hook_command,
        ])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();

            let enabled_item = CheckMenuItem::with_id(
                &handle,
                "toggle_enabled",
                "Speaking Enabled",
                true,
                initial.enabled,
                None::<&str>,
            )?;
            let stop_item =
                MenuItem::with_id(&handle, "stop_speaking", "Stop Speaking", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(&handle, "open_settings", "Settings…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(&handle, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                &handle,
                &[
                    &enabled_item,
                    &stop_item,
                    &PredefinedMenuItem::separator(&handle)?,
                    &settings_item,
                    &PredefinedMenuItem::separator(&handle)?,
                    &quit_item,
                ],
            )?;

            let menu_refs = Arc::new(MenuRefs {
                enabled: enabled_item.clone(),
            });
            app.manage(menu_refs);

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
                    "open_settings" => {
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        } else {
                            let _ = WebviewWindowBuilder::new(
                                app,
                                "settings",
                                WebviewUrl::App("index.html".into()),
                            )
                            .title("Claude Voice Settings")
                            .inner_size(460.0, 520.0)
                            .resizable(false)
                            .build();
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
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
