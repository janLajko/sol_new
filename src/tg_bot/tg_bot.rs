use crate::tg_bot::tg_bot_type::{
    get_send_message_parse_mode_str, 
    BotInstance, 
    ErrorResult, 
    SendMessageOption, 
    SendMessageParseMode, 
    StatusCode, 
    TelegramErrorResult
};
use url::Url;
use reqwest::Client;
use serde_json::json;
use anyhow::Result;

/// Struct to hold detailed token information
#[derive(Debug, Clone)]
pub struct TokenDetails {
    pub mint_address: String,
    pub name: String,
    pub symbol: String,
    pub url: String,
    pub ai_analysis: String,
    pub market_cap: String,
    pub creator: String, 
    pub launch_time: String,
}

impl BotInstance {
    /// Create a new `BotInstance`.
    ///
    /// # Arguments
    /// * `bot_token` - a string of bot token
    /// * `chat_id` - a chat id string to send message
    pub fn new(bot_token: String, chat_id: String) -> BotInstance {
        BotInstance { bot_token, chat_id }
    }

    /// Send a message asynchronously to Telegram
    pub async fn send_message_async(
        &self,
        msg: &str,
        options: Option<SendMessageOption>,
    ) -> Result<(), ErrorResult> {
        let raw_url_str = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );
        let url = Url::parse(&raw_url_str).map_err(|e| {
            ErrorResult {
                code: StatusCode::ErrorInternalError as u16,
                msg: format!("Error parsing Url; err={}", e),
            }
        })?;
    
        let parse_mode = options
            .as_ref()
            .and_then(|opt| opt.parse_mode.clone())
            .map(get_send_message_parse_mode_str);
    
        let mut json_body = json!({
            "chat_id": &self.chat_id,
            "text": msg,
        });
    
        if let Some(mode) = parse_mode {
            json_body["parse_mode"] = json!(mode);
        }
    
        let client = Client::new();
        let response = client.post(url).json(&json_body).send().await?;
    
        if response.status().is_success() {
            Ok(())
        } else {
            let telegram_error: TelegramErrorResult = response.json().await.map_err(|_| {
                ErrorResult {
                    code: StatusCode::ErrorInternalError as u16,
                    msg: "Error converting telegram error response to json".to_string(),
                }
            })?;
            Err(ErrorResult {
                code: StatusCode::ErrorInternalError as u16,
                msg: telegram_error.description,
            })
        }
    }

    /// Send a detailed coin alert for a new token on Pump.fun
    pub async fn send_coin_alert(
        &self,
        token_details: &TokenDetails,
    ) -> Result<(), ErrorResult> {
        // Build a message in MarkdownV2 format with rich token information
        let markdown_message = format!(
            r#"🚀 *New Pump\.fun Token Alert\!* 🚀

*Token:* `{token_name}` *\({symbol}\)*
*Mint Address:* `{mint_address}`

📈 *Token Metrics:* 
• *Market Cap:* `${market_cap}`
• *Created By:* `{creator}`
• *Launch Time:* `{launch_time}`

🌐 *Links:*
• [PumpFun Chart](https://pump.fun/{mint_address})

💡 *AI Analysis:* 
{ai_analysis}

⚠️ *Disclaimer:* DYOR\. High Risk Investment\!"#,
            token_name = escape_markdown(&token_details.name),
            symbol = escape_markdown(&token_details.symbol),
            mint_address = escape_markdown(&token_details.mint_address),
            market_cap = escape_markdown(&token_details.market_cap),
            creator = escape_markdown(&token_details.creator),
            launch_time = escape_markdown(&token_details.launch_time),
            ai_analysis = escape_markdown(&token_details.ai_analysis)
        );

        // Check message length and send
        if markdown_message.len() > 4096 {
            // If the message is too long, split it into multiple messages
            for (i, chunk) in markdown_message.chars().collect::<Vec<char>>().chunks(4000).enumerate() {
                let chunk_str: String = chunk.iter().collect();
                let chunk_message = if i == 0 {
                    chunk_str
                } else {
                    format!("*Continued:*\n\n{}", chunk_str)
                };
                self.send_message_async(&chunk_message, Some(SendMessageOption { parse_mode: Some(SendMessageParseMode::MarkdownV2) })).await?;
            }
        } else {
            // Send a single message
            self.send_message_async(&markdown_message, Some(SendMessageOption { parse_mode: Some(SendMessageParseMode::MarkdownV2) })).await?;
        }

        Ok(())
    }
}

/// Escaping special characters in MarkdownV2
fn escape_markdown(text: &str) -> String {
    text.chars().map(|c| {
        match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' |
            '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.' | '!' => format!("\\{}", c),
            '\\' => String::from("\\\\"),
            _ => c.to_string(),
        }
    }).collect()
}

/// Create a Telegram bot instance
pub fn create_instance(bot_token: &str, chat_id: &str) -> BotInstance {
    BotInstance { 
        bot_token: bot_token.to_string(), 
        chat_id: chat_id.to_string() 
    }
}

/// Get a preconfigured Telegram bot instance
pub fn get_instance() -> BotInstance {
    let keys: (String, String) = (
        "7985716563:AAE3RtrPsEnqBHqxFZh8HYdw4qig8n37Ugk".to_string(), 
        "-4704509264".to_string()
    );
    create_instance(&keys.0, &keys.1)
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use crate::tg_bot::tg_bot::get_instance;

    #[tokio::test]
    async fn test_send_coin_alert() -> Result<()> {
        let instance = get_instance();
        
        let token_details = TokenDetails {
            mint_address: "7Gx9DgQnTxnKNuBjDT5LNDRmfJz2kZRjGBKvDQC1Lr1z".to_string(),
            name: "CoolMemeToken".to_string(),
            symbol: "CMT".to_string(),
            url: "https://pump.fun/token".to_string(),
            ai_analysis: "This token shows potential for growth due to its unique market positioning.".to_string(),
            market_cap: "50,000".to_string(),
            creator: "0x1234...5678".to_string(),
            launch_time: "2024-04-11 12:00 UTC".to_string(),
        };

        instance.send_coin_alert(&token_details).await.expect("send_coin_alert failed");
        
        Ok(())
    }
}