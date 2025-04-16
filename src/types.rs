use std::str::FromStr;

use anyhow::anyhow;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{bs58, pubkey::Pubkey};
use solana_transaction_status::{UiCompiledInstruction, UiInstruction};

// PUMPFUN EVENT
const PUMPFUN_CREATE_EVENT: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
const PUMPFUN_COMPLETE_EVENT: [u8; 8] = [95, 114, 97, 156, 212, 46, 152, 8];
const PUMPFUN_TRADE_EVENT: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];

// AMM EVENT
pub const PUMPAMM_BUY_EVENT: [u8; 8] = [103, 244, 82, 31, 44, 245, 119, 119];
pub const PUMPAMM_SELL_EVENT: [u8; 8] = [62, 47, 55, 10, 165, 3, 220, 42];
pub const PUMPAMM_DEPOSIT_EVENT: [u8; 8] = [120, 248, 61, 83, 31, 142, 107, 144];
pub const PUMPAMM_WITHDRAW_EVENT: [u8; 8] = [22, 9, 133, 26, 160, 44, 71, 192];
pub const PUMPAMM_CREATE_POOL_EVENT: [u8; 8] = [177, 49, 12, 210, 160, 118, 167, 116];


#[derive(Debug, Clone)]
pub enum TargetEvent {
    PumpfunBuy(TradeEvent),
    PumpfunSell(TradeEvent),
    PumpfunCreate(CreateEvent),
    PumpfunComplete(CompleteEvent),
    PumpammBuy(AMMBuyEvent),
    PumpammSell(AMMSellEvent),
    PumpammDeposit(AMMDepositEvent),
    PumpammWithdraw(AMMWithdrawEvent),
    PumpammCreatePool(AMMCreatePoolEvent),
}
 
impl TryFrom<UiInstruction> for TargetEvent {
    type Error = anyhow::Error;

    fn try_from(inner_instruction: UiInstruction) -> Result<Self, Self::Error> {
        match inner_instruction {
            solana_transaction_status::UiInstruction::Compiled(ui_compiled_instruction) => {
                if let Some(create) =
                    CreateEvent::try_from_compiled_instruction(&ui_compiled_instruction)
                {
                    return Ok(TargetEvent::PumpfunCreate(create));
                }
                if let Some(complete) =
                    CompleteEvent::try_from_compiled_instruction(&ui_compiled_instruction)
                {
                    return Ok(Self::PumpfunComplete(complete));
                }
                if let Some(trade) =
                    TradeEvent::try_from_compiled_instruction(&ui_compiled_instruction)
                {
                    if trade.is_buy {
                        return Ok(TargetEvent::PumpfunBuy(trade));
                    } else {
                        return Ok(TargetEvent::PumpfunSell(trade));
                    }
                }
                if let Some(amm_buy) = AMMBuyEvent::try_from_compiled_instruction(&ui_compiled_instruction) {
                    return Ok(TargetEvent::PumpammBuy(amm_buy));
                }
                if let Some(amm_sell) = AMMSellEvent::try_from_compiled_instruction(&ui_compiled_instruction) {
                    return Ok(TargetEvent::PumpammSell(amm_sell));
                }
                if let Some(amm_deposit) = AMMDepositEvent::try_from_compiled_instruction(&ui_compiled_instruction) {
                    return Ok(TargetEvent::PumpammDeposit(amm_deposit));
                }
                if let Some(amm_withdraw) = AMMWithdrawEvent::try_from_compiled_instruction(&ui_compiled_instruction) {
                    return Ok(TargetEvent::PumpammWithdraw(amm_withdraw));
                }
                if let Some(amm_create_pool) = AMMCreatePoolEvent::try_from_compiled_instruction(&ui_compiled_instruction) {
                    return Ok(TargetEvent::PumpammCreatePool(amm_create_pool));
                }
            }
            _ => {}
        }
        return Err(anyhow!("failed to convert to target tx"));
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
}

impl CreateEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<CreateEvent> {
        let data = match bs58::decode(ui_compiled_instruction.data.clone()).into_vec() {
            Ok(d) => d,
            Err(_) => return None,
        }; 
        
        if data.len() < 16 {
            return None;
        }
        
        if data[8..16].eq(&PUMPFUN_CREATE_EVENT) {
            if let Ok(event) = CreateEvent::try_from_slice(&data[16..]) {
                // println!("create event: {:?}", event);
                return Some(event);
            }
            Self::try_manual_parse(&data)
        } else {
            return None;
        }
    }
    
