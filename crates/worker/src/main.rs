#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod context;
pub mod tasks;

#[tokio::main]
async fn main() {
    // Placeholder worker entrypoint; real worker logic will be migrated
    println!("Worker starting...");
}
