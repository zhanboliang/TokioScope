mod commands;
mod errors;
mod events;
mod examples;
mod harness;
mod parser;
mod prefs;
mod rewriter;
mod runner;

use std::sync::Arc;
use tauri::Manager;

pub struct AppState {
    pub runner: Arc<runner::Runner>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let runner = Arc::new(runner::Runner::new(app.handle().clone()));
            app.manage(AppState { runner });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_examples,
            commands::analyze_code,
            commands::start_run,
            commands::cancel_run,
            commands::ensure_runner,
            commands::runner_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokioScope");
}
