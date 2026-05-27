use crate::errors::TsResult;
use crate::examples::{self, Example};
use crate::parser::{self, ParseReport, RuntimeConfig};
use crate::runner::RunnerStatus;
use crate::AppState;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{Manager, State};

#[tauri::command]
pub fn list_examples() -> Vec<Example> {
    examples::list()
}

#[tauri::command]
pub fn analyze_code(source: String) -> ParseReport {
    parser::analyze(&source)
}

#[derive(Deserialize)]
pub struct StartArgs {
    pub source: String,
    pub runtime: Option<RuntimeConfig>,
}

#[tauri::command]
pub async fn start_run(args: StartArgs, state: State<'_, AppState>) -> TsResult<()> {
    let runtime = args.runtime.unwrap_or_else(|| {
        // Re-derive from the source so user changes to `#[tokio::main]` flow through.
        parser::analyze(&args.source).runtime
    });
    state.runner.start(args.source, runtime).await
}

#[tauri::command]
pub async fn cancel_run(state: State<'_, AppState>) -> TsResult<()> {
    state.runner.cancel().await;
    Ok(())
}

#[tauri::command]
pub async fn ensure_runner(app: tauri::AppHandle, state: State<'_, AppState>) -> TsResult<()> {
    let template_dir = resolve_template_dir(&app);
    state.runner.ensure(template_dir).await
}

#[tauri::command]
pub async fn runner_status(state: State<'_, AppState>) -> TsResult<RunnerStatus> {
    Ok(state.runner.status().await)
}

fn resolve_template_dir(app: &tauri::AppHandle) -> PathBuf {
    // In dev, the template lives at <cwd>/../runner-template. In bundled apps,
    // we ship it under the resource dir.
    if let Ok(resource) = app.path().resource_dir() {
        let cand = resource.join("runner-template");
        if cand.exists() {
            return cand;
        }
    }
    let mut cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.push("..");
    cwd.push("runner-template");
    cwd
}
