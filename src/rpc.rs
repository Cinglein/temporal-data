use crate::{db::RawTx, time::SlotMap, util::Config};
use bigdecimal::BigDecimal;
use futures::{sink::SinkExt, stream::StreamExt};
use maplit::hashmap;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tonic::transport::channel::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{geyser::subscribe_update::UpdateOneof, prelude::*};

const SOL: &str = "So11111111111111111111111111111111111111112";
const SOL_DECIMALS: i64 = 9;

pub async fn subscribe(
    config: &Config,
    send: mpsc::Sender<RawTx>,
    slotmap: Arc<Mutex<SlotMap>>,
) -> eyre::Result<()> {
    let mut client = GeyserGrpcClient::build_from_shared(config.rpc.to_string())?
        .x_token(Some(config.rpc_key.to_string()))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    let (mut subscribe_tx, mut stream) = client.subscribe().await?;
    subscribe_tx
        .send(SubscribeRequest {
            transactions: hashmap! { "target".to_owned() => SubscribeRequestFilterTransactions {
                account_include: vec![config.target.to_string()],
                ..Default::default()
            } },
            blocks_meta: hashmap! { "".to_owned() => SubscribeRequestFilterBlocksMeta {} },
            ..Default::default()
        })
        .await?;
    while let Some(message) = stream.next().await {
        match message {
            Ok(message) => match message.update_oneof {
                Some(UpdateOneof::BlockMeta(SubscribeUpdateBlockMeta {
                    slot,
                    block_time: Some(UnixTimestamp { timestamp }),
                    ..
                })) => {
                    slotmap.lock().await.insert(slot, timestamp);
                }
                Some(UpdateOneof::Transaction(SubscribeUpdateTransaction {
                    transaction:
                        Some(SubscribeUpdateTransactionInfo {
                            signature,
                            transaction:
                                Some(Transaction {
                                    message: Some(Message { account_keys, .. }),
                                    ..
                                }),
                            meta:
                                Some(TransactionStatusMeta {
                                    err,
                                    fee,
                                    pre_token_balances,
                                    post_token_balances,
                                    ..
                                }),
                            ..
                        }),
                    slot,
                })) => {
                    let signature = bs58::encode(signature).into_string();
                    let feepayer =
                        bs58::encode(account_keys.get(0).ok_or(eyre::eyre!("No signer found"))?)
                            .into_string();
                    let fee = BigDecimal::from_bigint(fee.into(), SOL_DECIMALS);
                    let data = if err.is_some() {
                        RawTx {
                            signature,
                            slot,
                            feepayer,
                            fee,
                            profit: 0.into(),
                        }
                    } else {
                        match parse_token_balance(
                            &feepayer,
                            &pre_token_balances,
                            &post_token_balances,
                        ) {
                            Ok(Some(profit)) => RawTx {
                                signature,
                                slot,
                                feepayer,
                                fee,
                                profit,
                            },
                            Ok(None) => RawTx {
                                signature,
                                slot,
                                feepayer,
                                fee,
                                profit: 0.into(),
                            },
                            Err(err) => {
                                log::error!("Error parsing token balance: {err:?}");
                                RawTx {
                                    signature,
                                    slot,
                                    feepayer,
                                    fee,
                                    profit: 0.into(),
                                }
                            }
                        }
                    };
                    if let Err(err) = send.send(data).await {
                        log::error!("Error sending data: {err:?}");
                    }
                }
                _ => {}
            },
            Err(err) => {
                log::error!("Stream error: {err:?}");
            }
        }
    }
    Ok(())
}

/// Parse raw token balance to a SOL profit number
/// We expect that the solmevbot will always arb
fn parse_token_balance<'a>(
    feepayer: &str,
    pre: &'a [TokenBalance],
    post: &'a [TokenBalance],
) -> eyre::Result<Option<BigDecimal>> {
    let mut changes: HashMap<String, BigDecimal> = HashMap::default();
    let post: HashMap<&'a str, &'a TokenBalance> = post
        .iter()
        .filter_map(|b| (b.owner.as_str() == feepayer).then_some((b.mint.as_str(), b)))
        .collect();
    assert!(!post.is_empty());
    for pre in pre.iter().filter(|b| b.owner.as_str() == feepayer) {
        if let Some(post) = post.get(pre.mint.as_str())
            && let Some(change) = parse_change(pre, post)
        {
            if change != 0.into() {
                changes.insert(pre.mint.to_string(), change);
            }
        }
    }
    if changes.is_empty() {
        return Ok(None);
    }
    if changes.len() != 1 {
        return Err(eyre::eyre!("More than one token balance change"));
    }
    Ok(Some(
        changes
            .remove(SOL)
            .ok_or(eyre::eyre!("No SOL token balance change"))?,
    ))
}

fn parse_change(pre: &TokenBalance, post: &TokenBalance) -> Option<BigDecimal> {
    let pre_balance = pre
        .ui_token_amount
        .as_ref()
        .and_then(|a| BigDecimal::from_str(&a.ui_amount_string).ok())?;
    let post_balance = post
        .ui_token_amount
        .as_ref()
        .and_then(|a| BigDecimal::from_str(&a.ui_amount_string).ok())?;
    let change = post_balance - pre_balance;
    Some(change)
}
