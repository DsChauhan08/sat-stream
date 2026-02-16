use color_eyre::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use crate::db;

const OLLAMA_URL: &str = "http://localhost:11434/api/generate";
const MODEL: &str = "qwen2.5:1.5b";

/// A question extracted by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedQuestion {
    question: String,
    option_a: String,
    option_b: String,
    option_c: String,
    option_d: String,
    correct_answer: String,  // "A", "B", "C", or "D"
    explanation: String,
    section: String,         // "math" or "english"
    domain: String,
    sub_domain: String,
    difficulty: u8,          // 1-3
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractedBatch {
    questions: Vec<ExtractedQuestion>,
}

/// Ollama API request
#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    format: String,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

/// Ollama API response
#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
}

/// Check if Ollama is running and the model is available
pub async fn check_ollama() -> Result<bool> {
    let client = reqwest::Client::new();
    let resp = client.get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await;

    match resp {
        Ok(r) => Ok(r.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Pull the model if not available
pub async fn ensure_model() -> Result<String> {
    let client = reqwest::Client::new();

    // Check existing models
    let resp = client.get("http://localhost:11434/api/tags")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    if let Some(models) = resp["models"].as_array() {
        for m in models {
            if let Some(name) = m["name"].as_str() {
                if name.starts_with("qwen2.5:1.5b") {
                    return Ok(format!("Model {} ready", MODEL));
                }
            }
        }
    }

    // Model not found, need to pull
    Ok(format!("Model {} not found. Run: ollama pull {}", MODEL, MODEL))
}

/// Extract questions from a PDF file using Ollama AI
pub async fn extract_from_pdf(pool: &SqlitePool, path: &str) -> Result<usize> {
    let bytes = std::fs::read(path)?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| color_eyre::eyre::eyre!("PDF extract error: {}", e))?;

    let source = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown PDF")
        .to_string();

    // Split text into chunks (~2000 chars each to stay within context window)
    let chunks = split_into_chunks(&text, 2000);
    let mut total = 0;

    for chunk in &chunks {
        // Skip chunks that are too short or look like TOC/headers
        if chunk.len() < 100 || is_likely_noise(chunk) {
            continue;
        }

        match extract_questions_from_chunk(chunk, &source).await {
            Ok(questions) => {
                for q in &questions {
                    if is_valid_question(q) {
                        let _ = db::insert_question(
                            pool,
                            &q.section,
                            &q.domain,
                            &q.sub_domain,
                            &source,
                            q.difficulty as i64,
                            &q.question,
                            &q.option_a,
                            &q.option_b,
                            &q.option_c,
                            &q.option_d,
                            &q.correct_answer,
                            &q.explanation,
                        ).await;
                        total += 1;
                    }
                }
            }
            Err(_) => {
                // Skip chunks that fail — some PDF text is garbled
                continue;
            }
        }
    }

    Ok(total)
}

/// Ask the local LLM to extract SAT questions from a text chunk
async fn extract_questions_from_chunk(text: &str, source: &str) -> Result<Vec<ExtractedQuestion>> {
    let prompt = format!(
        r#"You are an SAT question extraction expert. Extract ALL multiple-choice SAT questions from the following text.
For each question, identify:
- The question text
- Options A, B, C, D
- The correct answer letter (A/B/C/D)
- A brief explanation of why the answer is correct
- Section: "math" or "english"
- Domain: one of "Algebra", "Advanced Math", "Problem Solving & Data Analysis", "Geometry & Trigonometry", "Craft and Structure", "Information and Ideas", "Standard English Conventions", "Expression of Ideas"
- Sub-domain: a more specific topic
- Difficulty: 1 (easy), 2 (medium), 3 (hard)

If the text contains NO SAT questions, return {{"questions": []}}.
If the correct answer is not clear from the text, use your knowledge to determine it.

Return ONLY valid JSON in this exact format:
{{"questions": [{{"question": "...", "option_a": "...", "option_b": "...", "option_c": "...", "option_d": "...", "correct_answer": "A", "explanation": "...", "section": "math", "domain": "Algebra", "sub_domain": "Linear Equations", "difficulty": 2}}]}}

TEXT FROM "{source}":
---
{text}
---

JSON output:"#
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let request = OllamaRequest {
        model: MODEL.to_string(),
        prompt,
        stream: false,
        format: "json".to_string(),
        options: OllamaOptions {
            temperature: 0.1,  // Low temperature for structured output
            num_predict: 4096,
        },
    };

    let resp = client
        .post(OLLAMA_URL)
        .json(&request)
        .send()
        .await?;

    let ollama_resp: OllamaResponse = resp.json().await?;

    // Parse the JSON response
    let parsed: ExtractedBatch = serde_json::from_str(&ollama_resp.response)
        .unwrap_or(ExtractedBatch { questions: vec![] });

    Ok(parsed.questions)
}

/// Validate an extracted question is actually a proper SAT question
fn is_valid_question(q: &ExtractedQuestion) -> bool {
    // Question text must be substantial
    if q.question.len() < 15 {
        return false;
    }

    // Must have all 4 options
    if q.option_a.is_empty() || q.option_b.is_empty()
       || q.option_c.is_empty() || q.option_d.is_empty() {
        return false;
    }

    // Correct answer must be A/B/C/D
    let valid_answers = ["A", "B", "C", "D"];
    if !valid_answers.contains(&q.correct_answer.to_uppercase().as_str()) {
        return false;
    }

    // Section must be math or english
    if q.section != "math" && q.section != "english" {
        return false;
    }

    // Domain must be one of the 8
    let valid_domains = [
        "Algebra", "Advanced Math", "Problem Solving & Data Analysis",
        "Geometry & Trigonometry", "Craft and Structure", "Information and Ideas",
        "Standard English Conventions", "Expression of Ideas",
    ];
    if !valid_domains.contains(&q.domain.as_str()) {
        return false;
    }

    // Skip if it looks like instructions/headers rather than a question
    let lower = q.question.to_lowercase();
    if lower.starts_with("chapter") || lower.starts_with("section")
       || lower.starts_with("part") || lower.contains("table of contents") {
        return false;
    }

    true
}

/// Split text into chunks of approximately `max_chars` characters,
/// breaking at paragraph boundaries when possible
fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in text.split("\n\n") {
        if current.len() + paragraph.len() > max_chars && !current.is_empty() {
            chunks.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(paragraph);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Check if a text chunk is likely noise (TOC, blank pages, headers)
fn is_likely_noise(text: &str) -> bool {
    let lower = text.to_lowercase();

    // Too many short lines = likely a table of contents or index
    let lines: Vec<&str> = text.lines().collect();
    let short_lines = lines.iter().filter(|l| l.trim().len() < 5).count();
    if lines.len() > 5 && short_lines as f64 / lines.len() as f64 > 0.7 {
        return true;
    }

    // Common noise patterns
    let noise_patterns = [
        "table of contents", "copyright", "all rights reserved",
        "isbn", "printed in", "about the author", "acknowledgments",
        "bibliography", "index", "appendix", "answer key",
    ];

    noise_patterns.iter().any(|p| lower.contains(p))
        && !lower.contains("question") && !lower.contains("solve")
}

/// Scan directory for PDFs and extract questions from all of them using AI
pub async fn extract_from_directory(pool: &SqlitePool, dir: &str) -> Result<usize> {
    // First check if Ollama is available
    if !check_ollama().await? {
        return Err(color_eyre::eyre::eyre!(
            "Ollama is not running. Start it with: ollama serve\n\
             Then pull the model: ollama pull {}", MODEL
        ));
    }

    // Check if model is available
    let model_status = ensure_model().await?;
    if model_status.contains("not found") {
        return Err(color_eyre::eyre::eyre!("{}", model_status));
    }

    let mut total = 0;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext.to_ascii_lowercase() == "pdf" {
                let path_str = path.to_string_lossy().to_string();
                match extract_from_pdf(pool, &path_str).await {
                    Ok(count) => {
                        total += count;
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to extract from {}: {}", path_str, e);
                    }
                }
            }
        }
    }

    Ok(total)
}

/// Fallback: regex-based extraction (no AI needed)
pub async fn extract_regex_fallback(pool: &SqlitePool, path: &str) -> Result<usize> {
    let bytes = std::fs::read(path)?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| color_eyre::eyre::eyre!("PDF extract error: {}", e))?;

    let source = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown PDF")
        .to_string();

    let mut count = 0;

    // Pattern: question text followed by (A), (B), (C), (D) options
    let mc_pattern = Regex::new(
        r"(?s)(\d+[\.\)]\s+.+?)\s*\(?A\)?\s*(.+?)\s*\(?B\)?\s*(.+?)\s*\(?C\)?\s*(.+?)\s*\(?D\)?\s*(.+?)(?:\n\n|\z|\d+[\.\)])"
    )?;

    for cap in mc_pattern.captures_iter(&text) {
        let q_text = clean_text(&cap[1]);
        let opt_a = clean_text(&cap[2]);
        let opt_b = clean_text(&cap[3]);
        let opt_c = clean_text(&cap[4]);
        let opt_d = clean_text(&cap[5]);

        if q_text.len() < 10 || opt_a.is_empty() || opt_b.is_empty() {
            continue;
        }

        let (section, domain) = classify_question(&q_text);

        db::insert_question(
            pool, &section, &domain, "General", &source,
            2, &q_text, &opt_a, &opt_b, &opt_c, &opt_d,
            "A", "Extracted from PDF via pattern matching."
        ).await?;

        count += 1;
    }

    Ok(count)
}

/// Classify question into section/domain based on keywords
fn classify_question(text: &str) -> (String, String) {
    let lower = text.to_lowercase();

    let math_keywords = [
        "equation", "solve", "graph", "slope", "triangle", "circle",
        "area", "volume", "angle", "sin", "cos", "polynomial",
        "quadratic", "linear", "exponent", "integer", "fraction",
        "percent", "ratio", "probability", "median", "mean",
        "function", "value of x", "expression", "inequality",
        "factor", "simplify", "calculate",
    ];

    if math_keywords.iter().any(|kw| lower.contains(kw)) {
        let geo_kw = ["triangle", "circle", "area", "volume", "angle", "sin", "cos", "tan"];
        let data_kw = ["probability", "median", "mean", "standard deviation", "percent", "ratio"];
        let adv_kw = ["quadratic", "polynomial", "exponent", "logarithm", "factor", "vertex"];

        if geo_kw.iter().any(|kw| lower.contains(kw)) {
            ("math".into(), "Geometry & Trigonometry".into())
        } else if data_kw.iter().any(|kw| lower.contains(kw)) {
            ("math".into(), "Problem Solving & Data Analysis".into())
        } else if adv_kw.iter().any(|kw| lower.contains(kw)) {
            ("math".into(), "Advanced Math".into())
        } else {
            ("math".into(), "Algebra".into())
        }
    } else {
        let grammar_kw = ["verb", "pronoun", "comma", "semicolon", "punctuation", "tense"];
        let expression_kw = ["transition", "concise", "revision", "tone", "paragraph"];

        if grammar_kw.iter().any(|kw| lower.contains(kw)) {
            ("english".into(), "Standard English Conventions".into())
        } else if expression_kw.iter().any(|kw| lower.contains(kw)) {
            ("english".into(), "Expression of Ideas".into())
        } else if lower.contains("passage") || lower.contains("author") {
            ("english".into(), "Craft and Structure".into())
        } else {
            ("english".into(), "Information and Ideas".into())
        }
    }
}

/// Clean extracted text
fn clean_text(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect();
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(cleaned.trim(), " ").to_string()
}
