use color_eyre::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::db;

/// Default model filename to look for
const DEFAULT_MODEL: &str = "qwen2.5-1.5b-instruct-q4_k_m.gguf";

/// Config for the extraction pipeline
pub struct ExtractConfig {
    /// Path to llama-cli binary
    pub llama_cli: PathBuf,
    /// Path to .gguf model file
    pub model_path: PathBuf,
    /// Number of GPU layers (-1 = all)
    pub n_gpu_layers: i32,
    /// Context size
    pub ctx_size: u32,
}

impl ExtractConfig {
    /// Auto-detect llama-cli and model from common locations
    pub fn auto_detect() -> Result<Self> {
        let llama_cli = find_llama_cli()
            .ok_or_else(|| color_eyre::eyre::eyre!(
                "llama-cli not found. Install llama.cpp:\n\
                 \n  # Option 1: Build from source\n\
                 git clone https://github.com/ggerganov/llama.cpp\n\
                 cd llama.cpp && cmake -B build && cmake --build build --config Release\n\
                 \n  # Option 2: Download release\n\
                 # https://github.com/ggerganov/llama.cpp/releases\n\
                 \n  Then ensure 'llama-cli' is in your PATH."
            ))?;

        let model_path = find_model()
            .ok_or_else(|| color_eyre::eyre::eyre!(
                "No .gguf model found. Download one:\n\
                 \n  # Recommended: Qwen2.5-1.5B-Instruct (Q4_K_M, ~1GB)\n\
                 wget https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf\n\
                 \n  Place the .gguf file in ~/.local/share/sat-stream/models/ or the current directory."
            ))?;

        Ok(Self {
            llama_cli,
            model_path,
            n_gpu_layers: -1,
            ctx_size: 4096,
        })
    }
}

/// Find llama-cli binary in common locations
fn find_llama_cli() -> Option<PathBuf> {
    // Check PATH first
    if let Ok(output) = Command::new("which").arg("llama-cli").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }

    // Common install locations
    let candidates = [
        dirs::home_dir().map(|h| h.join("llama.cpp/build/bin/llama-cli")),
        dirs::home_dir().map(|h| h.join(".local/bin/llama-cli")),
        Some(PathBuf::from("/usr/local/bin/llama-cli")),
        Some(PathBuf::from("/usr/bin/llama-cli")),
        // Also check for the old name "main"
        dirs::home_dir().map(|h| h.join("llama.cpp/build/bin/main")),
    ];

    for candidate in candidates.iter().flatten() {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    None
}

/// Find a .gguf model file in common locations
fn find_model() -> Option<PathBuf> {
    let search_dirs: Vec<PathBuf> = vec![
        // sat-stream model directory
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sat-stream/models"),
        // Current working directory
        std::env::current_dir().unwrap_or_default(),
        // Home directory models folder
        dirs::home_dir().unwrap_or_default().join("models"),
        dirs::home_dir().unwrap_or_default().join(".cache/lm-studio/models"),
        dirs::home_dir().unwrap_or_default().join("llama.cpp/models"),
    ];

    // First pass: look for the recommended model by name
    for dir in &search_dirs {
        let preferred = dir.join(DEFAULT_MODEL);
        if preferred.exists() {
            return Some(preferred);
        }
    }

    // Second pass: any .gguf file
    for dir in &search_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "gguf").unwrap_or(false) {
                    return Some(p);
                }
            }
        }
    }

    None
}

// ─── PDF Text Extraction ───────────────────────────────────────────────

