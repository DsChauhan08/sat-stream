use serde::{Deserialize, Serialize};

/// Gemini AI client for hints, explanations, and question generation
#[derive(Clone)]
pub struct AiClient {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl AiClient {
    pub fn clone_for_spawn(&self) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Clone, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentResp>,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResp {
    parts: Option<Vec<GeminiPartResp>>,
}

#[derive(Debug, Deserialize)]
struct GeminiPartResp {
    text: Option<String>,
}

/// AI response types
#[derive(Debug, Clone)]
pub enum AiResponse {
    Success(String),
    Offline(String),
    Error(String),
}

impl AiClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    #[allow(dead_code)]
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Get a hint for a question (without revealing the answer)
    pub async fn get_hint(&self, question_text: &str, domain: &str) -> AiResponse {
        let prompt = format!(
            "You are a SAT tutor. A student is stuck on this question and needs a hint. \
             Do NOT reveal the answer. Give a strategic hint that guides their thinking.\n\n\
             Domain: {}\n\
             Question: {}\n\n\
             Provide a concise hint (2-3 sentences max) that helps them figure out the approach \
             without giving away the answer.",
            domain, question_text
        );
        self.call_gemini(&prompt).await
    }

    /// Explain why an answer is correct after the student got it wrong
    pub async fn explain_answer(
        &self,
        question_text: &str,
        correct_answer: &str,
        student_answer: &str,
        domain: &str,
    ) -> AiResponse {
        let prompt = format!(
            "You are a SAT tutor. A student got this question wrong. Explain clearly and concisely.\n\n\
             Domain: {}\n\
             Question: {}\n\
             Student's Answer: {}\n\
             Correct Answer: {}\n\n\
             Explain in 3-4 sentences: (1) Why the correct answer is right, \
             (2) Why the student's answer is wrong, (3) The key concept to remember.",
            domain, question_text, student_answer, correct_answer
        );
        self.call_gemini(&prompt).await
    }

    /// Generate similar practice questions
    #[allow(dead_code)]
    pub async fn generate_questions(
        &self,
        domain: &str,
        sub_domain: &str,
        difficulty: &str,
    ) -> AiResponse {
        let prompt = format!(
            "Generate 3 SAT-style multiple choice questions.\n\n\
             Section: {}\n\
             Sub-topic: {}\n\
             Difficulty: {}\n\n\
             For each question, provide:\n\
             - Question text\n\
             - Four options (A, B, C, D)\n\
             - Correct answer\n\
             - Brief explanation\n\n\
             Format each clearly with numbers (1, 2, 3) and line breaks.",
            domain, sub_domain, difficulty
        );
        self.call_gemini(&prompt).await
    }

    async fn call_gemini(&self, prompt: &str) -> AiResponse {
        let api_key = match &self.api_key {
            Some(key) => key,
            None => return AiResponse::Offline(
                "AI not configured. Set your Gemini API key in Settings (press 'K').".to_string()
            ),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
            api_key
        );

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
        };

        match self.client.post(&url).json(&request).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    return AiResponse::Error(format!("API error: {}", response.status()));
                }
                match response.json::<GeminiResponse>().await {
                    Ok(data) => {
                        let text = data.candidates
                            .and_then(|c| c.into_iter().next())
                            .and_then(|c| c.content)
                            .and_then(|c| c.parts)
                            .and_then(|p| p.into_iter().next())
                            .and_then(|p| p.text)
                            .unwrap_or_else(|| "No response generated.".to_string());
                        AiResponse::Success(text)
                    }
                    Err(e) => AiResponse::Error(format!("Parse error: {}", e)),
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    AiResponse::Offline("Request timed out. Check your internet connection.".to_string())
                } else if e.is_connect() {
                    AiResponse::Offline("Cannot connect. You appear to be offline.".to_string())
                } else {
                    AiResponse::Error(format!("Network error: {}", e))
                }
            }
        }
    }
}
