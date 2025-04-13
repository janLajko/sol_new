use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Struct to hold detailed token information
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub url: String,
    pub x_content: String,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: String,
}

pub async fn generate_token_summary(token: &TokenInfo) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let api_url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent";
    let api_key = std::env::var("AI_API_KEY").expect("AI_API_KEY not found");
    
    // Create a flexible prompt that can work with or without X content
    let prompt = if token.x_content.is_empty() {
        format!(
            "Provide a two-sentence investment analysis of the '{}' ({}) token based solely on its ticker symbol characteristics. Offer a concise, objective perspective on its potential market positioning and dynamics without mentioning investment risks.",
            token.name, token.symbol
        )
    } else {
        format!(
            "Provide a two-sentence investment analysis of the '{}' ({}) token, using both its ticker symbol and X (Twitter) content: '{}'. Offer a concise, objective perspective on its brand positioning and market dynamics without including risk disclaimers.",
            token.name, token.symbol, token.x_content
        )
    };
    
    // Prepare the request
    let request = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: prompt }],
        }],
    };
    
    // Make the API call
    let response = client
        .post(&format!("{}?key={}", api_url, api_key))
        .json(&request)
        .send()
        .await?
        .json::<GeminiResponse>()
        .await?;
    
    // Extract and return the summary
    if let Some(candidate) = response.candidates.first() {
        if let Some(part) = candidate.content.parts.first() {
            return Ok(part.text.trim().to_string());
        }
    }
    
    Err("Failed to generate summary".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_token_summary_real_request() {
        dotenv::dotenv().ok();
        // Create a test token
        let token = TokenInfo {
            name: "PEPE".to_string(),
            symbol: "PEPE".to_string(),
            url: "https://pepe.pump.fun".to_string(),
            x_content: "PEPE is a token, it target to fire 1 M".to_string(),
        };

        // Call the actual function with a real API request
        let result = generate_token_summary(&token).await;

        // Verify the result is Ok and contains some text
        assert!(result.is_ok(), "API request failed: {:?}", result.err());

        let summary = result.unwrap();

        // Basic validation of the response
        assert!(!summary.is_empty(), "Summary should not be empty");
        println!("Generated summary: {}", summary);

        // Optional: Additional assertions about the content
        assert!(summary.contains("PEPE") || summary.contains("pepe"),
                "Summary should mention the token name");
    }
}