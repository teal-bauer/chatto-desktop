use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_store::StoreExt;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use serde_json::json;

#[tauri::command]
fn set_server_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("server_url", json!(url));
    store.save().map_err(|e| e.to_string())?;

    let window = app.get_webview_window("main").ok_or("no main window")?;
    let parsed: tauri::Url = url.parse().map_err(|e| format!("invalid URL: {e}"))?;
    window.navigate(parsed).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_server_url(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    Ok(store
        .get("server_url")
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

fn toggle_window_visibility(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide = MenuItem::with_id(app, "show_hide", "Show/Hide", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Chatto", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_hide, &settings, &quit])?;

    TrayIconBuilder::new()
        .tooltip("Chatto")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_hide" => toggle_window_visibility(app),
            "settings" => {
                let _ = app.emit("open-settings", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window_visibility(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn navigate_to_server(app: &tauri::AppHandle) -> bool {
    let store = match app.store("config.json") {
        Ok(s) => s,
        Err(_) => return false,
    };

    if let Some(url_str) = store.get("server_url").and_then(|v| v.as_str().map(String::from)) {
        if let Ok(url) = url_str.parse::<tauri::Url>() {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.navigate(url);
                return true;
            }
        }
    }

    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![set_server_url, get_server_url])
        .setup(|app| {
            // Autostart
            #[cfg(desktop)]
            {
                use tauri_plugin_autostart::MacosLauncher;
                app.handle().plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    None,
                ))?;
            }

            // Window state persistence
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;

            // Deep links
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            app.deep_link().register_all()?;

            if let Some(urls) = app.deep_link().get_current()? {
                eprintln!("launched via deep link: {:?}", urls);
            }

            let app_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let urls = event.urls();
                eprintln!("deep link opened: {:?}", urls);
                // Navigate to the first URL's path on the configured server
                if let Some(url) = urls.first() {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.navigate(url.clone());
                    }
                }
            });

            // System tray
            setup_tray(app)?;

            // Navigate to configured server or show settings
            let handle = app.handle().clone();
            if !navigate_to_server(&handle) {
                // No server configured — frontend will show settings UI
                let _ = handle.emit("open-settings", ());
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide to tray on close instead of quitting
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
