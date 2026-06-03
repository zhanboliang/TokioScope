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
    /// Snippet / single-file source. Ignored when `project_root` is set.
    pub source: Option<String>,
    pub runtime: Option<RuntimeConfig>,
    /// When present, trace/run the whole Cargo project at this path instead.
    pub project_root: Option<String>,
}

#[tauri::command]
pub async fn start_run(args: StartArgs, state: State<'_, AppState>) -> TsResult<()> {
    if let Some(root) = args.project_root {
        return state.runner.start_project(PathBuf::from(root)).await;
    }
    let source = args.source.unwrap_or_default();
    let runtime = args.runtime.unwrap_or_else(|| {
        // Re-derive from the source so user changes to `#[tokio::main]` flow through.
        parser::analyze(&source).runtime
    });
    state.runner.start(source, runtime).await
}

#[tauri::command]
pub async fn cancel_run(state: State<'_, AppState>) -> TsResult<()> {
    state.runner.cancel().await;
    Ok(())
}

#[tauri::command]
pub async fn ensure_runner(state: State<'_, AppState>) -> TsResult<()> {
    state.runner.ensure().await
}

#[tauri::command]
pub async fn runner_status(state: State<'_, AppState>) -> TsResult<RunnerStatus> {
    Ok(state.runner.status().await)
}

pub(crate) fn resolve_template_dir(app: &tauri::AppHandle) -> PathBuf {
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
