use std::{str::FromStr, time::Duration};

use anyhow::{anyhow, Result};
use chrono::{TimeZone, Utc};
use chrono_tz::America::New_York;
use reqwest::Client;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{EncodedTransactionWithStatusMeta, UiTransactionEncoding};
use yellowstone_grpc_proto::{convert_from, geyser::SubscribeUpdateTransactionInfo};

use crate::{constants::PUMPFUN_PROGRAM_ID, types::CreateEvent};
pub fn convert_to_encoded_tx(
    tx_info: SubscribeUpdateTransactionInfo,
) -> Result<EncodedTransactionWithStatusMeta> {
    convert_from::create_tx_with_meta(tx_info)
        .unwrap()
        .encode(UiTransactionEncoding::Base64, Some(u8::MAX), true)
        .map_err(|e| anyhow!("{}", e))
}

pub fn cal_pumpfun_price(virtual_sol_reserves: u64, virtual_token_reserves: u64) -> f64 {
    (virtual_sol_reserves as f64 / 10f64.powi(9)) / (virtual_token_reserves as f64 / 10f64.powi(6))
}

pub fn cal_pumpfun_marketcap(price: f64) -> f64 {
    price * 1_000_000_000.0
}

// base_reserve -> meme
// quote_reserve -> WSOL

const WSOL_DECIMALS: u8 = 9;
const TOKEN_DECIMALS: u8 = 6;

pub fn cal_pumpamm_price(
    base_reserves: u64,
    quote_reserves: u64,
) -> f64 {
    let base = base_reserves as f64 / 10f64.powi(TOKEN_DECIMALS as i32);
    let quote = quote_reserves as f64 / 10f64.powi(WSOL_DECIMALS as i32);
    if base == 0.0 {
        return 0.0; 
    }
    quote / base
}

pub fn cal_pumpamm_marketcap_precise( 
    price: f64,
) -> f64 {
    price * 1_000_000_000.0 
}



pub fn get_pumpamm_base_mint_info(create_event: &CreateEvent) -> Vec<(Pubkey, u8)> {
    // let base_token = if pool.base_mint == WSOL {
    //     pool.quote_mint
    // } else {
    //     pool.base_mint
    // };
    // let quote_token = if pool.base_mint == WSOL {
    //     pool.quote_mint
    // } else {
    //     pool.base_mint
    // };

    vec![]
}
pub async fn have_tg_or_x(client: &Client, mint: &str) -> Result<bool> {
    let response = client
        .get(format!(
            "https://frontend-api.pump.fun/coins/{mint}?sync=false"
        ))
        .timeout(Duration::from_millis(300))
        .send()
        .await?;

    if response.status().is_success() {
        let data: Value = response.json().await?;
        if data.get("twitter").is_some()
            || data.get("telegram").is_some()
            || data.get("website").is_some()
        {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn find_bonding_curve(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &["bonding-curve".as_bytes(), mint.as_ref()],
        &PUMPFUN_PROGRAM_ID,
    )
    .0
}

pub fn format_timestamp_to_et(timestamp_ms: u64) -> String {
    let seconds = (timestamp_ms / 1000) as i64;
    let dt = Utc.timestamp_opt(seconds, 0).unwrap();
    let et = dt.with_timezone(&New_York);   
    et.format("%Y-%m-%d %I:%M %p ET").to_string()
}
