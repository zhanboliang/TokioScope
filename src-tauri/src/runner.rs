use crate::errors::{TsError, TsResult};
use crate::events::EVENT_PREFIX;
use crate::harness::{ensure_runner_crate, render_main, CrateLayout};
use crate::parser::{Flavor, RuntimeConfig};
use crate::rewriter;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Default, Clone, Serialize)]
pub struct RunnerStatus {
    pub ready: bool,
    pub building: bool,
    pub running: bool,
    pub cache_dir: Option<String>,
    pub last_error: Option<String>,
}

pub struct Runner {
    app: AppHandle,
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    status: RunnerStatus,
    child: Option<Child>,
    layout: Option<CrateLayout>,
}

impl Runner {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            inner: Arc::new(Mutex::new(Inner {
                status: RunnerStatus::default(),
                child: None,
                layout: None,
            })),
        }
    }

    pub async fn status(&self) -> RunnerStatus {
        self.inner.lock().await.status.clone()
    }

    /// Initialise the runner crate in the user cache dir from the bundled
    /// `runner-template/` and prime its target cache with `cargo build`.
    pub async fn ensure(&self, template_dir: PathBuf) -> TsResult<()> {
        {
            let mut g = self.inner.lock().await;
            if g.status.building {
                return Err(TsError::Busy);
            }
            g.status.building = true;
            g.status.last_error = None;
        }
        let app = self.app.clone();
        let _ = app.emit("ts:status", self.status().await);

        let layout = match ensure_runner_crate(&template_dir) {
            Ok(l) => l,
            Err(e) => {
                self.fail_build(e.to_string()).await;
                return Err(TsError::Other("failed to materialise runner crate".into()));
            }
        };

        // Render an empty stub so the crate is buildable on first warm-up.
        let stub = "pub fn __ts_user_entry() -> impl std::future::Future<Output = ()> + Send + 'static { async {} }\n";
        if let Err(e) = render_main(&layout, stub, &RuntimeConfig::default()) {
            self.fail_build(format!("stub render failed: {e}")).await;
            return Err(TsError::Other("stub render failed".into()));
        }

        let cargo_dir = layout.root.clone();
        let res = Command::new("cargo")
            .args(["build", "--release", "--quiet"])
            .current_dir(&cargo_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match res {
            Ok(mut child) => {
                let _ = child.wait().await;
                let mut g = self.inner.lock().await;
                g.status.building = false;
                g.status.ready = true;
                g.status.cache_dir = Some(layout.root.display().to_string());
                g.layout = Some(layout);
                drop(g);
                let _ = self.app.emit("ts:status", self.status().await);
                Ok(())
            }
            Err(e) => {
                self.fail_build(format!("cargo not found: {e}")).await;
                Err(TsError::Cargo(e.to_string()))
            }
        }
    }

    async fn fail_build(&self, msg: String) {
        let mut g = self.inner.lock().await;
        g.status.building = false;
        g.status.ready = false;
        g.status.last_error = Some(msg);
        drop(g);
        let _ = self.app.emit("ts:status", self.status().await);
    }

    /// Compile + run user code, streaming events via `ts:event` and stdout
    /// via `ts:stdout`. Returns once the child has been spawned.
    pub async fn start(&self, source: String, runtime: RuntimeConfig) -> TsResult<()> {
        let layout = {
            let g = self.inner.lock().await;
            g.layout
                .clone()
                .ok_or_else(|| TsError::NotReady("runner crate not initialised".into()))?
        };

        // Render rewritten user + main.
        let rewritten = rewriter::rewrite(&source);
        render_main(&layout, &rewritten, &runtime)?;

        // Cancel any prior run.
        self.cancel().await;

        let mut child = Command::new("cargo")
            .args(["run", "--release", "--quiet"])
            .current_dir(&layout.root)
            .env("TS_FLAVOR", match runtime.flavor {
                Flavor::CurrentThread => "current_thread",
                Flavor::MultiThread => "multi_thread",
            })
            .env("TS_WORKERS", runtime.worker_threads.to_string())
            .env("TS_BLOCKING_SLOTS", runtime.blocking_slots.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| TsError::Cargo(e.to_string()))?;

        let stdout = child.stdout.take().ok_or_else(|| TsError::Other("no stdout".into()))?;
        let stderr = child.stderr.take().ok_or_else(|| TsError::Other("no stderr".into()))?;

        {
            let mut g = self.inner.lock().await;
            g.status.running = true;
            g.child = Some(child);
        }
        let _ = self.app.emit("ts:status", self.status().await);

        let app = self.app.clone();
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(rest) = line.strip_prefix(EVENT_PREFIX) {
                    match serde_json::from_str::<serde_json::Value>(rest) {
                        Ok(v) => {
                            let _ = app.emit("ts:event", v);
                        }
                        Err(e) => {
                            let _ = app.emit(
                                "ts:warn",
                                format!("bad event: {e} on line: {line}"),
                            );
                        }
                    }
                } else {
                    let _ = app.emit("ts:stdout", line);
                }
            }

            // Drain stderr (cargo build output / panics).
            let mut errlines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = errlines.next_line().await {
                let _ = app.emit("ts:stderr", line);
            }

            let mut g = inner.lock().await;
            let exit = if let Some(mut c) = g.child.take() {
                c.wait().await.ok().and_then(|s| s.code())
            } else {
                None
            };
            g.status.running = false;
            drop(g);
            let _ = app.emit("ts:done", exit);
        });

        Ok(())
    }

    pub async fn cancel(&self) {
        let mut g = self.inner.lock().await;
        if let Some(mut c) = g.child.take() {
            let _ = c.start_kill();
        }
        g.status.running = false;
    }
}
