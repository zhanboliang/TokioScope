use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("current_thread runtime, single worker");

    let a = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        println!("a wakes");
    });

    let b = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("b wakes");
        tokio::time::sleep(Duration::from_millis(400)).await;
        println!("b again");
    });

    let _ = tokio::join!(a, b);
    println!("bye");
}
