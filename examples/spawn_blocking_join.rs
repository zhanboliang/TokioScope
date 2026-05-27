use std::time::Duration;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    println!("start");

    let blocking = tokio::task::spawn_blocking(|| {
        // 模拟 CPU 重活,占用一个 blocking 槽位
        let mut sum = 0u64;
        for i in 0..50_000_000 {
            sum = sum.wrapping_add(i);
        }
        sum
    });

    let async_task = tokio::spawn(async {
        for i in 0..3 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            println!("async tick {}", i);
        }
        42
    });

    let (a, b) = tokio::join!(blocking, async_task);
    println!("done: blocking={:?} async={:?}", a.unwrap(), b.unwrap());
}
