use yoku_core::db::operations::*;


#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let y = create_workout().await.unwrap();
    println!("Created new workout with ID: {}", y.id);
}
