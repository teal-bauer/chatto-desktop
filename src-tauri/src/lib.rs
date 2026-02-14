use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_store::StoreExt;

#[cfg(desktop)]
use tauri::{
    menu::{AboutMetadataBuilder, CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri::{WebviewUrl, WebviewWindowBuilder};

use serde_json::json;

const NOTIFICATION_BRIDGE_JS: &str = r#"
(function() {
    if (window.__chattoNotificationBridged) return;
    window.__chattoNotificationBridged = true;

    // Bridge Notification API to native notifications
    const OrigNotification = window.Notification;
    window.Notification = function(title, options) {
        if (window.__TAURI_INTERNALS__) {
            window.__TAURI_INTERNALS__.invoke('show_notification', {
                title: title,
                body: (options && options.body) || ''
            }).catch(function() {});
        }
        try { return new OrigNotification(title, options); } catch(e) {}
    };
    window.Notification.permission = 'granted';
    window.Notification.requestPermission = function() {
        return Promise.resolve('granted');
    };


})();
"#;

// Template icon for macOS menu bar (black on transparent, used as template image)
#[cfg(desktop)]
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray-icon.png");

#[tauri::command]
fn set_server_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed: tauri::Url = url.parse().map_err(|e| format!("Invalid URL: {e}"))?;

    // Validate the server is reachable before saving
    match ureq::head(parsed.as_str())
        .timeout(std::time::Duration::from_secs(10))
        .call()
    {
        Ok(_) => {}
        Err(ureq::Error::Status(_, _)) => {
            // Any HTTP response means the server is reachable
        }
        Err(ureq::Error::Transport(e)) => {
            let reason = match e.kind() {
                ureq::ErrorKind::Dns => "Server not found — check the address",
                ureq::ErrorKind::ConnectionFailed => "Could not connect to server",
                ureq::ErrorKind::Io => "Connection error",
                _ => "Server unreachable",
            };
            return Err(format!("{reason} ({e})"));
        }
    }

    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("server_url", json!(url));
    store.save().map_err(|e| e.to_string())?;

    let window = app.get_webview_window("main").ok_or("no main window")?;
    window.navigate(parsed).map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn clear_server_url(app: tauri::AppHandle) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.delete("server_url");
    store.save().map_err(|e| e.to_string())?;

    let window = app.get_webview_window("main").ok_or("no main window")?;
    window.navigate(frontend_url("/")).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_server_url(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    Ok(store
        .get("server_url")
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

#[tauri::command]
fn show_notification(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    // Check if notifications are enabled
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    let enabled = store
        .get("notifications_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !enabled {
        return Ok(());
    }

    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_notifications_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    Ok(store
        .get("notifications_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}

#[tauri::command]
fn set_notifications_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("notifications_enabled", json!(enabled));
    store.save().map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    Ok(app.autolaunch().is_enabled().unwrap_or(false))
}

#[cfg(desktop)]
#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())
    } else {
        autolaunch.disable().map_err(|e| e.to_string())
    }
}

#[cfg(desktop)]
fn frontend_url(path: &str) -> tauri::Url {
    #[cfg(debug_assertions)]
    let base = "http://localhost:1420";
    #[cfg(not(debug_assertions))]
    let base = "tauri://localhost";
    format!("{base}{path}").parse().unwrap()
}

#[cfg(desktop)]
fn navigate_to_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.navigate(frontend_url("/?settings"));
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(desktop)]
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

#[cfg(desktop)]
fn setup_app_menu(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let about_metadata = AboutMetadataBuilder::new()
        .version(Some(env!("GIT_VERSION")))
        .website(Some("https://github.com/teal-bauer/chatto-desktop"))
        .website_label(Some("GitHub"))
        .license(Some("AGPL-3.0"))
        .build();

    let app_submenu = Submenu::with_items(
        app,
        "Chatto",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("About Chatto"), Some(about_metadata))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "menu_settings", "Settings…", true, Some("CmdOrCtrl+,"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let edit_submenu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let view_submenu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &MenuItem::with_id(app, "menu_back", "Back", true, Some("CmdOrCtrl+["))?,
            &MenuItem::with_id(app, "menu_forward", "Forward", true, Some("CmdOrCtrl+]"))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "menu_reload", "Reload", true, Some("CmdOrCtrl+R"))?,
        ],
    )?;

    let window_submenu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let menu = Menu::with_items(
        app,
        &[&app_submenu, &edit_submenu, &view_submenu, &window_submenu],
    )?;

    menu.set_as_app_menu()?;

    // Handle custom menu events
    app.on_menu_event(move |app, event| match event.id().as_ref() {
        "menu_settings" => {
            navigate_to_settings(app);
        }
        "menu_back" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("history.back()");
            }
        }
        "menu_forward" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("history.forward()");
            }
        }
        "menu_reload" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("location.reload()");
            }
        }
        _ => {}
    });

    Ok(())
}

