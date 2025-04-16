use std::env;

use once_cell::sync::Lazy;
use solana_program::pubkey;
use solana_sdk::pubkey::Pubkey;

pub static GRPC: Lazy<String> = Lazy::new(|| env::var("GRPC_URL").unwrap());
pub static RPC: Lazy<String> = Lazy::new(|| env::var("RPC_URL").unwrap());



pub static REDIS_URL: Lazy<String> = Lazy::new(|| env::var("REDIS_URL").unwrap());

pub static MARKET_CAP: Lazy<f32> = Lazy::new(|| {
    env::var("MARKET_CAP")
        .unwrap()
        .parse::<f32>()
        .unwrap_or(50000.0)
}); 


// program related
pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
pub const SYSTEM_RENT_PROGRAM_ID: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const ASSOC_TOKEN_ACC_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const EVENT_AUTHORITY: Pubkey = pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const KEY_PREFIX: &'static str = "token:info:";

// pumpfun
pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMPFUN_GLOBAL: Pubkey = pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUMPFUN_FEE_RECIPIENT: Pubkey = pubkey!("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM");
pub const INIT_SOL_REVERSES: u64 = 30_000_000_000;
pub const INIT_TOKEN_REVERSES: u64 = 1_073_000_191_000_000;
pub const INIT_PRICE: f32 = (INIT_SOL_REVERSES as f32 / 1e9) / (INIT_TOKEN_REVERSES as f32 / 1e6);
pub const PUMPFUN_TOTAL_SUPPLY: u64 = 1_000_000_000_000_000;

pub const PUMPAMM_PROGRAM_ID: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
 
// scalars
pub const MINUTES: u64 = 60 * 1000;
pub const SECONDS: u64 = 1000;

// Tokens
pub const WSOL: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
 
// Time
pub const NEW_COIN_MIN_TIME: u64 = 10 * 60 * 1000; // 10分钟 (以毫秒为单位)
pub const NEW_COIN_MAX_TIME: u64 = 15 * 60 * 1000; // 15分钟 (以毫秒为单位)