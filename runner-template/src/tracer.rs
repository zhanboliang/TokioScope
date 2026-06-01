//! Runtime tracer compiled into the wrapper crate. Every primitive the rewriter
//! emits has a counterpart here. Events are serialised as JSONL prefixed with
//! `__TS__` so the host process can filter them out from normal stdout.

#![allow(dead_code)]

use parking_lot::Mutex;
use serde::Serialize;
use std::future::Future;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;

const PREFIX: &str = "__TS__";
const DEFAULT_BLOCKING_TICKS: u32 = 5;

static TICK: AtomicU64 = AtomicU64::new(0);
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);
static WORKER_COUNT: AtomicU32 = AtomicU32::new(1);
static BLOCKING: Mutex<Option<Vec<bool>>> = Mutex::new(None);

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Event<'a> {
    Boot {
        flavor: &'a str,
        worker_threads: u32,
        blocking_slots: u32,
    },
    TaskSpawn {
        tick: u64,
        id: u64,
        parent: Option<u64>,
        line: u32,
        name: String,
        blocking: bool,
    },
    TaskPoll {
        tick: u64,
        id: u64,
        worker: u32,
        line: u32,
    },
    TaskYield {
        tick: u64,
        id: u64,
        line: u32,
    },
    TaskAwait {
        tick: u64,
        id: u64,
        line: u32,
        reason: &'a str,
    },
    TaskWake {
        tick: u64,
        id: u64,
        cause: &'a str,
    },
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
    TimerFire {
        tick: u64,
        id: u64,
    },
    TaskDone {
        tick: u64,
        id: u64,
    },
    Println {
        tick: u64,
        id: u64,
        line: u32,
        text: String,
    },
    Tick {
        tick: u64,
    },
    Finish {
        tick: u64,
    },
}

fn emit(e: &Event<'_>) {
    let s = serde_json::to_string(e).expect("serialize event");
    println!("{PREFIX}{s}");
}

fn tick_now() -> u64 {
    TICK.load(Ordering::SeqCst)
}

tokio::task_local! {
    static CURRENT_TASK: u64;
}

fn current_id() -> u64 {
    CURRENT_TASK.try_with(|p| *p).unwrap_or(0)
}

/// Re-emit a `TaskPoll` when a task resumes execution after a suspension point.
/// In real tokio every resume IS a poll; without this the engine only ever sees
/// the task's first poll and leaves it stuck in `ready` for the rest of the run,
/// so the worker cores look idle even while the task is actively running.
fn repoll(id: u64, line: u32) {
    let worker = (id as u32) % WORKER_COUNT.load(Ordering::SeqCst).max(1);
    emit(&Event::TaskPoll {
        tick: tick_now(),
        id,
        worker,
        line,
    });
}

fn alloc_blocking_slot() -> u32 {
    let mut g = BLOCKING.lock();
    let slots = g.as_mut().expect("blocking state");
    for (i, used) in slots.iter_mut().enumerate() {
        if !*used {
            *used = true;
            return i as u32;
        }
    }
    // No free slot — return last index but mark as overflow (id high bit)
    (slots.len() as u32).saturating_sub(1)
}

fn free_blocking_slot(slot: u32) {
    let mut g = BLOCKING.lock();
    if let Some(slots) = g.as_mut() {
        if (slot as usize) < slots.len() {
            slots[slot as usize] = false;
        }
    }
}

