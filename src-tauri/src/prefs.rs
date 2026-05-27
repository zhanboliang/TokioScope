#![allow(dead_code)]

// Preferences are persisted via tauri-plugin-store directly from the frontend.
// We expose this module as a placeholder for any backend-side defaults that
// other modules need to know about.

pub const DEFAULT_WORKER_THREADS: u32 = 4;
pub const DEFAULT_BLOCKING_SLOTS: u32 = 8;
pub const TICK_MILLIS: u64 = 100;
