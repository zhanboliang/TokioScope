//! Filesystem + project-scaffolding commands for the IDE shell. These run in
//! the Rust backend with full `std::fs` access, so no `tauri-plugin-fs` scope
//! configuration is needed — the user authorises paths by picking/dropping them.

use crate::errors::{TsError, TsResult};
use crate::tools;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Refuse to load anything larger than this into the editor (avoids choking on
/// a stray multi-MB build artifact the user clicks).
const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Serialize)]
pub struct FileRead {
    pub text: String,
    pub too_large: bool,
    pub binary: bool,
    pub byte_len: u64,
}

#[derive(Serialize)]
pub struct WorkspaceInfo {
    pub root: String,
    pub name: String,
    pub is_cargo: bool,
}

/// One directory level (lazy — the tree expands on demand). Dirs first, then
/// case-insensitive name.
#[tauri::command]
pub fn read_dir(path: String) -> TsResult<Vec<DirEntry>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&path)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        out.push(DirEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: ft.is_dir(),
            is_symlink: ft.is_symlink(),
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(out)
}

#[tauri::command]
pub fn read_text_file(path: String) -> TsResult<FileRead> {
    let meta = std::fs::metadata(&path)?;
    let byte_len = meta.len();
    if byte_len > MAX_TEXT_BYTES {
        return Ok(FileRead { text: String::new(), too_large: true, binary: false, byte_len });
    }
    let bytes = std::fs::read(&path)?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(FileRead { text, too_large: false, binary: false, byte_len }),
        Err(_) => Ok(FileRead { text: String::new(), too_large: false, binary: true, byte_len }),
    }
}

#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> TsResult<()> {
    std::fs::write(&path, contents)?;
    Ok(())
}

/// Describe a folder the user opened: its display name + whether it's a Cargo
/// crate (has a `Cargo.toml`).
#[tauri::command]
pub fn workspace_info(path: String) -> TsResult<WorkspaceInfo> {
    let p = PathBuf::from(&path);
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    Ok(WorkspaceInfo {
        root: path,
        name,
        is_cargo: p.join("Cargo.toml").is_file(),
    })
}

const STARTER_MAIN: &str = r#"#[tokio::main]
async fn main() {
    println!("hello from TokioScope");

    let worker = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        println!("spawned task finished");
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let _ = worker.await;
}
"#;

/// Create a new Tokio binary project. Prefers `cargo new`; falls back to a
/// manual scaffold if cargo is unavailable. Returns the new project root.
#[tauri::command]
pub async fn create_project(parent: String, name: String) -> TsResult<String> {
    let root = PathBuf::from(&parent).join(&name);
    if root.exists() {
        return Err(TsError::Other(format!("'{name}' already exists in that folder")));
    }

    let cargo_ok = tools::cargo_command()
        .args(["new", "--bin", "--name", &name])
        .arg(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if cargo_ok {
        write_tokio_starter(&root)?;
    } else {
        scaffold_manually(&root, &name)?;
    }
    Ok(root.to_string_lossy().to_string())
}

/// Overwrite a freshly `cargo new`'d crate with a Tokio starter + dependency.
fn write_tokio_starter(root: &Path) -> TsResult<()> {
    std::fs::write(root.join("src").join("main.rs"), STARTER_MAIN)?;
    let cargo_path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&cargo_path)?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| TsError::Parse(e.to_string()))?;
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    doc["dependencies"]["tokio"]["version"] = toml_edit::value("1");
    let mut features = toml_edit::Array::new();
    features.push("full");
    doc["dependencies"]["tokio"]["features"] = toml_edit::value(features);
    std::fs::write(&cargo_path, doc.to_string())?;
    Ok(())
}

/// Manual scaffold when `cargo new` isn't available.
fn scaffold_manually(root: &Path, name: &str) -> TsResult<()> {
    std::fs::create_dir_all(root.join("src"))?;
    let cargo = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ntokio = {{ version = \"1\", features = [\"full\"] }}\n"
    );
    std::fs::write(root.join("Cargo.toml"), cargo)?;
    std::fs::write(root.join("src").join("main.rs"), STARTER_MAIN)?;
    std::fs::write(root.join(".gitignore"), "/target\n")?;
    Ok(())
}
