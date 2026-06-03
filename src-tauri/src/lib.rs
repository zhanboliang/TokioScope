mod commands;
mod errors;
mod events;
mod examples;
mod harness;
mod parser;
mod prefs;
mod project;
mod pty;
mod rewriter;
mod runner;
mod tools;
mod workspace;

use std::sync::Arc;
use tauri::Manager;

pub struct AppState {
    pub runner: Arc<runner::Runner>,
    pub ptys: Arc<pty::PtyRegistry>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let template_dir = commands::resolve_template_dir(app.handle());
            let runner = Arc::new(runner::Runner::new(app.handle().clone(), template_dir));
            app.manage(AppState {
                runner,
                ptys: Arc::new(pty::PtyRegistry::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_examples,
            commands::analyze_code,
            commands::start_run,
            commands::cancel_run,
            commands::ensure_runner,
            commands::runner_status,
            workspace::read_dir,
            workspace::read_text_file,
            workspace::write_text_file,
            workspace::workspace_info,
            workspace::create_project,
            pty::pty_spawn,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_kill,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokioScope");
}
