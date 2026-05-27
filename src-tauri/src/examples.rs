use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Example {
    pub id: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    pub code: &'static str,
}

pub fn list() -> Vec<Example> {
    vec![
        Example {
            id: "spawn_blocking_join",
            title: "spawn_blocking + join",
            blurb: "把 CPU 重活丢给 blocking 线程池,然后 join! 等结果",
            code: include_str!("../../examples/spawn_blocking_join.rs"),
        },
        Example {
            id: "yield_now",
            title: "yield_now 协作调度",
            blurb: "三个任务通过 yield_now 让出 worker,观察 Ready 队列波动",
            code: include_str!("../../examples/yield_now.rs"),
        },
        Example {
            id: "current_thread",
            title: "current_thread 单线程",
            blurb: "current_thread runtime 下的串行调度",
            code: include_str!("../../examples/current_thread.rs"),
        },
    ]
}
