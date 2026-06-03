use crate::errors::{TsError, TsResult};
use crate::events::EVENT_PREFIX;
use crate::harness::{ensure_runner_crate, render_main, CrateLayout};
use crate::parser::{Flavor, RuntimeConfig};
use crate::project;
use crate::rewriter;
use crate::tools;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex;

/// Hard ceiling on lines forwarded from one run, so a runaway program can't
/// flood the webview IPC bridge. The frontend caps its visualisation far below
/// this and cancels first; this is only a last-resort backstop.
const MAX_FORWARDED_LINES: u64 = 200_000;

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
    /// Bundled `runner-template/` source, resolved once at startup so the
    /// runner can self-heal (lazily build) without the caller threading it in.
    template_dir: PathBuf,
    /// Serializes warm-up builds: a second caller awaits the in-flight build
    /// instead of racing it or getting a `Busy` error.
    build_lock: Mutex<()>,
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    status: RunnerStatus,
    child: Option<Child>,
    layout: Option<CrateLayout>,
    /// Bumped on every `start`. A reader task only owns shared state while this
    /// still matches the generation it was spawned with — so a stale reader
    /// from a superseded run can't stomp a newer run's child/status.
    generation: u64,
}

impl Runner {
    pub fn new(app: AppHandle, template_dir: PathBuf) -> Self {
        Self {
            app,
            template_dir,
            build_lock: Mutex::new(()),
            inner: Arc::new(Mutex::new(Inner {
                status: RunnerStatus::default(),
                child: None,
                layout: None,
                generation: 0,
            })),
        }
    }

    pub async fn status(&self) -> RunnerStatus {
        self.inner.lock().await.status.clone()
    }

    /// Public warm-up: ensure the snippet runner crate is built and cached.
    /// Called fire-and-forget on app start; safe to call again concurrently.
    pub async fn ensure(&self) -> TsResult<()> {
        self.ensure_ready().await.map(|_| ())
    }

    /// Return the cached crate layout, building it first if necessary. Builds
    /// are serialized by `build_lock`, so a click on Run *during* the initial
    /// warm-up blocks on that build instead of failing with `NotReady` — the
    /// root cause of "runner crate not initialised".
    async fn ensure_ready(&self) -> TsResult<CrateLayout> {
        {
            let g = self.inner.lock().await;
            if let Some(l) = &g.layout {
                return Ok(l.clone());
            }
        }
        let _guard = self.build_lock.lock().await;
        // Another caller may have finished the build while we waited.
        {
            let g = self.inner.lock().await;
            if let Some(l) = &g.layout {
                return Ok(l.clone());
            }
        }
        self.build_template().await
    }

    /// Materialise the runner crate from the bundled template and prime its
    /// target cache with `cargo build`. Caller must hold `build_lock`.
    async fn build_template(&self) -> TsResult<CrateLayout> {
        {
            let mut g = self.inner.lock().await;
            g.status.building = true;
            g.status.last_error = None;
        }
        let _ = self.app.emit("ts:status", self.status().await);

        let layout = match ensure_runner_crate(&self.template_dir) {
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

        let res = tools::cargo_command()
            .args(["build", "--release", "--quiet"])
            .current_dir(&layout.root)
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
                g.layout = Some(layout.clone());
                drop(g);
                let _ = self.app.emit("ts:status", self.status().await);
                Ok(layout)
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

    /// Compile + run a single self-contained snippet through the template crate,
    /// streaming events via `ts:event`. Returns once the child has been spawned.
    pub async fn start(&self, source: String, runtime: RuntimeConfig) -> TsResult<()> {
        // Lazily build the runner crate if a run is requested before the
        // background warm-up finished (rather than erroring with NotReady).
        let layout = self.ensure_ready().await?;

        // Render rewritten user + main.
        let rewritten = rewriter::rewrite(&source);
        render_main(&layout, &rewritten, &runtime)?;

        // Cancel any prior run.
        self.cancel().await;

        let mut cmd = tools::cargo_command();
        cmd.args(["run", "--release", "--quiet"])
            .current_dir(&layout.root);
        apply_run_env(&mut cmd, &runtime);
        self.spawn_and_stream(cmd).await
    }

    /// Run an opened Cargo project. Best-effort: a traced shadow copy is built
    /// and run for visualization; if injection or the traced build fails, falls
    /// back to a plain `cargo run` of the original project (real output, no
    /// visualization) so the run still produces something useful.
    pub async fn start_project(&self, root: PathBuf) -> TsResult<()> {
        self.cancel().await;

        // Serialize shadow prepare+build: two overlapping runs would otherwise
        // rewrite the same shadow source files while a build reads them.
        let _build_guard = self.build_lock.lock().await;

        let prepared = match project::prepare(&root, &self.template_dir) {
            Ok(p) => p,
            Err(e) => {
                let _ = self
                    .app
                    .emit("ts:stderr", format!("⚠ 无法对该项目插桩,降级为直接运行(无可视化):{e}"));
                let mut cmd = tools::cargo_command();
                cmd.args(["run"]).current_dir(&root);
                return self.spawn_and_stream(cmd).await;
            }
        };

        // Build the traced shadow first so a compile failure degrades cleanly
        // rather than dumping rewritten-code errors as the "result".
        self.set_building(true).await;
        let (ok, build_err) = self.build_shadow(&prepared.root).await;
        self.set_building(false).await;
        if !ok {
            let _ = self.app.emit(
                "ts:stderr",
                "⚠ 项目插桩构建失败,降级为直接运行原项目(无可视化):".to_string(),
            );
            for line in build_err.lines() {
                let _ = self.app.emit("ts:stderr", line.to_string());
            }
            let mut cmd = tools::cargo_command();
            cmd.args(["run"]).current_dir(&root);
            return self.spawn_and_stream(cmd).await;
        }

        // Traced build succeeded — `cargo run` reuses those artifacts and
        // streams the tracer events.
        let mut cmd = tools::cargo_command();
        cmd.args(["run"]).current_dir(&prepared.root);
        apply_run_env(&mut cmd, &prepared.runtime);
        self.spawn_and_stream(cmd).await
    }

    async fn set_building(&self, building: bool) {
        {
            let mut g = self.inner.lock().await;
            g.status.building = building;
        }
        let _ = self.app.emit("ts:status", self.status().await);
    }

    /// Quietly build a shadow project. Returns (succeeded, captured stderr).
    async fn build_shadow(&self, dir: &std::path::Path) -> (bool, String) {
        match tools::cargo_command()
            .args(["build", "--quiet"])
            .current_dir(dir)
            .output()
            .await
        {
            Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stderr).to_string()),
            Err(e) => (false, format!("cargo build failed to start: {e}")),
        }
    }

