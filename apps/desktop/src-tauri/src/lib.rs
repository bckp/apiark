mod commands;
pub use apiark_core::{exporter, http, importer, models, scripting, storage};
mod oauth;
mod runner;
mod watcher;
use commands::backup::{export_app_state, import_app_state};
use commands::collection::{
    create_collection, create_folder, create_request, delete_item, get_collection_defaults,
    open_collection, read_request_file, rename_item, save_folder_order, save_request_file,
    update_collection_defaults,
};
use commands::cookies::{clear_cookie_jar, delete_cookie, get_cookie_jar};
use commands::curl::{export_curl_command, parse_curl_command};
use commands::environment::{
    get_resolved_variables, get_variable_names, load_environments, load_root_dotenv,
    save_environment,
};
use commands::history::{clear_history, delete_history_entry, get_history, search_history};
use commands::http::{send_request, send_request_with_scripts};
use commands::import_export::{
    detect_import_format, export_collection, import_collection, import_environment, import_preview,
};
use commands::oauth::{oauth_clear_token, oauth_get_token_status, oauth_start_flow};
use commands::runner::run_collection_command;
use commands::settings::{get_settings, update_settings};
use commands::state::{load_persisted_state, save_persisted_state};
use commands::trash::{empty_trash, list_trash, restore_from_trash};
use commands::watcher::{unwatch_collection, watch_collection};
use commands::window::open_new_window;
use commands::{history::AppState, settings::SettingsState};
use http::cookies::CookieJarManager;
use oauth::OAuthTokenStore;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use watcher::collection_watcher::CollectionWatcher;
pub fn run() {
    let data = dirs::home_dir()
        .expect("Cannot locate home directory")
        .join(".apiark-internal");
    std::fs::create_dir_all(&data).expect("Cannot create internal fork data directory");
    let log = tracing_appender::rolling::daily(data.join("logs"), "apiark.log");
    let (writer, _guard) = tracing_appender::non_blocking(log);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "apiark=info".into()),
        )
        .init();
    let history = Arc::new(
        storage::history::HistoryDb::open(&data.join("data.db"))
            .expect("Cannot open history database"),
    );
    let settings_path = data.join("settings.json");
    let settings = storage::settings::load_settings(&settings_path);
    let persisted = storage::state::load_persisted_state(&data.join("state.json"));
    let maximize = persisted.window_state.as_ref().is_some_and(|w| w.maximized);
    tauri::Builder::default()
        .manage(AppState {
            history_db: history,
        })
        .manage(SettingsState {
            settings: Mutex::new(settings),
            settings_path,
        })
        .manage(CookieJarManager::new())
        .manage(OAuthTokenStore::new())
        .manage(CollectionWatcher::new())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            if maximize {
                if let Some(window) = app.get_webview_window("main") {
                    window.maximize()?;
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            export_app_state,
            import_app_state,
            get_collection_defaults,
            update_collection_defaults,
            open_collection,
            read_request_file,
            save_request_file,
            create_request,
            create_folder,
            delete_item,
            save_folder_order,
            rename_item,
            create_collection,
            get_cookie_jar,
            delete_cookie,
            clear_cookie_jar,
            parse_curl_command,
            export_curl_command,
            load_root_dotenv,
            load_environments,
            save_environment,
            get_resolved_variables,
            get_variable_names,
            get_history,
            search_history,
            clear_history,
            delete_history_entry,
            send_request,
            send_request_with_scripts,
            detect_import_format,
            import_preview,
            import_collection,
            export_collection,
            import_environment,
            oauth_start_flow,
            oauth_get_token_status,
            oauth_clear_token,
            run_collection_command,
            get_settings,
            update_settings,
            load_persisted_state,
            save_persisted_state,
            list_trash,
            restore_from_trash,
            empty_trash,
            watch_collection,
            unwatch_collection,
            open_new_window,
        ])
        .run(tauri::generate_context!())
        .expect("Cannot run ApiArk");
}
