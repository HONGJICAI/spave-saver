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
            broken_file_check,
            fix_file_extensions,
            delete_files,
            get_storage_stats,
            get_compression_plugins,
            set_plugin_quality,
            scan_compressible_files,
            compress_files_in_place,
            get_skip_cache_info,
            clear_skip_cache,
            get_config,
            set_config,
            detect_tools
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
