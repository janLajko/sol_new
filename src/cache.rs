use std::collections::HashMap;

use redis::{aio::MultiplexedConnection, AsyncCommands, RedisResult};
use solana_sdk::timing::timestamp;
use tracing::info;

use crate::{ai::{generate_token_summary, TokenInfo}, constants::MARKET_CAP, tg_bot::{tg_bot::TokenDetails, tg_bot_type::BotInstance}, types::CreateEvent, utils::format_timestamp_to_et, x::{Tweet, XClient}};
const TOKEN_SET_KEY: &str = "token_info_set";

// ! blockhash 相关
pub async fn get_block_hash_str(conn: &mut MultiplexedConnection) -> RedisResult<String> {
    redis::cmd("get").arg("blockhash").query_async(conn).await
}

// ! 代币 相关
pub async fn add_token_info(
    conn: &mut MultiplexedConnection, 
    create: &CreateEvent,
) -> RedisResult<()> {
    // info = mint|mk|create_time|token_name|token_symbol|token_uri|user|bonding_curve
    let info = format!("{}|{}|{}|{}|{}|{}|{}|{}", create.mint, 0, timestamp(), create.name, create.symbol, create.uri, create.user.to_string(), create.bonding_curve.to_string());
    conn.hset(TOKEN_SET_KEY, create.mint.to_string(), info)
        .await
}

pub async fn update_mk(
    conn: &mut MultiplexedConnection,
    mint: &str,
    market_cap: f32,
) -> RedisResult<()> {
    match conn.hget::<_, _, String>(TOKEN_SET_KEY, mint).await {
        Ok(old_info) => {
            let splits: Vec<_> = old_info.split("|").collect();

            let (mint, create_time) = (splits[0], splits[2]);
            let new_info = format!("{}|{}|{}|{}|{}|{}|{}|{}", mint, market_cap, create_time, splits[3], splits[4], splits[5], splits[6], splits[7]);
            conn.hset(TOKEN_SET_KEY, mint, new_info).await
        } 
        Err(_) => Ok(()),
    }
}

pub async fn check_mk(conn: &mut MultiplexedConnection, instance: BotInstance, x_instance: XClient) -> RedisResult<()> {
    // 获取所有数据
    match conn
        .hgetall::<'_, _, HashMap<String, String>>(TOKEN_SET_KEY)
        .await
    {
        Ok(result) => {
            let mut tokens_to_exist = result.clone();
            for (_, info) in result {
                let splits: Vec<_> = info.split("|").collect();
                let (mint, mk, create_time) = (
                    splits[0],
                    splits[1].parse::<f32>().unwrap(),
                    splits[2].parse::<u64>().unwrap(),
                );

                info!("===============> {}|{}|{}", mint, mk, *MARKET_CAP);

                // 检查是否是新代币（创建时间不超过10分钟）
                let is_new_coin = create_time + 600_000 > timestamp();
                // 检查市值是否达标
                let has_enough_market_cap = mk >= *MARKET_CAP;
                
                // 如果不是新代币，删除
                if !is_new_coin {
                    conn.hdel(TOKEN_SET_KEY, mint).await?;
                    tokens_to_exist.remove(&mint.to_string());
                }
                // 否则，如果是新代币但市值不够，也删除
                else if !has_enough_market_cap {
                    conn.hdel(TOKEN_SET_KEY, mint).await?;
                    tokens_to_exist.remove(&mint.to_string());
                }
            }

            // 准备要处理的代币信息和相关状态
            let mut tokens_to_process = Vec::new();
            
            for (mint, info) in tokens_to_exist {
                // 检查是否已经发送过警报
                if !is_token_alert_sent(conn, &mint).await? {
                    // 标记为已发送
                    mark_token_alert_sent(conn, &mint).await?;
                    // 添加到要处理的列表
                    tokens_to_process.push((mint, info));
                }
            }
            
            // 只有在有需要处理的代币时才启动异步任务
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
                        
                        // 直接发送消息，不需要再次检查是否已发送
                        let _ = instance.send_coin_alert(&token_details).await;
                    }
                });
            }

            Ok(())
        }
        Err(e) => Err(e),
    }
}


// 在Redis中存储已发送消息的代币标记
pub async fn mark_token_alert_sent(conn: &mut MultiplexedConnection, mint: &str) -> RedisResult<()> {
    conn.set(format!("token_alert_sent:{}", mint), 1).await
}

pub async fn is_token_alert_sent(conn: &mut MultiplexedConnection, mint: &str) -> RedisResult<bool> {
    conn.exists(format!("token_alert_sent:{}", mint)).await
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
        // 1. 添加一个token info
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

        // 2. 更新mk
        update_mk(&mut con, &mint.to_string(), 100.0).await?;

        // 3. 停顿后检查
        sleep(Duration::from_secs(11));
        check_mk(&mut con, instance, get_x_instance()).await?;

        Ok(())
    }
}

/*

用一个数组，记录当前检查过的所有代币信息，具体是 mint|mk|create_time

当一笔交易进入，更新每个代币的mk

每30s, 检查一次所有代币，只要create_time至今超过15分钟，并且mk还低于7000，则将该代币从redis中删除

*/
