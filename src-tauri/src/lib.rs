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
            let window = app.get_webview_window("main").unwrap();

            // 设置窗口图标
            let icon_bytes = include_bytes!("../icons/128x128.png");
            let img = image::load_from_memory(icon_bytes)
                .expect("failed to load icon")
                .to_rgba8();
            let (width, height) = img.dimensions();
            let icon = tauri::image::Image::new_owned(img.into_raw(), width, height);
            window.set_icon(icon).expect("failed to set icon");

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::export::open_repo,
            commands::export::list_commits,
            commands::export::get_unpushed_commits,
            commands::export::get_last_sync,
            commands::export::preview_export,
            commands::export::exec_export,
            commands::import::verify_bundle,
            commands::import::exec_import,
            commands::merge_cmd::get_conflicts,
            commands::merge_cmd::auto_resolve_conflicts,
            commands::merge_cmd::apply_resolution,
            commands::merge_cmd::commit_merge,
            commands::merge_cmd::abort_merge,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitSneaker");
}
