#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Wire schema for every JSONL line printed by the runner-template with the
/// `__TS__` prefix. Kept stable across versions — frontend `engine.ts` mirrors
/// this shape verbatim. The Tauri backend forwards events as opaque
/// `serde_json::Value`, so these enums act as documentation + a single source
/// of truth for the wire format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunnerEvent {
    /// Emitted once at boot.
    Boot {
        flavor: String,
        worker_threads: u32,
        blocking_slots: u32,
    },
    /// New task created (root task is id=0, parent=None).
    TaskSpawn {
        tick: u64,
        id: u64,
        parent: Option<u64>,
        line: u32,
        name: String,
        blocking: bool,
    },
    /// Task starts running on a worker.
    TaskPoll {
        tick: u64,
        id: u64,
        worker: u32,
        line: u32,
    },
    /// Task voluntarily yielded; goes back to ready queue.
    TaskYield {
        tick: u64,
        id: u64,
        line: u32,
    },
    /// Task started awaiting on something.
    TaskAwait {
        tick: u64,
        id: u64,
        line: u32,
        reason: AwaitReason,
    },
    /// Task was woken up (back to ready).
    TaskWake {
        tick: u64,
        id: u64,
        cause: WakeCause,
    },
    /// Blocking task occupies a slot for `duration_ticks`.
    BlockingStart {
        tick: u64,
        id: u64,
        slot: u32,
        duration_ticks: u32,
    },
    BlockingDone {
        tick: u64,
        id: u64,
        slot: u32,
    },
    /// Timer (sleep) fired.
    TimerFire {
        tick: u64,
        id: u64,
    },
    /// Task completed.
    TaskDone {
        tick: u64,
        id: u64,
    },
    /// Println output captured.
    Println {
        tick: u64,
        id: u64,
        line: u32,
        text: String,
    },
    /// Tick boundary marker (emitted at end of each tick).
    Tick {
        tick: u64,
    },
    /// Final event.
    Finish {
        tick: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AwaitReason {
    Timer,
    Blocking,
    Children,
    Generic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WakeCause {
    TimerFire,
    BlockingDone,
    ChildDone,
    Yield,
    Generic,
}

pub const EVENT_PREFIX: &str = "__TS__";
