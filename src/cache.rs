use std::collections::HashMap;

use redis::{aio::MultiplexedConnection, AsyncCommands, RedisResult};
use solana_sdk::timing::timestamp;
use tracing::{debug, info};

use crate::{ai::{generate_token_summary, TokenInfo}, constants::{MARKET_CAP, NEW_COIN_MAX_TIME, NEW_COIN_MIN_TIME}, tg_bot::{tg_bot::TokenDetails, tg_bot_type::BotInstance}, types::CreateEvent, utils::format_timestamp_to_et, x::{Tweet, XClient}};
const TOKEN_SET_KEY: &str = "token_info_set";

// ! blockhash
pub async fn get_block_hash_str(conn: &mut MultiplexedConnection) -> RedisResult<String> {
    redis::cmd("get").arg("blockhash").query_async(conn).await
}

pub async fn add_token_info(
    conn: &mut MultiplexedConnection, 
    create: &CreateEvent,
) -> RedisResult<()> {
    // info = mint|mk|create_time|token_name|token_symbol|token_uri|user|bonding_curve|pool
    let info = format!("{}|{}|{}|{}|{}|{}|{}|{}|{}", create.mint, 0, timestamp(), create.name, create.symbol, create.uri, create.user.to_string(), create.bonding_curve.to_string(), "".to_string());
    let mint = format!("{}", create.mint.to_string());

    info!("create token info: {} | {} | {} | {} | {} ", mint,  timestamp(), create.name, create.symbol, create.user.to_string());  

    conn.hset(TOKEN_SET_KEY, mint, info)
        .await
}

pub async fn query_token_info(conn: &mut MultiplexedConnection, mint: &str) -> RedisResult<String> {
    match conn.hget::<_, _, String>(TOKEN_SET_KEY, mint).await {
        Ok(info) => Ok(info),
        Err(e) => Err(e),
    }
}

pub async fn from_pool_query_token_mint(conn: &mut MultiplexedConnection, pool: &str) -> RedisResult<String> {
    
    match conn.hgetall::<_, HashMap<String, String>>(TOKEN_SET_KEY).await {
        Ok(result) => {
            for (mint, info) in result {
                let splits: Vec<_> = info.split("|").collect();
                if splits.len() > 8 && splits[8] == pool.to_string() {
                    return Ok(mint.to_string());
                } 
            }
            Ok("".to_string())
        },
        Err(e) => Err(e),
    }
}


pub async fn update_mk(
    conn: &mut MultiplexedConnection,
    mint: &str,
    market_cap: f64,
    pool: &str,
) -> RedisResult<()> { 
    match conn.hget::<_, _, String>(TOKEN_SET_KEY, mint).await {
        Ok(old_info) => {
            let splits: Vec<_> = old_info.split("|").collect();

            let (mint, create_time) = (splits[0], splits[2]);
            let new_info = format!("{}|{}|{}|{}|{}|{}|{}|{}|{}", mint, market_cap.to_string(), create_time, splits[3], splits[4], splits[5], splits[6], splits[7], pool.to_string());
            conn.hset(TOKEN_SET_KEY, mint, new_info).await
        } 
        Err(_) => Ok(()), 
    }
}

