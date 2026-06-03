//! Embedded interactive terminals backed by `portable-pty`. Each session owns a
//! real PTY pair; the slave runs the user's shell and the master is bridged to
//! an xterm.js view in the frontend. Output is streamed as `ts:pty` events and
//! the child's exit as `ts:pty-exit`.
//!
//! `portable-pty`'s reader is a blocking `std::io::Read`, so each session gets a
//! dedicated OS thread (not a tokio task). A second thread waits on the child to
//! reap it and report the exit code; `ChildKiller` lets `pty_kill` terminate it
//! from the command handler.

use crate::errors::{TsError, TsResult};
use crate::tools;
use crate::AppState;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
struct PtyChunk {
    id: u64,
    data: Vec<u8>,
}

#[derive(Clone, Serialize)]
struct PtyExit {
    id: u64,
    code: Option<i32>,
}

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

#[derive(Default)]
pub struct PtyRegistry {
    sessions: Mutex<HashMap<u64, Session>>,
    next_id: AtomicU64,
}

impl PtyRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

fn default_shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
    }
}

#[tauri::command]
pub fn pty_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    shell: Option<String>,
) -> TsResult<u64> {
    let size = PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    };
    let pair = native_pty_system()
        .openpty(size)
        .map_err(|e| TsError::Other(format!("openpty: {e}")))?;

    let shell = shell.unwrap_or_else(default_shell);
    let mut cmd = CommandBuilder::new(&shell);
    if let Some(dir) = cwd.as_ref() {
        cmd.cwd(dir);
    }
    // Give the shell a sane terminal type and a PATH that includes the rust
    // toolchain, so `cargo` works even when the app launched with a minimal env.
    cmd.env("TERM", "xterm-256color");
    cmd.env("PATH", tools::augmented_path());

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| TsError::Other(format!("spawn shell: {e}")))?;
    // Slave fd is no longer needed in this process; dropping it lets the master
    // see EOF cleanly once the child exits.
    drop(pair.slave);

    let killer = child.clone_killer();
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| TsError::Other(format!("clone reader: {e}")))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| TsError::Other(format!("take writer: {e}")))?;

    let id = state.ptys.next_id.fetch_add(1, Ordering::SeqCst);

    // Reader thread: stream bytes to the frontend.
    let app_read = app.clone();
    let mut reader = reader;
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = app_read.emit("ts:pty", PtyChunk { id, data: buf[..n].to_vec() });
                }
            }
        }
    });

    // Waiter thread: reap the child and report its exit code.
    let app_wait = app.clone();
    std::thread::spawn(move || {
        let code = child.wait().ok().map(|s| s.exit_code() as i32);
        let _ = app_wait.emit("ts:pty-exit", PtyExit { id, code });
    });

    state.ptys.sessions.lock().unwrap().insert(
        id,
        Session {
            master: pair.master,
            writer,
            killer,
        },
    );
    Ok(id)
}

#[tauri::command]
pub fn pty_write(state: State<'_, AppState>, id: u64, data: String) -> TsResult<()> {
    let mut sessions = state.ptys.sessions.lock().unwrap();
    if let Some(s) = sessions.get_mut(&id) {
        s.writer
            .write_all(data.as_bytes())
            .map_err(|e| TsError::Other(format!("pty write: {e}")))?;
        let _ = s.writer.flush();
    }
    Ok(())
}

#[tauri::command]
pub fn pty_resize(state: State<'_, AppState>, id: u64, cols: u16, rows: u16) -> TsResult<()> {
    let sessions = state.ptys.sessions.lock().unwrap();
    if let Some(s) = sessions.get(&id) {
        let _ = s.master.resize(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        });
    }
    Ok(())
}

#[tauri::command]
pub fn pty_kill(state: State<'_, AppState>, id: u64) -> TsResult<()> {
    if let Some(mut s) = state.ptys.sessions.lock().unwrap().remove(&id) {
        let _ = s.killer.kill();
    }
    Ok(())
}