    fn try_manual_parse(data: &[u8]) -> Option<Self> {
        // println!("try_manual_parse: {:?}", data);
        if data.len() < 100 {
            return None;
        }
        
        let mut offset = 16;
        
        let (name, new_offset) = Self::parse_string(data, offset)?;
        offset = new_offset;
        
        let (symbol, new_offset) = Self::parse_string(data, offset)?;
        offset = new_offset;
        
        let (uri, new_offset) = Self::parse_string(data, offset)?;
        offset = new_offset;
        
        if offset + 32 * 3 > data.len() {
            return None;
        }
        
        let mint = match Pubkey::try_from_slice(&data[offset..offset + 32]) {
            Ok(m) => m,
            Err(_) => return None,
        };
        offset += 32;
        
        let bonding_curve = match Pubkey::try_from_slice(&data[offset..offset + 32]) {
            Ok(bc) => bc,
            Err(_) => return None,
        };
        offset += 32;
        
        let user = match Pubkey::try_from_slice(&data[offset..offset + 32]) {
            Ok(u) => u,
            Err(_) => return None,
        };
        
        Some(Self {
            name,
            symbol,
            uri,
            mint,
            bonding_curve,
            user,
        })
    }
    
    fn parse_string(data: &[u8], offset: usize) -> Option<(String, usize)> {
  
        if offset + 4 > data.len() {
            return None;
        }
        
        let len = u32::from_le_bytes([
            data[offset], data[offset + 1], 
            data[offset + 2], data[offset + 3]
        ]) as usize;
        
        if offset + 4 + len > data.len() {
            return None;
        }
        
        let string_content = match String::from_utf8(data[offset + 4..offset + 4 + len].to_vec()) {
            Ok(s) => s,
            Err(_) => return None,
        };
        
        Some((string_content, offset + 4 + len))
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct CompleteEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}

impl CompleteEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<CompleteEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPFUN_COMPLETE_EVENT) {
            match CompleteEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}


#[derive(Debug, BorshSerialize, Clone, BorshDeserialize)]
pub struct BuyArgs {
    pub amount: u64,
    pub max_sol_cost: u64,
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64, 
    pub real_token_reserves: u64,
}

impl TradeEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<TradeEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPFUN_TRADE_EVENT) {
            match TradeEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct AMMBuyEvent {
    pub timestamp: i64,
    pub base_amount_out: u64,
    pub max_quote_amount_in: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_in: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_in_with_lp_fee: u64,
    pub user_quote_amount_in: u64,
    pub pool: Pubkey, 
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub protocol_fee_recipient: Pubkey,
    pub protocol_fee_recipient_token_account: Pubkey,
} 

impl AMMBuyEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<AMMBuyEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPAMM_BUY_EVENT) {
            match AMMBuyEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct AMMSellEvent {
    pub timestamp: i64,
    pub base_amount_in: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64, 
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_out: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_out_without_lp_fee: u64,
    pub user_quote_amount_out: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub protocol_fee_recipient: Pubkey,
    pub protocol_fee_recipient_token_account: Pubkey,
}

impl AMMSellEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<AMMSellEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPAMM_SELL_EVENT) {
            match AMMSellEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct AMMDepositEvent {
    pub timestamp: i64,
    pub lp_token_amount_out: u64,
    pub max_base_amount_in: u64,
    pub max_quote_amount_in: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub lp_mint_supply: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub user_pool_token_account: Pubkey,
}

impl AMMDepositEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<AMMDepositEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPAMM_DEPOSIT_EVENT) {
            match AMMDepositEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct AMMWithdrawEvent {
    pub timestamp: i64,
    pub lp_token_amount_in: u64,
    pub min_base_amount_out: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub base_amount_out: u64,
    pub quote_amount_out: u64,
    pub lp_mint_supply: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub user_pool_token_account: Pubkey,
}

impl AMMWithdrawEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<AMMWithdrawEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPAMM_WITHDRAW_EVENT) {
            match AMMWithdrawEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct AMMCreatePoolEvent {
    pub timestamp: i64,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_mint_decimals: u8,
    pub quote_mint_decimals: u8,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub pool_base_amount: u64,
    pub pool_quote_amount: u64,
    pub minimum_liquidity: u64,
    pub initial_liquidity: u64,
    pub lp_token_amount_out: u64,
    pub pool_bump: u8,
    pub pool: Pubkey,
    pub lp_mint: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
}

impl AMMCreatePoolEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<AMMCreatePoolEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPAMM_CREATE_POOL_EVENT) {
            match AMMCreatePoolEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}   


#[tokio::test]
async fn test() {
    let data = "2K7nL28PxCW8ejnyCeuMpbYAmP2pnuyvkxEQgp79nsKJzbKfMq82LAVFjwFY1xYhKmuaA8H3M5xLfFnF85Xbai9s9aaCyDETZgWMQJayFp8t1HM9ihUxb1TCcsXYVsNKDqaGANFoxSEAPLvpAXJVQHTNyAMxFcgM9s3knpLcDTYtGe7Ufq3WZ9kvAGdd";
    let data = bs58::decode(data.as_bytes()).into_vec().unwrap();
    println!("data {:?}", data);
    let result = TradeEvent::try_from_slice(&data[16..]).unwrap();
    println!("result {:?}", result);
}

