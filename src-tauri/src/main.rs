// リリース版ではコンソール窓を出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod announce;
mod backup;
mod commands;
mod model;
mod secrets;
mod store;
mod text;
mod twitch;

use commands::{apply_theme, emit_auth, AppState};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "開く", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("StreamStatus for Twitch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn main() {
    let store = store::Store::open(&store::data_dir());
    let state = AppState {
        store: Mutex::new(store),
        twitch: twitch::Twitch::new(secrets::load()),
        login: Mutex::new(None),
        last_channel: Mutex::new(None),
    };

    tauri::Builder::default()
        // 二重起動したら、起動済みのウィンドウを前に出す（最初に登録する決まり）
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| show_main(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(|app| {
            build_tray(app)?;
            let handle = app.handle().clone();

            if let Some(w) = app.get_webview_window("main") {
                let theme = handle.state::<AppState>().store.lock().unwrap().data.settings.theme.clone();
                apply_theme(&w, &theme);
            }

            // 画面の準備が遅れても、3秒たったら表示する
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(3)).await;
                if let Some(w) = h.get_webview_window("main") {
                    if !w.is_visible().unwrap_or(true) {
                        let _ = w.show();
                    }
                }
            });

            // トークンを起動時と1時間ごとに検証する（Twitch の決まり）
            tauri::async_runtime::spawn(async move {
                loop {
                    let st = handle.state::<AppState>();
                    if st.twitch.user().await.is_some() {
                        if let Err(e) = st.twitch.validate().await {
                            if e.code == "auth_expired" || e.code == "not_signed_in" {
                                emit_auth(&handle, "signed_out", None, Some(e.message));
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(60 * 60)).await;
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let to_tray = window.state::<AppState>().store.lock().unwrap().data.settings.close_to_tray;
                if to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::ui_ready,
            commands::login_start,
            commands::login_cancel,
            commands::open_activation,
            commands::logout,
            commands::fetch_channel,
            commands::search_categories,
            commands::apply_preset,
            commands::save_preset,
            commands::delete_preset,
            commands::duplicate_preset,
            commands::reorder_presets,
            commands::save_settings,
            commands::announce_text,
            commands::open_x_intent,
            commands::export_presets,
            commands::import_presets,
        ])
        .run(tauri::generate_context!())
        .expect("StreamStatus を起動できませんでした");
}
