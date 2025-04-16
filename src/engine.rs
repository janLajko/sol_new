use std::sync::Arc;

use futures_util::StreamExt;
use redis::aio::MultiplexedConnection;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_transaction_status::{option_serializer::OptionSerializer, UiInnerInstructions, UiTransactionStatusMeta};
use tokio::sync::Mutex;
use tracing::{debug, info, trace};
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;

use crate::{
    cache::{
        add_token_info, check_mk, from_pool_query_token_mint, query_token_info, update_mk
    }, client::GrpcClient, constants::{
        GRPC, PUMPAMM_PROGRAM_ID, PUMPFUN_PROGRAM_ID, REDIS_URL, RPC
    }, tg_bot::tg_bot::get_instance, types::TargetEvent, utils::{
        cal_pumpamm_marketcap_precise, cal_pumpamm_price, cal_pumpfun_marketcap, cal_pumpfun_price, convert_to_encoded_tx
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
                vec![PUMPAMM_PROGRAM_ID.to_string(), PUMPFUN_PROGRAM_ID.to_string()],
                vec![],
                vec![],
                yellowstone_grpc_proto::geyser::CommitmentLevel::Confirmed,
            )
            .await?;

        let mut block_times = 0;

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
                            debug!("check mk!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
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

    // check instruction
    async fn check_instruction(
        &self,
        inner_ixs: Vec<UiInnerInstructions>,
    ) -> Result<()> {
        let mut conn = self.redis.clone();

        // let mut temp_price = HashMap::new();
        for inner in inner_ixs {
            for ix in inner.instructions {
                if let Ok(target_event) = TargetEvent::try_from(ix.clone()) {
                    match target_event {
                        TargetEvent::PumpfunBuy(buy) => {
                            let sol_reserves = buy.virtual_sol_reserves;
                            let token_reserves = buy.virtual_token_reserves;
                            let price = cal_pumpfun_price(sol_reserves, token_reserves);
                            let market_cap = cal_pumpfun_marketcap(price);
                            update_mk(&mut conn, &buy.mint.to_string(), market_cap, &"".to_string()).await?;
                            // // info!("buy ===========> {:?}, {:?}, {:?}, {:?}, {:?}", buy.mint, sol_reserves, token_reserves, price, market_cap);

                            // temp_price.insert(buy.mint, (price, market_cap));
                        }

                        TargetEvent::PumpfunSell(sell) => {
                            let sol_reserves = sell.virtual_sol_reserves;
                            let token_reserves = sell.virtual_token_reserves;
                            let price = cal_pumpfun_price(sol_reserves, token_reserves);
                            let market_cap = cal_pumpfun_marketcap(price); 
                            update_mk(&mut conn, &sell.mint.to_string(), market_cap, &"".to_string()).await?;

                            // temp_price.insert(sell.mint, (price, market_cap));
                        }

                        TargetEvent::PumpfunCreate(create) => {
                            // let mint = create.mint;
                            // 默认是没有
                            // default is false
                            // if mint.to_string().ends_with("pump") {
                                // let have_x_or_tg = have_tg_or_x(&self.http, &mint.to_string())
                                //     .await
                                //     .unwrap_or(false); 
                                // todo！ get token info
                                add_token_info(&mut conn, &create).await?;
                            // }
                        }

                        TargetEvent::PumpfunComplete(_) => {
                            // safe delete
                        }

                        TargetEvent::PumpammCreatePool(pool_info) => {
                            let pool = pool_info;
                         
                            // 该池子的base_mint必须在redis中存在
                            if query_token_info(&mut conn, &pool.base_mint.to_string()).await.is_ok() {    
                                debug!("create pool: {:?}", pool);
                                let price = cal_pumpamm_price(pool.pool_base_amount, pool.pool_quote_amount);

                                let market_cap = cal_pumpamm_marketcap_precise(price);
                                debug!("create pool mint {} pool {} market cap: {}", pool.base_mint.to_string(), pool.pool.to_string(), market_cap);
                                
                                update_mk(&mut conn, &pool.base_mint.to_string(), market_cap, &pool.pool.to_string()).await?;
                            } 
                        } 

                        TargetEvent::PumpammBuy(buy) => {
                            // println!("buy ===========> {:?}", buy);
                            // TODO! AMM buy
                            let buy_info = buy;
                            if let Ok(mint) = from_pool_query_token_mint(&mut conn, &buy_info.pool.to_string()).await {   
                                // 如果毕业的话则更新价格和市场市值
                                // debug!("have token graduation");
                                // debug!("buy_info = {:?}", buy_info);
                                let price = cal_pumpamm_price(buy_info.pool_base_token_reserves, buy_info.pool_quote_token_reserves);

                                let market_cap = cal_pumpamm_marketcap_precise(price);
                                // debug!("buy mint {} pool {} price {} market cap: {}", mint, buy_info.pool.to_string(), price, market_cap);
                                 
                                update_mk(&mut conn, &mint, market_cap, &buy_info.pool.to_string()).await?;
                            } else {
                                continue;
                            }
                        } 
 
                        TargetEvent::PumpammSell(sell) => {
                            // println!("sell ===========> {:?}", sell);
                            // TODO! AMM sell
                            let sell_info = sell; 
                            if let Ok(mint) = from_pool_query_token_mint(&mut conn, &sell_info.pool.to_string()).await {   
                                // 如果毕业的话则更新价格和市场市值
                                // debug!("have token graduation");
                                // debug!("sell_info = {:?}", sell_info);
                                let price = cal_pumpamm_price(sell_info.pool_base_token_reserves, sell_info.pool_quote_token_reserves);

                                let market_cap = cal_pumpamm_marketcap_precise(price);
                                // debug!("sell mint {} pool {} market cap: {}", mint, sell_info.pool.to_string(), market_cap);
                                 
                                update_mk(&mut conn, &mint, market_cap, &sell_info.pool.to_string()).await?;
                            } else {
                                continue;
                            }
                        } 

                        TargetEvent::PumpammDeposit(deposit) => {
                            // TODO! AMM deposit
                            // println!("deposit ===========> {:?}", deposit);
                            if let Ok(mint) = from_pool_query_token_mint(&mut conn, &deposit.pool.to_string()).await {   
                                // 如果毕业的话则更新价格和市场市值
                                // debug!("have token graduation");
                                // debug!("deposit_info = {:?}", deposit);
                                let price = cal_pumpamm_price(deposit.pool_base_token_reserves, deposit.pool_quote_token_reserves);

                                let market_cap = cal_pumpamm_marketcap_precise(price);
                                // debug!("deposit mint {} pool {} market cap: {}", mint, deposit.pool.to_string(), market_cap);
                                 
                                update_mk(&mut conn, &mint, market_cap, &deposit.pool.to_string()).await?;
                            } else {
                                continue;
                            }
                        }

                        TargetEvent::PumpammWithdraw(withdraw) => {
                            // TODO! AMM withdraw
                            // println!("withdraw ===========> {:?}", withdraw);
                            if let Ok(mint) = from_pool_query_token_mint(&mut conn, &withdraw.pool.to_string()).await {   
                                // 如果毕业的话则更新价格和市场市值
                                // debug!("have token graduation");
                                // debug!("withdraw_info = {:?}", withdraw);
                                let price = cal_pumpamm_price(withdraw.pool_base_token_reserves, withdraw.pool_quote_token_reserves);

                                let market_cap = cal_pumpamm_marketcap_precise(price);
                                // debug!("withdraw mint {} pool {} market cap: {}", mint, withdraw.pool.to_string(), market_cap);
                                 
                                update_mk(&mut conn, &mint, market_cap, &withdraw.pool.to_string()).await?;
                            } else {
                                continue;
                            }
                        }
                        _ => todo!()
                    }
                }
                //  else {
                //     println!("ix ===========> {:?}", ix);
                // }
            }
        }

        // for (key, (_, mk)) in temp_price {
        //     // update marketcap
        //     update_mk(&mut conn, &key.to_string(), mk).await?;
        // } 

        Ok(())
    }
}



