use std::{
    collections::HashMap, sync::Arc
};

use futures_util::StreamExt;
use redis::aio::MultiplexedConnection;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_transaction_status::{option_serializer::OptionSerializer, UiInnerInstructions, UiTransactionStatusMeta};
use tokio::sync::Mutex;
use tracing::info;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;

use crate::{
    cache::{
        add_token_info, check_mk, update_mk,
    }, client::GrpcClient, constants::{
        GRPC, PUMPFUN_PROGRAM_ID, REDIS_URL, RPC,
    }, tg_bot::tg_bot::get_instance, types::TargetEvent, utils::{
        cal_pumpfun_marketcap, cal_pumpfun_price, convert_to_encoded_tx,
    }, x::get_x_instance
};
use anyhow::{Context, Result};


pub struct Monitor {
    pub rpc: Arc<RpcClient>,
    pub http: reqwest::Client,
    pub transaction_lock: Arc<Mutex<()>>,
    pub redis: MultiplexedConnection,
}

impl Monitor {
    pub async fn new() -> Result<Self> {
        let redis = redis::Client::open(REDIS_URL.to_string())?;
        let conn = redis
            .get_multiplexed_async_connection()
            .await
            .context("get redis connection error")
            .unwrap();

        Ok(Self {
            rpc: Arc::new(RpcClient::new(RPC.to_string())),
            http: Client::new(),
            transaction_lock: Arc::new(Mutex::new(())),
            redis: conn,
        })
    }

    pub async fn run(&self) -> Result<()> {
        // grpc
        let grpc_url = GRPC.to_string();
        let tg_instance = get_instance();
        let x_instance = get_x_instance();
        
        let grpc = GrpcClient::new(grpc_url);
        let mut stream = grpc
            .subscribe_transaction(
                vec![PUMPFUN_PROGRAM_ID.to_string()],
                vec![],
                vec![],
                yellowstone_grpc_proto::geyser::CommitmentLevel::Confirmed,
            )
            .await?;

        let mut block_times = 0;
        // 接收消息
        // receive messages
        while let Some(Ok(sub)) = stream.next().await {
            if let Some(update) = sub.update_oneof {
                match update {
                    UpdateOneof::Transaction(sub_tx) => {
                        if let Some(tx_info) = sub_tx.transaction {
                            let tx = convert_to_encoded_tx(tx_info)?;
                            if let Some(meta) = tx.meta {
                                self.update_token_info(meta).await?;
                            }
                        }
                    }

                    UpdateOneof::BlockMeta(meta) => {
                        block_times += 1;
                        let mut conn = self.redis.clone();
                        redis::cmd("set")
                            .arg("blockhash")
                            .arg(&meta.blockhash)
                            .exec_async(&mut conn)
                            .await?;
                        if block_times == 100 {
                            check_mk(&mut conn, tg_instance.clone(), x_instance.clone()).await?;
                            block_times = 0;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    // 更新token info
    // update token info
    async fn update_token_info(
        &self,
        meta: UiTransactionStatusMeta,
    ) -> Result<()> {
        if let OptionSerializer::Some(inner_ixs) = meta.inner_instructions {
            self.check_instruction(inner_ixs).await
        } else {
            Ok(())
        }
    }

    // 检查内部指令
    // check instruction
    async fn check_instruction(
        &self,
        inner_ixs: Vec<UiInnerInstructions>,
    ) -> Result<()> {
        let mut conn = self.redis.clone();

        let mut temp_price = HashMap::new();
        for inner in inner_ixs {
            for ix in inner.instructions {
                if let Ok(target_event) = TargetEvent::try_from(ix) {
                    match target_event {
                        TargetEvent::PumpfunBuy(buy) => {
                            // 基本数据
                            // basic data
                            let sol_reserves = buy.virtual_sol_reserves;
                            let token_reserves = buy.virtual_token_reserves;
                            let price = cal_pumpfun_price(sol_reserves, token_reserves);
                            let market_cap = cal_pumpfun_marketcap(price);
                            // info!("buy ===========> {:?}, {:?}, {:?}, {:?}, {:?}", buy.mint, sol_reserves, token_reserves, price, market_cap);

                            temp_price.insert(buy.mint, (price, market_cap));
                        }

                        TargetEvent::PumpfunSell(sell) => {
                            // 出现了出售，通道发送
                            // sell occurred, channel sent
                            // 基本数据
                            // basic data
                            let sol_reserves = sell.virtual_sol_reserves;
                            let token_reserves = sell.virtual_token_reserves;
                            let price = cal_pumpfun_price(sol_reserves, token_reserves);
                            let market_cap = cal_pumpfun_marketcap(price);

                            temp_price.insert(sell.mint, (price, market_cap));
                        }

                        TargetEvent::PumpfunCreate(create) => {
                            let mint = create.mint;
                            info!("create token info: {}", mint.to_string());
                            // 默认是没有
                            // default is false
                            if mint.to_string().ends_with("pump") {
                                // let have_x_or_tg = have_tg_or_x(&self.http, &mint.to_string())
                                //     .await
                                //     .unwrap_or(false); 
                                // todo！ get token info
                                add_token_info(&mut conn, &create).await?;
                            }
                        }

                        TargetEvent::PumpfunComplete(_) => {
                            // safe delete
                        }

                        TargetEvent::PumpammCreatePool(_) => {
                            // TODO! 是否处理池子的创建 池子的创建应该暂时不需要处理
                        }

                        TargetEvent::PumpammBuy(buy) => {
                            // TODO! AMM buy
                        }

                        TargetEvent::PumpammSell(sell) => {
                            // TODO! AMM sell
                        }

                        TargetEvent::PumpammDeposit(deposit) => {
                            // TODO! AMM deposit
                        }

                        TargetEvent::PumpammWithdraw(withdraw) => {
                            // TODO! AMM withdraw
                        }
                        _ => todo!()
                    }
                }
            }
        }

        for (key, (_, mk)) in temp_price {
            // update marketcap
            update_mk(&mut conn, &key.to_string(), mk).await?;
        }

        Ok(())
    }
}



