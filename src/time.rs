use crate::util::Config;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::Mutex, time::*};

const SLEEP_INTERVAL: Duration = Duration::from_millis(400);

pub struct SlotMap {
    client: RpcClient,
    slots: HashMap<u64, i64>,
}

impl SlotMap {
    pub async fn new(config: &Config) -> eyre::Result<Arc<Mutex<Self>>> {
        let client = RpcClient::new(format!("{}/{}", config.rpc, config.rpc_key));
        client.get_health().await?;
        let slots = Default::default();
        Ok(Arc::new(Mutex::new(Self { client, slots })))
    }
    pub async fn get(&self, slot: &u64) -> eyre::Result<i64> {
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
        self.slots.insert(slot, ts);
    }
}
