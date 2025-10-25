use sqlx::PgPool;
use std::{env, sync::Arc};
use tokio::sync::mpsc;

pub mod db;
pub mod rpc;
pub mod time;
pub mod util;

pub use db::*;
pub use rpc::*;
pub use time::*;
pub use util::*;

const CHANNEL_MAX: usize = 10000;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let config = Config::load(env!("CARGO_MANIFEST_DIR"))?;
    env_logger::init();
    let pool = PgPool::connect(&config.database_url).await?;
    let slotmap = SlotMap::new(&config).await?;
    let (send, recv) = mpsc::channel(CHANNEL_MAX);
    db_loop(pool, recv, Arc::clone(&slotmap));
    subscribe(&config, send, Arc::clone(&slotmap)).await?;
    Ok(())
}
