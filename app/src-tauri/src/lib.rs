mod commands;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger
    space_saver_utils::init_logger();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan,
            empty_folder_check,
            duplicate_file_check,
            similar_file_check,
            delete_files,
            get_storage_stats,
            get_compression_plugins,
            scan_compressible_files,
            compress_files_in_place
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
