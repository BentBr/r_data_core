#[tokio::main]
async fn main() -> r_data_core_core::error::Result<()> {
    r_data_core_worker::runtime::run().await
}