pub async fn check_mk(conn: &mut MultiplexedConnection, instance: BotInstance, x_instance: XClient) -> RedisResult<()> {
    match conn
        .hgetall::<'_, _, HashMap<String, String>>(TOKEN_SET_KEY)
        .await
    {
        Ok(result) => {
            let mut tokens_to_exist = result.clone();
            for (_, info) in result {
                let splits: Vec<_> = info.split("|").collect();
                if splits.len() != 9 {
                    continue;
                }
                let (mint, mk, create_time, _, _, _, _, _, _pool) = (
                    splits[0], 
                    splits[1].parse::<f32>().unwrap(),
                    splits[2].parse::<u64>().unwrap(),  
                    splits[3], 
                    splits[4],
                    splits[5],
                    splits[6],
                    splits[7],
                    splits[8],
                ); 
                
                // 只在NEW_COIN_MIN_TIME和NEW_COIN_MAX_TIME之间检查市值
                let is_mid_age_coin = 
                    create_time + NEW_COIN_MIN_TIME <= timestamp() && 
                    create_time + NEW_COIN_MAX_TIME > timestamp();
                
                let has_enough_market_cap = mk >= *MARKET_CAP;

                if !has_enough_market_cap {
                    if is_mid_age_coin {
                        // Remove token from Redis hash set
                        conn.hdel(TOKEN_SET_KEY, mint).await?;
                        
                        // Remove from local tracking collection
                        tokens_to_exist.remove(&mint.to_string());
                        
                        info!("Remove token from Redis: {} | {} | {}", mint, timestamp(), mk);
                    }
                }
            }

            // Prepare tokens to process
            let mut tokens_to_process = Vec::new();
            
            for (mint, info) in tokens_to_exist { 
                let splits: Vec<_> = info.as_str().split("|").collect();
                if splits.len() != 9 {
                    continue;
                }
                let (_, _, create_time, _, _, _, _, _, _) = (
                    splits[0], 
                    splits[1].parse::<f32>().unwrap(),
                    splits[2].parse::<u64>().unwrap(), 
                    splits[3],
                    splits[4],
                    splits[5],
                    splits[6],
                    splits[7],
                    splits[8],
                ); 
                if splits[1].parse::<f32>().unwrap() > 0.0 {
                    info!("checking ======> mint: {} | create_time: {} | mk: {}", mint, create_time, splits[1]);
                }
                // Check if token alert has already been sent
                let mint_warning = format!("token_alert_sent:{}", mint);
                if !is_token_alert_sent(conn, &mint_warning).await? && splits[1].parse::<f32>().unwrap() > *MARKET_CAP {
                    // Mark as sent
                    mark_token_alert_sent(conn, &mint_warning).await?;
                    // Add to processing list
                    tokens_to_process.push((mint, info));
                }
            }

            if !tokens_to_process.is_empty() {
                tokio::spawn(async move {
                    for (mint, info) in tokens_to_process {
                        let splits: Vec<_> = info.split("|").collect();
                        let (_mint, mk, create_time, name, symbol, uri, user, _bonding_curve) = (
                            splits[0],
                            splits[1].parse::<f32>().unwrap(),
                            splits[2].parse::<u64>().unwrap(),
                            splits[3],
                            splits[4],
                            splits[5],
                            splits[6],
                            splits[7],
                        );
                        
                        // get token x info
                        let x_info = if let Ok(x_infos) = x_instance.search_tweets(&mint, None, Some("Top")).await {
                            x_infos.tweets.first().unwrap().clone()
                        } else {
                            Tweet::default()
                        };

                        // get token ai summary
                        let summary = generate_token_summary(&TokenInfo {
                            url: uri.to_string(),
                            name: name.to_string(),
                            symbol: symbol.to_string(),
                            x_content: x_info.text,
                        }).await.expect("Failed to get token summary");
                       
                        // send coin alert
                        let token_details = TokenDetails {
                            mint_address: mint.to_string(),   
                            name: name.to_string(),
                            symbol: symbol.to_string(),
                            url: uri.to_string(),
                            ai_analysis: summary,
                            ai_from_x_url: x_info.tweet_id,
                            market_cap: mk.to_string(),
                            creator: user.to_string(),
                            launch_time: format_timestamp_to_et(create_time),
                        };
                        
                        // Directly send message, no need to check again
                        let _ = instance.send_coin_alert(&token_details).await;
                    }
                });
            }

            Ok(())
        }
        Err(e) => Err(e),
    }
}



// Store token alert status in Redis
pub async fn mark_token_alert_sent(conn: &mut MultiplexedConnection, mint: &str) -> RedisResult<()> {
    conn.set(mint, 1).await  
}

pub async fn is_token_alert_sent(conn: &mut MultiplexedConnection, mint: &str) -> RedisResult<bool> {
    // Check if token alert has already been sent
    conn.exists(mint).await
}

#[cfg(test)]
mod test {
    use std::{thread::sleep, time::Duration};

    use solana_sdk::pubkey::Pubkey;

    use crate::{
        cache::{add_token_info, check_mk, update_mk}, constants::REDIS_URL, tg_bot::tg_bot::get_instance, types::CreateEvent, x::get_x_instance
    };

    #[tokio::test]
    async fn alert_test() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
        let instance = get_instance();
        let redis = redis::Client::open(REDIS_URL.to_string())?;
        let mut con = redis.get_multiplexed_async_connection().await?;
        // 1. Add a token info
        let mint = Pubkey::new_unique();
        add_token_info(
            &mut con,
            &CreateEvent {
                name: "".to_string(),
                symbol: "".to_string(),
                uri: "".to_string(),
                mint,
                user: Pubkey::new_unique(),
                bonding_curve: Pubkey::new_unique(),
            },
        )
        .await?;

        let pool = "0x1234";

        // 2. Update mk
        update_mk(&mut con, &mint.to_string(), 100.0, pool).await?;

        // 3. Pause and check
        sleep(Duration::from_secs(11));
        check_mk(&mut con, instance, get_x_instance()).await?;

        Ok(())
    }
}

