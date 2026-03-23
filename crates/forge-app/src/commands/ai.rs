#[tauri::command]
pub async fn ai_chat(message: String) -> Result<String, String> {
    // TODO: proxy through forge-ai ClaudeProvider
    Ok(format!(
        "AI analysis for your query: '{}'\n\n\
        This is a mock response. Configure AI provider in Settings \
        to get real analysis from Claude/GPT/Gemini.",
        message
    ))
}
