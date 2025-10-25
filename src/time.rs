use crate::util::Config;
use core::num::NonZeroUsize;
use lru::LruCache;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use tokio::{sync::Mutex, time::*};

const SLEEP_INTERVAL: Duration = Duration::from_millis(400);
const MAX_SLOTS: NonZeroUsize = NonZeroUsize::new(100).unwrap();

pub struct SlotMap {
    client: RpcClient,
    slots: LruCache<u64, i64>,
}

impl SlotMap {
    pub async fn new(config: &Config) -> eyre::Result<Arc<Mutex<Self>>> {
        let client = RpcClient::new(format!("{}/{}", config.rpc, config.rpc_key));
        client.get_health().await?;
        let slots = LruCache::new(MAX_SLOTS);
        Ok(Arc::new(Mutex::new(Self { client, slots })))
    }
    pub async fn get(&mut self, slot: &u64) -> eyre::Result<i64> {
        for _ in 0..2 {
            match self.slots.get(slot).copied() {
                None => {
                    sleep(SLEEP_INTERVAL).await;
                }
                Some(slot) => {
                    return Ok(slot);
                }
            }
        }
        let time = self.client.get_block_time(*slot).await?;
        Ok(time)
    }
    pub fn insert(&mut self, slot: u64, ts: i64) {
        self.slots.put(slot, ts);
    }
}
