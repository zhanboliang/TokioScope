//! Project-trace injection. Builds a traced *shadow copy* of an opened Cargo
//! project in the cache dir: every `src/**/*.rs` is instrumented (best-effort,
//! per-file verbatim fallback on parse failure), the `#[tokio::main]` entry is
//! swapped for `tracer::bootstrap`, and `Cargo.toml` is patched to enable the
//! `test-util` feature the deterministic tracer needs.
//!
//! This is intentionally best-effort: real I/O / external-dep async on a paused
//! runtime won't advance the virtual clock, and exotic projects (cfg-gated code,
//! proc-macros, lib+bin sharing state) may not trace meaningfully — see the
//! warning surfaced in the UI. Compilation safety is the priority: anything we
//! can't rewrite is left verbatim so the shadow still builds.

use crate::errors::{TsError, TsResult};
use crate::parser::{self, Flavor, RuntimeConfig};
use crate::rewriter;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use toml_edit::{value, Array, DocumentMut, InlineTable, Item, Table, Value};

pub struct PreparedProject {
    /// The shadow directory to run `cargo` in.
    pub root: PathBuf,
    pub runtime: RuntimeConfig,
}

/// Refresh the traced shadow of `project_root` and return where to run it.
/// `template_dir` is the bundled runner-template (provides `tracer.rs`).
pub fn prepare(project_root: &Path, template_dir: &Path) -> TsResult<PreparedProject> {
    let project_root = project_root
        .canonicalize()
        .map_err(|e| TsError::Other(format!("project path: {e}")))?;

    // Step A — detect bin entry (v0.2: default src/main.rs).
    let entry_rel = find_entry_rel(&project_root)?;
    let entry_src = std::fs::read_to_string(project_root.join(&entry_rel))
        .map_err(|e| TsError::Other(format!("read entry: {e}")))?;
    let entry = rewriter::prepare_entry(&entry_src);
    if !entry.found_main {
        return Err(TsError::NotReady(
            "未找到 #[tokio::main] async fn main —— 项目追踪需要 Tokio 二进制入口".into(),
        ));
    }
    let runtime = parser::analyze(&entry_src).runtime;

    // Step B — shadow copy (keeps the shadow's own target/ for incremental builds).
    let shadow = shadow_dir(&project_root);
    std::fs::create_dir_all(&shadow)?;
    sync_sources(&project_root, &shadow)?;

    // Step C — inject the tracer module beside the entry's crate root.
    let tracer_src = std::fs::read_to_string(template_dir.join("src").join("tracer.rs"))
        .map_err(|e| TsError::Other(format!("read tracer template: {e}")))?;
    std::fs::write(shadow.join("src").join("__ts_tracer.rs"), &tracer_src)?;

    // Step D/E — instrument the tree + swap the entry.
    let has_lib = shadow.join("src").join("lib.rs").is_file();
    instrument_tree(&shadow, &entry_rel, &entry, &runtime, has_lib)?;

    // Step F — patch the manifest.
    patch_manifest(&shadow)?;

    Ok(PreparedProject { root: shadow, runtime })
}

fn find_entry_rel(root: &Path) -> TsResult<PathBuf> {
    let cargo = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&cargo)
        .map_err(|e| TsError::Other(format!("read Cargo.toml: {e}")))?;
    let doc = text
        .parse::<DocumentMut>()
        .map_err(|e| TsError::Parse(e.to_string()))?;
    if doc.get("package").is_none() && doc.get("workspace").is_some() {
        return Err(TsError::NotReady(
            "这是一个 Cargo workspace —— 请打开具体的成员 crate".into(),
        ));
    }
    let main_rs = PathBuf::from("src").join("main.rs");
    if root.join(&main_rs).is_file() {
        Ok(main_rs)
    } else {
        Err(TsError::NotReady(
            "未找到 src/main.rs 入口(v0.2 项目追踪需要默认二进制入口)".into(),
        ))
    }
}

fn shadow_dir(root: &Path) -> PathBuf {
    let mut h = DefaultHasher::new();
    root.hash(&mut h);
    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    base.join("tokioscope")
        .join("projects")
        .join(format!("{:016x}", h.finish()))
}