/// Entry point invoked by the generated `main.rs`. Drives the deterministic
/// tick loop while running the user's future on a paused current_thread
/// tokio runtime.
pub fn bootstrap<F>(flavor: &str, workers: u32, blocking_slots: u32, user_fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let workers = workers.max(1);
    WORKER_COUNT.store(workers, Ordering::SeqCst);
    *BLOCKING.lock() = Some(vec![false; blocking_slots as usize]);

    emit(&Event::Boot {
        flavor,
        worker_threads: workers,
        blocking_slots,
    });

    let rt = Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .expect("tokio runtime");

    rt.block_on(async move {
        let root_id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
        emit(&Event::TaskSpawn {
            tick: tick_now(),
            id: root_id,
            parent: None,
            line: 0,
            name: "main".into(),
            blocking: false,
        });
        emit(&Event::TaskPoll {
            tick: tick_now(),
            id: root_id,
            worker: 0,
            line: 0,
        });
        let handle = tokio::spawn(CURRENT_TASK.scope(root_id, user_fut));

        let max_ticks: u64 = std::env::var("TS_MAX_TICKS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600);

        loop {
            // Let all ready-and-not-time-blocked work run to a quiescent state.
            for _ in 0..64 {
                tokio::task::yield_now().await;
            }
            let cur = TICK.fetch_add(1, Ordering::SeqCst);
            emit(&Event::Tick { tick: cur });

            if handle.is_finished() {
                emit(&Event::TaskDone {
                    tick: cur,
                    id: root_id,
                });
                emit(&Event::Finish { tick: cur });
                break;
            }
            if cur >= max_ticks {
                emit(&Event::Finish { tick: cur });
                break;
            }
            tokio::time::advance(Duration::from_millis(100)).await;
        }
    });
}

pub fn spawn<F, T>(line: u32, fut: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let parent = CURRENT_TASK.try_with(|p| *p).ok();
    let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
    emit(&Event::TaskSpawn {
        tick: tick_now(),
        id,
        parent,
        line,
        name: format!("task#{id} @L{line}"),
        blocking: false,
    });

    tokio::spawn(CURRENT_TASK.scope(id, async move {
        let worker = (id as u32) % WORKER_COUNT.load(Ordering::SeqCst).max(1);
        emit(&Event::TaskPoll {
            tick: tick_now(),
            id,
            worker,
            line,
        });
        let r = fut.await;
        emit(&Event::TaskDone {
            tick: tick_now(),
            id,
        });
        r
    }))
}

pub fn spawn_blocking<F, T>(line: u32, f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let parent = CURRENT_TASK.try_with(|p| *p).ok();
    let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
    let slot = alloc_blocking_slot();
    let duration_ticks = DEFAULT_BLOCKING_TICKS;

    emit(&Event::TaskSpawn {
        tick: tick_now(),
        id,
        parent,
        line,
        name: format!("blocking#{id} @L{line}"),
        blocking: true,
    });
    emit(&Event::BlockingStart {
        tick: tick_now(),
        id,
        slot,
        duration_ticks,
    });

    tokio::spawn(CURRENT_TASK.scope(id, async move {
        tokio::time::sleep(Duration::from_millis(duration_ticks as u64 * 100)).await;
        let res = f();
        emit(&Event::BlockingDone {
            tick: tick_now(),
            id,
            slot,
        });
        free_blocking_slot(slot);
        emit(&Event::TaskDone {
            tick: tick_now(),
            id,
        });
        res
    }))
}

pub async fn yield_now(line: u32) {
    let id = current_id();
    emit(&Event::TaskYield {
        tick: tick_now(),
        id,
        line,
    });
    tokio::task::yield_now().await;
    emit(&Event::TaskWake {
        tick: tick_now(),
        id,
        cause: "yield",
    });
    repoll(id, line);
}

pub async fn sleep(line: u32, dur: Duration) {
    let id = current_id();
    emit(&Event::TaskAwait {
        tick: tick_now(),
        id,
        line,
        reason: "timer",
    });
    tokio::time::sleep(dur).await;
    emit(&Event::TimerFire {
        tick: tick_now(),
        id,
    });
    emit(&Event::TaskWake {
        tick: tick_now(),
        id,
        cause: "timer_fire",
    });
    repoll(id, line);
}

pub async fn trace_await<F>(line: u32, fut: F) -> F::Output
where
    F: Future,
{
    let id = current_id();
    emit(&Event::TaskAwait {
        tick: tick_now(),
        id,
        line,
        reason: "generic",
    });
    let r = fut.await;
    emit(&Event::TaskWake {
        tick: tick_now(),
        id,
        cause: "generic",
    });
    repoll(id, line);
    r
}

pub fn println(line: u32, text: String) {
    let id = current_id();
    emit(&Event::Println {
        tick: tick_now(),
        id,
        line,
        text,
    });
    // Advance one tick per println so synchronous, output-driven code (no `.await`
    // points) still produces a multi-step, playable timeline instead of collapsing
    // into a single frame. Mirrors the bootstrap loop's Tick emission.
    let closed = TICK.fetch_add(1, Ordering::SeqCst);
    emit(&Event::Tick { tick: closed });
}

pub fn join_enter(line: u32) {
    let id = current_id();
    emit(&Event::TaskAwait {
        tick: tick_now(),
        id,
        line,
        reason: "children",
    });
}

pub fn join_exit(line: u32) {
    let id = current_id();
    emit(&Event::TaskWake {
        tick: tick_now(),
        id,
        cause: "child_done",
    });
    repoll(id, line);
}
