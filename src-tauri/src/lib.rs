mod commands;
mod engine;
mod error;
mod merge;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::export::open_repo,
            commands::export::get_unpushed_commits,
            commands::export::get_last_sync,
            commands::export::preview_export,
            commands::export::exec_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitSneaker");
}
