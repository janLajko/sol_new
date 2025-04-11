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

/// Generate a brief summary about a pump.fun token using Gemini API
pub async fn generate_token_summary(token: &TokenInfo) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let api_url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent";
    let api_key = std::env::var("AI_API_KEY").expect("AI_API_KEY not found");
    
    // Create the prompt for Gemini
    let prompt = format!(
        "Provide a concise two-sentence summary about the '{}' ({}) token from pump.fun. Include relevant information about its purpose and unique features. The token's website is {}. Keep your response brief and factual.",
        token.name, token.symbol, token.url
    );
    
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
            x_content: "".to_string(),
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