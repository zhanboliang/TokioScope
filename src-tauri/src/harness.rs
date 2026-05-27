use crate::errors::TsResult;
use crate::parser::{Flavor, RuntimeConfig};
use std::path::{Path, PathBuf};

/// Layout of the wrapper crate copied into the user cache dir.
#[derive(Clone)]
pub struct CrateLayout {
    pub root: PathBuf,
}

impl CrateLayout {
    pub fn cache_dir() -> PathBuf {
        let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
        base.join("tokioscope").join("runner")
    }

    pub fn new() -> Self {
        Self { root: Self::cache_dir() }
    }

    pub fn cargo_toml(&self) -> PathBuf {
        self.root.join("Cargo.toml")
    }
    pub fn src_main(&self) -> PathBuf {
        self.root.join("src").join("main.rs")
    }
    pub fn src_tracer(&self) -> PathBuf {
        self.root.join("src").join("tracer.rs")
    }
    pub fn src_user(&self) -> PathBuf {
        self.root.join("src").join("user.rs")
    }
}

/// Ensure the runner crate exists in cache. If missing, materialize it from
/// the bundled `runner-template/` under the app's resource dir.
pub fn ensure_runner_crate(template_dir: &Path) -> TsResult<CrateLayout> {
    let layout = CrateLayout::new();
    std::fs::create_dir_all(layout.root.join("src"))?;

    let toml_src = template_dir.join("Cargo.toml");
    let tracer_src = template_dir.join("src").join("tracer.rs");
    let main_tmpl_src = template_dir.join("src").join("main.rs.tmpl");

    if toml_src.exists() {
        std::fs::copy(&toml_src, layout.cargo_toml())?;
    }
    if tracer_src.exists() {
        std::fs::copy(&tracer_src, layout.src_tracer())?;
    }
    if main_tmpl_src.exists() {
        let tmpl = std::fs::read_to_string(&main_tmpl_src)?;
        std::fs::write(layout.root.join("src").join("main.rs.tmpl"), tmpl)?;
    }

    Ok(layout)
}

/// Render `main.rs` for a given user code + runtime config. The user source
/// has already been rewritten by `rewriter::rewrite`.
pub fn render_main(layout: &CrateLayout, rewritten_user: &str, runtime: &RuntimeConfig) -> TsResult<()> {
    let tmpl_path = layout.root.join("src").join("main.rs.tmpl");
    let tmpl = std::fs::read_to_string(&tmpl_path).unwrap_or_else(|_| DEFAULT_MAIN_TMPL.to_string());
    let flavor = match runtime.flavor {
        Flavor::CurrentThread => "current_thread",
        Flavor::MultiThread => "multi_thread",
    };
    let rendered = tmpl
        .replace("{{FLAVOR}}", flavor)
        .replace("{{WORKERS}}", &runtime.worker_threads.to_string())
        .replace("{{BLOCKING_SLOTS}}", &runtime.blocking_slots.to_string());

    // The rewriter emits paths like `__tracer::*`. Bind that alias to the
    // sibling `tracer` module so the rewritten user code resolves.
    let user_src = format!("use crate::tracer as __tracer;\n\n{rewritten_user}");
    std::fs::write(layout.src_user(), user_src)?;
    std::fs::write(layout.src_main(), rendered)?;
    Ok(())
}

const DEFAULT_MAIN_TMPL: &str = r#"mod tracer;
mod user;

fn main() {
    tracer::bootstrap("{{FLAVOR}}", {{WORKERS}}, {{BLOCKING_SLOTS}}, user::__ts_user_entry());
}
"#;