#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide = MenuItem::with_id(app, "show_hide", "Show/Hide", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let autostart_enabled = {
        use tauri_plugin_autostart::ManagerExt;
        app.autolaunch().is_enabled().unwrap_or(false)
    };
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start at Login",
        true,
        autostart_enabled,
        None::<&str>,
    )?;

    let quit = MenuItem::with_id(app, "quit", "Quit Chatto", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show_hide, &settings, &separator, &autostart, &separator, &quit],
    )?;

    let icon = tauri::image::Image::from_bytes(TRAY_ICON_BYTES)?;

    TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Chatto")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_hide" => toggle_window_visibility(app),
            "settings" => {
                navigate_to_settings(app);
            }
            "autostart" => {
                use tauri_plugin_autostart::ManagerExt;
                let autolaunch = app.autolaunch();
                let enabled = autolaunch.is_enabled().unwrap_or(false);
                if enabled {
                    let _ = autolaunch.disable();
                } else {
                    let _ = autolaunch.enable();
                }
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

fn get_server_url_from_store(app: &tauri::AppHandle) -> Option<String> {
    app.store("config.json")
        .ok()
        .and_then(|store| store.get("server_url").and_then(|v| v.as_str().map(String::from)))
}

fn create_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let url = get_server_url_from_store(app.handle());

    let webview_url = match &url {
        Some(u) => WebviewUrl::External(u.parse()?),
        None => WebviewUrl::default(),
    };

    let builder = WebviewWindowBuilder::new(app, "main", webview_url)
        .initialization_script(NOTIFICATION_BRIDGE_JS);

    #[cfg(desktop)]
    let builder = builder
        .title("Chatto")
        .inner_size(1024.0, 768.0)
        .min_inner_size(400.0, 300.0)
        .disable_drag_drop_handler()
        .on_document_title_changed(|window, title| {
            let _ = window.set_title(&title);
        });

    let _window = builder.build()?;

    if url.is_none() {
        let _ = app.handle().emit("open-settings", ());
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        set_server_url,
        get_server_url,
        clear_server_url,
        show_notification,
        get_notifications_enabled,
        set_notifications_enabled,
        get_autostart_enabled,
        set_autostart_enabled,
    ]);
    #[cfg(mobile)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        set_server_url,
        get_server_url,
        show_notification,
        get_notifications_enabled,
        set_notifications_enabled,
    ]);

    let builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
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
                if let Some(url) = urls.first() {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.navigate(url.clone());
                    }
                }
            });

            // macOS menu bar
            #[cfg(desktop)]
            setup_app_menu(app)?;

            // System tray
            #[cfg(desktop)]
            setup_tray(app)?;

            // Create main window
            create_main_window(app)?;

            Ok(())
        });

    // Close-to-tray on desktop only
    #[cfg(desktop)]
    let builder = builder.on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            let _ = window.hide();
            api.prevent_close();
        }
    });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