/// Extract text from PDF using poppler's pdftotext (much better than Rust crates)
fn extract_pdf_text(path: &str) -> Result<String> {
    let output = Command::new("pdftotext")
        .arg("-layout")    // Preserve layout for better question parsing
        .arg(path)
        .arg("-")          // Output to stdout
        .output()
        .map_err(|e| color_eyre::eyre::eyre!(
            "Failed to run pdftotext: {}. Install: sudo dnf install poppler-utils", e
        ))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(color_eyre::eyre::eyre!("pdftotext failed: {}", err));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ─── LLM Inference ─────────────────────────────────────────────────────

/// Call llama-cli to run inference with the given prompt
fn run_llama(config: &ExtractConfig, prompt: &str) -> Result<String> {
    let output = Command::new(&config.llama_cli)
        .arg("-m").arg(&config.model_path)
        .arg("-p").arg(prompt)
        .arg("-n").arg("4096")           // Max tokens to generate
        .arg("-c").arg(config.ctx_size.to_string())
        .arg("--temp").arg("0.1")        // Low temp for structured output
        .arg("--repeat-penalty").arg("1.1")
        .arg("-ngl").arg(config.n_gpu_layers.to_string())
        .arg("--no-display-prompt")      // Don't echo prompt back
        .arg("--log-disable")            // Disable logging to stderr
        .output()
        .map_err(|e| color_eyre::eyre::eyre!("Failed to run llama-cli: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        // Filter out common non-error messages from llama.cpp
        let filtered: String = err.lines()
            .filter(|l| !l.contains("llama_") && !l.contains("ggml_") && !l.starts_with("main:"))
            .collect::<Vec<_>>()
            .join("\n");
        if !filtered.trim().is_empty() {
            return Err(color_eyre::eyre::eyre!("llama-cli error: {}", filtered));
        }
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ─── Question Extraction ───────────────────────────────────────────────

/// A question extracted by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedQuestion {
    question: String,
    option_a: String,
    option_b: String,
    option_c: String,
    option_d: String,
    correct_answer: String,
    explanation: String,
    section: String,
    domain: String,
    sub_domain: String,
    difficulty: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractedBatch {
    questions: Vec<ExtractedQuestion>,
}

/// Build the extraction prompt for a text chunk
fn build_prompt(text: &str, source: &str) -> String {
    format!(
r#"<|im_start|>system
You extract SAT multiple-choice questions from text. Return ONLY valid JSON.
For each question found, identify: question text, options A/B/C/D, correct answer (A/B/C/D), brief explanation, section (math/english), domain (one of: Algebra, Advanced Math, Problem Solving & Data Analysis, Geometry & Trigonometry, Craft and Structure, Information and Ideas, Standard English Conventions, Expression of Ideas), sub_domain, difficulty (1/2/3).
If no questions exist in the text, return {{"questions":[]}}.
<|im_end|>
<|im_start|>user
Extract all SAT questions from this text from "{source}":

{text}

Return JSON: {{"questions":[{{"question":"...","option_a":"...","option_b":"...","option_c":"...","option_d":"...","correct_answer":"A","explanation":"...","section":"math","domain":"Algebra","sub_domain":"Linear Equations","difficulty":2}}]}}
<|im_end|>
<|im_start|>assistant
"#)
}

/// Extract JSON from LLM output (handles markdown fences and extra text)
fn parse_json_response(response: &str) -> Vec<ExtractedQuestion> {
    // Try to find JSON in the response
    let trimmed = response.trim();

    // Try direct parse
    if let Ok(batch) = serde_json::from_str::<ExtractedBatch>(trimmed) {
        return batch.questions;
    }

    // Try to extract JSON from markdown code block
    let json_block = Regex::new(r#"(?s)```(?:json)?\s*(\{.+?\})\s*```"#).ok();
    if let Some(re) = json_block {
        if let Some(cap) = re.captures(trimmed) {
            if let Ok(batch) = serde_json::from_str::<ExtractedBatch>(&cap[1]) {
                return batch.questions;
            }
        }
    }

    // Try to find a JSON object anywhere in the response
    let json_obj = Regex::new(r#"(?s)(\{[^{}]*"questions"\s*:\s*\[.+?\]\s*\})"#).ok();
    if let Some(re) = json_obj {
        if let Some(cap) = re.captures(trimmed) {
            if let Ok(batch) = serde_json::from_str::<ExtractedBatch>(&cap[1]) {
                return batch.questions;
            }
        }
    }

    vec![]
}

/// Validate an extracted question
fn is_valid_question(q: &ExtractedQuestion) -> bool {
    if q.question.len() < 15 { return false; }
    if q.option_a.is_empty() || q.option_b.is_empty()
       || q.option_c.is_empty() || q.option_d.is_empty() { return false; }

    let valid_answers = ["A", "B", "C", "D"];
    if !valid_answers.contains(&q.correct_answer.to_uppercase().as_str()) { return false; }
    if q.section != "math" && q.section != "english" { return false; }

    let valid_domains = [
        "Algebra", "Advanced Math", "Problem Solving & Data Analysis",
        "Geometry & Trigonometry", "Craft and Structure", "Information and Ideas",
        "Standard English Conventions", "Expression of Ideas",
    ];
    if !valid_domains.contains(&q.domain.as_str()) { return false; }

    let lower = q.question.to_lowercase();
    if lower.starts_with("chapter") || lower.starts_with("section")
       || lower.contains("table of contents") { return false; }

    true
}

// ─── Chunking ──────────────────────────────────────────────────────────

/// Split text into chunks, breaking at question boundaries when possible
/// Split text into chunks based on page breaks (form feeds)
fn split_into_chunks(text: &str, _max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();

    // Split by form feed (page break)
    for page in text.split('\u{000C}') {
        let cleaned = page.trim();
        if !cleaned.is_empty() {
            chunks.push(cleaned.to_string());
        }
    }

    chunks
}

/// Check if a text chunk is likely noise (TOC, blank pages, headers)
fn is_likely_noise(text: &str) -> bool {
    let lower = text.to_lowercase();

    let lines: Vec<&str> = text.lines().collect();
    let short_lines = lines.iter().filter(|l| l.trim().len() < 5).count();
    if lines.len() > 5 && short_lines as f64 / lines.len() as f64 > 0.7 {
        return true;
    }

    let noise_patterns = [
        "table of contents", "copyright", "all rights reserved",
        "isbn", "printed in", "about the author", "acknowledgments",
        "bibliography", "appendix",
    ];

    // If has noise markers AND doesn't look like questions
    let has_noise = noise_patterns.iter().any(|p| lower.contains(p));
    let has_questions = lower.contains("question") || lower.contains("(a)")
        || lower.contains("(b)") || lower.contains("solve");

    has_noise && !has_questions
}

// ─── Public API ────────────────────────────────────────────────────────

/// Extract questions from a single PDF using llama.cpp
pub async fn extract_from_pdf(pool: &SqlitePool, path: &str, config: &ExtractConfig) -> Result<usize> {
    let text = extract_pdf_text(path)?;

    let source = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown PDF")
        .to_string();

    // Check for image-based PDFs
    if text.trim().len() < 1000 && std::fs::metadata(path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
        eprintln!("  ⚠ Warning: {} appears to be an image scan (very little text extracted). OCR needed.", source);
        return Ok(0);
    }

    let chunks = split_into_chunks(&text, 2000);
    let mut total = 0;

    println!("  • Found {} text pages/chunks to process...", chunks.len());

    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.len() < 50 || is_likely_noise(chunk) {
            continue;
        }

        // Processing indicator (every 5 chunks)
        if i % 5 == 0 {
            println!("  [{} / {}] Processing chunk {}/{}...", source, i + 1, i + 1, chunks.len());
        }

        let prompt = build_prompt(chunk, &source);

        match run_llama(config, &prompt) {
            Ok(response) => {
                let questions = parse_json_response(&response);
                for q in &questions {
                    if is_valid_question(q) {
                        println!("  ✔ Found question: {}", q.question.chars().take(60).collect::<String>());
                        let _ = db::insert_question(
                            pool,
                            &q.section,
                            &q.domain,
                            &q.sub_domain,
                            &source,
                            q.difficulty as i64,
                            "",  // passage - not extracted from PDF yet
                            &q.question,
                            &q.option_a,
                            &q.option_b,
                            &q.option_c,
                            &q.option_d,
                            &q.correct_answer.to_uppercase(),
                            &q.explanation,
                        ).await;
                        total += 1;
                    }
                }
            }
            Err(e) => {
                // Ignore silent errors, but log if major
                if !e.to_string().contains("llama-cli") {
                   eprintln!("  ⚠ Llama error on chunk {}: {}", i, e);
                }
                continue;
            }
        }
    }

    Ok(total)
}

/// Scan directory for PDFs and extract questions from all of them
pub async fn extract_from_directory(pool: &SqlitePool, dir: &str) -> Result<(usize, String)> {
    let config = ExtractConfig::auto_detect()?;
    let model_name = config.model_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut total = 0;
    let mut processed = 0;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext.to_ascii_lowercase() == "pdf" {
                let path_str = path.to_string_lossy().to_string();
                println!("📄 Extracting from: {}", path.file_name().unwrap().to_string_lossy());
                
                match extract_from_pdf(pool, &path_str, &config).await {
                    Ok(count) => {
                        println!("  ✨ extraction complete: {} questions found", count);
                        total += count;
                        processed += 1;
                    }
                    Err(e) => {
                        eprintln!("  ❌ Failed: {}", e);
                        continue;
                    }
                }
            }
        }
    }

    Ok((total, format!("{} PDFs processed with {}", processed, model_name)))
}

/// Check system readiness: pdftotext, llama-cli, model
#[allow(dead_code)]
pub fn check_readiness() -> Vec<(String, bool, String)> {
    let mut checks = Vec::new();

    // pdftotext
    let has_pdftotext = Command::new("which").arg("pdftotext").output()
        .map(|o| o.status.success()).unwrap_or(false);
    checks.push((
        "pdftotext".into(),
        has_pdftotext,
        if has_pdftotext { "✓ installed".into() }
        else { "✗ sudo dnf install poppler-utils".into() }
    ));

    // llama-cli
    let llama = find_llama_cli();
    checks.push((
        "llama-cli".into(),
        llama.is_some(),
        if let Some(ref p) = llama { format!("✓ {}", p.display()) }
        else { "✗ build llama.cpp or add to PATH".into() }
    ));

    // Model
    let model = find_model();
    checks.push((
        "GGUF model".into(),
        model.is_some(),
        if let Some(ref p) = model {
            format!("✓ {}", p.file_name().unwrap_or_default().to_string_lossy())
        } else {
            format!("✗ download {} (~1GB)", DEFAULT_MODEL)
        }
    ));

    checks
}
