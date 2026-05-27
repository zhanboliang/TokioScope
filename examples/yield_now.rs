#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    println!("3 tasks yielding cooperatively");

    let mut handles = Vec::new();
    for id in 0..3 {
        handles.push(tokio::spawn(async move {
            for round in 0..4 {
                println!("task {} round {}", id, round);
                tokio::task::yield_now().await;
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }
    println!("all done");
}
