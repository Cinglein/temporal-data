use crate::time::SlotMap;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool, query};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::{Mutex, mpsc};

const BATCH_SIZE: usize = 100;

#[derive(Clone, Debug)]
pub struct RawTx {
    pub feepayer: String,
    pub signature: String,
    pub slot: u64,
    pub fee: BigDecimal,
    pub profit: BigDecimal,
}

#[derive(Serialize, Deserialize, FromRow, Clone, Debug)]
pub struct Tx {
    pub feepayer: String,
    pub signature: String,
    pub ts: DateTime<Utc>,
    pub slot: u64,
    pub fee: BigDecimal,
    pub profit: BigDecimal,
}

pub async fn insert_txs(pool: &PgPool, txs: Vec<Tx>) -> Result<(), Error> {
    let (feepayers, signatures, ts, slots, fees, profits): (
        Vec<_>,
        Vec<_>,
        Vec<_>,
        Vec<_>,
        Vec<_>,
        Vec<_>,
    ) = txs
        .into_iter()
        .map(|r| {
            (
                r.feepayer,
                r.signature,
                r.ts,
                r.slot as i64,
                r.fee,
                r.profit,
            )
        })
        .multiunzip();
    query!(
        r#"
        INSERT INTO txs (feepayer, signature, ts, slot, fee, profit)
        SELECT * 
        FROM UNNEST($1::text[], $2::text[], $3::timestamptz[], $4::bigint[], $5::numeric[], $6::numeric[])
        "#,
        &feepayers,
        &signatures,
        &ts,
        &slots,
        &fees,
        &profits
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub fn db_loop(pool: PgPool, mut recv: mpsc::Receiver<RawTx>, slotmap: Arc<Mutex<SlotMap>>) {
    tokio::spawn(async move {
        let mut buffer = Vec::with_capacity(BATCH_SIZE);
        while recv.recv_many(&mut buffer, BATCH_SIZE).await != 0 {
            let buffer = std::mem::replace(&mut buffer, Vec::with_capacity(BATCH_SIZE));
            let slots: HashSet<_> = buffer.iter().map(|raw| raw.slot).collect();
            let (timestamps, errs): (HashMap<u64, i64>, Vec<_>) =
                futures::future::join_all(slots.into_iter().map(async |slot| {
                    let ts = slotmap.lock().await.get(&slot).await.map_err(|err| {
                        eyre::eyre!("Error finding blocktime for slot {slot}: {err:?}")
                    })?;
                    Ok::<_, eyre::Report>((slot, ts))
                }))
                .await
                .into_iter()
                .partition_result();
            for err in errs.into_iter() {
                log::error!("Error getting blocktime: {err:?}");
            }
            let (txs, errs): (Vec<_>, Vec<_>) = buffer
                .iter()
                .cloned()
                .map(
                    |RawTx {
                         feepayer,
                         signature,
                         slot,
                         fee,
                         profit,
                     }| {
                        let ts = timestamps
                            .get(&slot)
                            .ok_or(eyre::eyre!("Error getting timestamp for {slot}"))?;
                        let ts = DateTime::from_timestamp_secs(*ts)
                            .ok_or(eyre::eyre!("Error parsing timestamp {ts}"))?;
                        Ok::<_, eyre::Report>(Tx {
                            feepayer,
                            signature,
                            ts,
                            slot,
                            fee,
                            profit,
                        })
                    },
                )
                .partition_result();
            for err in errs.into_iter() {
                log::error!("Error making TX: {err:?}");
            }
            if let Err(err) = insert_txs(&pool, txs).await {
                log::error!("Error inserting txs into database: {err:?}");
            }
        }
    });
}
