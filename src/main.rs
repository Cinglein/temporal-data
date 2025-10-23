pub mod db;
pub mod rpc;
pub mod util;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Hello, world!");
    Ok(())
}