/// Copy source files from the original into the shadow, honoring `.gitignore`
/// and skipping `target/`, `.git/`, `node_modules/`. The shadow's own `target/`
/// is left intact so rebuilds are incremental.
fn sync_sources(src_root: &Path, dst_root: &Path) -> TsResult<()> {
    let walker = ignore::WalkBuilder::new(src_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .require_git(false)
        .build();
    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let rel = match path.strip_prefix(src_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        // Skip the build dir only at the crate root (a user module legitimately
        // named `target` deeper in the tree must be copied), and VCS/vendored
        // dirs anywhere.
        let first_is_target = rel.components().next().map_or(false, |c| c.as_os_str() == "target");
        let has_vendor = rel.components().any(|c| {
            let s = c.as_os_str();
            s == ".git" || s == "node_modules"
        });
        if first_is_target || has_vendor {
            continue;
        }
        let dst = dst_root.join(rel);
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            std::fs::create_dir_all(&dst)?;
        } else {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, &dst)?;
        }
    }
    Ok(())
}

fn instrument_tree(
    shadow: &Path,
    entry_rel: &Path,
    entry: &rewriter::EntryResult,
    runtime: &RuntimeConfig,
    has_lib: bool,
) -> TsResult<()> {
    let src_dir = shadow.join("src");
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir)? {
            let e = e?;
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            if path.file_name().and_then(|s| s.to_str()) == Some("__ts_tracer.rs") {
                continue;
            }
            let rel = path.strip_prefix(shadow).unwrap_or(&path);
            if rel == entry_rel {
                std::fs::write(&path, render_entry(entry, runtime))?;
            } else if !has_lib && !is_aux_crate_root(rel) {
                // Bin-only project: every src file is part of the bin crate, so
                // `crate::__ts_tracer` resolves. With a lib present, or under
                // src/bin & src/examples (each its own crate root), sibling files
                // belong to a different crate — leave them verbatim to stay
                // compilable (their async simply isn't traced).
                let original = std::fs::read_to_string(&path)?;
                std::fs::write(&path, with_tracer_use(&rewriter::rewrite_file(&original)))?;
            }
        }
    }
    Ok(())
}

/// Files under `src/bin/` and `src/examples/` are each their own crate root, so
/// `crate::__ts_tracer` (declared only in the bin's `main.rs`) is unresolvable
/// there — they must be left verbatim.
fn is_aux_crate_root(rel: &Path) -> bool {
    rel.starts_with(Path::new("src").join("bin"))
        || rel.starts_with(Path::new("src").join("examples"))
}

/// Append the tracer alias as a trailing item — appending (not prepending) keeps
/// any leading `#![..]` inner attributes valid.
fn with_tracer_use(code: &str) -> String {
    format!("{code}\n#[allow(unused_imports)]\nuse crate::__ts_tracer as __tracer;\n")
}

fn render_entry(entry: &rewriter::EntryResult, runtime: &RuntimeConfig) -> String {
    let flavor = match runtime.flavor {
        Flavor::CurrentThread => "current_thread",
        Flavor::MultiThread => "multi_thread",
    };
    format!(
        "{body}\n\
         #[allow(unused_imports)]\n\
         use crate::__ts_tracer as __tracer;\n\
         mod __ts_tracer;\n\
         fn main() {{\n    \
            __ts_tracer::bootstrap(\"{flavor}\", {workers}, {slots}, async move {{ let _ = __ts_user_main().await; }});\n\
         }}\n",
        body = entry.code,
        flavor = flavor,
        workers = runtime.worker_threads,
        slots = runtime.blocking_slots,
    )
}

fn patch_manifest(shadow: &Path) -> TsResult<()> {
    let path = shadow.join("Cargo.toml");
    let text = std::fs::read_to_string(&path)?;
    let mut doc = text
        .parse::<DocumentMut>()
        .map_err(|e| TsError::Parse(e.to_string()))?;

    // The tracer needs paused-time (`test-util`) and a broad tokio surface.
    ensure_dep(&mut doc, "tokio", "1", &["full", "test-util"]);
    ensure_dep(&mut doc, "serde", "1", &["derive"]);
    ensure_dep(&mut doc, "serde_json", "1", &[]);
    ensure_dep(&mut doc, "parking_lot", "0.12", &[]);

    std::fs::write(&path, doc.to_string())?;
    Ok(())
}

