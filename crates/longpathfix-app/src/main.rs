#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;
use tauri::Manager;
use windows_sys::Win32::System::LibraryLoader::{
    SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_SYSTEM32,
};

fn harden_dll_search_order() {
    // SAFETY: Restricts implicit DLL lookups to System32 to prevent DLL search-order hijacking.
    unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
    }
}

fn main() {
    harden_dll_search_order();

    // Elevated one-shot helper path spawned exclusively via UAC ("runas") when the user
    // clicks "Enable Windows Long Paths" in Diagnostics.
    if std::env::args().any(|a| a == "--enable-long-paths") {
        let exit = match longpathfix_core::windows::enable_long_paths_in_registry() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        };
        std::process::exit(exit);
    }

    let app_state = AppState::initialize();
    let initial_theme = app_state.config.lock().map(|c| c.theme).unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .setup(move |app| {
            commands::apply_window_theme(app.handle(), initial_theme);
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::set_config,
            commands::inspect_root,
            commands::start_scan,
            commands::cancel_operation,
            commands::update_plan_options,
            commands::query_findings,
            commands::inspect_path,
            commands::validate_custom_name,
            commands::analyze_path_input,
            commands::preview_repair,
            commands::execute_repair,
            commands::get_history,
            commands::get_history_batch_detail,
            commands::undo_batch,
            commands::delete_history_batch,
            commands::export_findings,
            commands::export_outcomes,
            commands::get_system_diagnostics,
            commands::enable_system_long_paths,
            commands::open_in_explorer,
            commands::open_data_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LongPathFix");
}