    /// Spawn an already-configured cargo command and stream its JSONL events +
    /// stdout/stderr. Shared by snippet, file and project runs.
    async fn spawn_and_stream(&self, mut cmd: tokio::process::Command) -> TsResult<()> {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        // `cargo run` execs the user binary as a *child* of cargo. Put the whole
        // thing in its own process group so cancel() can signal the entire tree;
        // killing cargo alone would orphan the still-running user program.
        #[cfg(unix)]
        cmd.process_group(0);
        let mut child = cmd.spawn().map_err(|e| TsError::Cargo(e.to_string()))?;

        let stdout = child.stdout.take().ok_or_else(|| TsError::Other("no stdout".into()))?;
        let stderr = child.stderr.take().ok_or_else(|| TsError::Other("no stderr".into()))?;

        let generation = {
            let mut g = self.inner.lock().await;
            g.generation += 1;
            g.status.running = true;
            g.child = Some(child);
            g.generation
        };
        let _ = self.app.emit("ts:status", self.status().await);

        let app = self.app.clone();
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            let mut forwarded: u64 = 0;
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

                // Backstop against a runaway program (e.g. tight infinite loop)
                // flooding the IPC bridge faster than the UI can cancel it.
                forwarded += 1;
                if forwarded >= MAX_FORWARDED_LINES {
                    let _ = app.emit(
                        "ts:warn",
                        format!("output limit ({MAX_FORWARDED_LINES}) reached — stopping run"),
                    );
                    let mut g = inner.lock().await;
                    if let Some(mut c) = g.child.take() {
                        kill_process_tree(&c);
                        let _ = c.start_kill();
                    }
                    drop(g);
                    break;
                }
            }

            // Drain stderr (cargo build output / panics).
            let mut errlines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = errlines.next_line().await {
                let _ = app.emit("ts:stderr", line);
            }

            let mut g = inner.lock().await;
            // A rerun bumped the generation; the child handle and status now
            // belong to the newer run, so leave them alone.
            if g.generation != generation {
                return;
            }
            let exit = if let Some(mut c) = g.child.take() {
                c.wait().await.ok().and_then(|s| s.code())
            } else {
                None
            };
            g.status.running = false;
            let status = g.status.clone();
            drop(g);
            let _ = app.emit("ts:done", exit);
            let _ = app.emit("ts:status", status);
        });

        Ok(())
    }

    pub async fn cancel(&self) {
        let mut g = self.inner.lock().await;
        if let Some(mut c) = g.child.take() {
            // Kill the whole process group, not just cargo — otherwise the user
            // binary cargo spawned keeps running and keeps streaming events.
            kill_process_tree(&c);
            let _ = c.start_kill();
        }
        g.status.running = false;
        drop(g);
        let _ = self.app.emit("ts:status", self.status().await);
    }
}

/// Apply the deterministic-runtime environment the tracer reads at boot.
fn apply_run_env(cmd: &mut tokio::process::Command, runtime: &RuntimeConfig) {
    cmd.env(
        "TS_FLAVOR",
        match runtime.flavor {
            Flavor::CurrentThread => "current_thread",
            Flavor::MultiThread => "multi_thread",
        },
    )
    .env("TS_WORKERS", runtime.worker_threads.to_string())
    .env("TS_BLOCKING_SLOTS", runtime.blocking_slots.to_string());
}

/// Terminate the spawned child *and* everything it spawned. `cargo run` execs
/// the user binary as a child of cargo, so signalling only cargo leaves the
/// program orphaned and running. On Unix the child is spawned in its own
/// process group (see `start`), so a negative PID signals the entire group.
fn kill_process_tree(child: &Child) {
    let Some(pid) = child.id() else { return };
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        // `/T` kills the whole tree rooted at cargo. Block until taskkill has
        // finished walking it — otherwise the caller dropping/`start_kill`-ing
        // cargo's handle could terminate cargo first and orphan the child.
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