/// Ensure `[dependencies].<name>` exists and carries `want_features`. Never
/// downgrades an existing version; merges features (promoting a bare string dep
/// or a `workspace = true` dep into a table as needed).
fn ensure_dep(doc: &mut DocumentMut, name: &str, default_version: &str, want_features: &[&str]) {
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = Item::Table(Table::new());
    }
    let deps = &mut doc["dependencies"];

    if deps.get(name).is_none() {
        if want_features.is_empty() {
            deps[name] = value(default_version);
        } else {
            let mut it = InlineTable::new();
            it.insert("version", default_version.into());
            let mut arr = Array::new();
            for f in want_features {
                arr.push(*f);
            }
            it.insert("features", Value::Array(arr));
            deps[name] = Item::Value(Value::InlineTable(it));
        }
        return;
    }

    if want_features.is_empty() {
        return;
    }

    let item = &mut deps[name];
    if item.is_str() {
        let ver = item.as_str().unwrap_or("").to_string();
        let mut it = InlineTable::new();
        if !ver.is_empty() {
            it.insert("version", ver.into());
        }
        *item = Item::Value(Value::InlineTable(it));
    }

    let mut existing: Vec<String> = item
        .get("features")
        .and_then(|f| f.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    for f in want_features {
        if !existing.iter().any(|e| e == f) {
            existing.push(f.to_string());
        }
    }
    let mut arr = Array::new();
    for f in &existing {
        arr.push(f.as_str());
    }
    item["features"] = value(arr);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("runner-template")
    }

    #[test]
    fn prepare_generates_well_formed_shadow() {
        let base = std::env::temp_dir().join(format!("ts_proj_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("src").join("bin")).unwrap();
        std::fs::write(
            base.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ntokio = \"1\"\n",
        )
        .unwrap();
        std::fs::write(
            base.join("src").join("main.rs"),
            "#[tokio::main]\nasync fn main() {\n    tokio::spawn(async {});\n    tokio::time::sleep(std::time::Duration::from_millis(1)).await;\n    println!(\"hi\");\n}\n",
        )
        .unwrap();
        std::fs::write(
            base.join("src").join("helper.rs"),
            "pub async fn h() { tokio::task::yield_now().await; }\n",
        )
        .unwrap();
        // A second binary: its crate root has no __ts_tracer, so it must stay verbatim.
        std::fs::write(
            base.join("src").join("bin").join("aux.rs"),
            "fn main() { println!(\"aux\"); }\n",
        )
        .unwrap();

        let prepared = prepare(&base, &template_dir()).expect("prepare");
        let shadow = prepared.root.clone();

        assert!(shadow.join("src").join("__ts_tracer.rs").is_file());

        let main = std::fs::read_to_string(shadow.join("src").join("main.rs")).unwrap();
        assert!(main.contains("__ts_user_main"));
        assert!(main.contains("mod __ts_tracer;"));
        assert!(main.contains("__ts_tracer::bootstrap"));
        assert!(!main.contains("tokio::main"));
        syn::parse_file(&main).expect("generated main.rs must parse");

        let helper = std::fs::read_to_string(shadow.join("src").join("helper.rs")).unwrap();
        assert!(helper.contains("use crate::__ts_tracer as __tracer;"));
        assert!(helper.contains("__tracer::yield_now"));

        // src/bin/aux.rs is a separate crate root — must NOT get the tracer alias.
        let aux = std::fs::read_to_string(shadow.join("src").join("bin").join("aux.rs")).unwrap();
        assert!(!aux.contains("__ts_tracer"));

        let cargo = std::fs::read_to_string(shadow.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("test-util"));
        assert!(cargo.contains("parking_lot"));

        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&shadow);
    }
}
