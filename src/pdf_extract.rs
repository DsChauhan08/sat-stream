use color_eyre::Result;
use regex::Regex;
use sqlx::SqlitePool;
use crate::db;

/// Extract questions from a PDF file and insert them into the database.
/// Uses heuristic parsing since PDF text extraction is unstructured.
pub async fn extract_from_pdf(pool: &SqlitePool, path: &str) -> Result<usize> {
    let bytes = std::fs::read(path)?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| color_eyre::eyre::eyre!("PDF extract error: {}", e))?;

    // Determine source name from filename
    let source = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown PDF")
        .to_string();

    let mut total_inserted = 0;

    // Try multiple extraction strategies
    total_inserted += extract_multiple_choice_blocks(&text, pool, &source).await?;
    total_inserted += extract_numbered_questions(&text, pool, &source).await?;

    Ok(total_inserted)
}

/// Strategy 1: Find multiple choice blocks with A/B/C/D pattern
async fn extract_multiple_choice_blocks(text: &str, pool: &SqlitePool, source: &str) -> Result<usize> {
    let mut count = 0;

    // Pattern: question text followed by (A), (B), (C), (D) options
    let mc_pattern = Regex::new(
        r"(?s)(\d+[\.\)]\s+.+?)\s*\(A\)\s*(.+?)\s*\(B\)\s*(.+?)\s*\(C\)\s*(.+?)\s*\(D\)\s*(.+?)(?:\n\n|\z|\d+[\.\)])"
    )?;

    for cap in mc_pattern.captures_iter(text) {
        let q_text = clean_text(&cap[1]);
        let opt_a = clean_text(&cap[2]);
        let opt_b = clean_text(&cap[3]);
        let opt_c = clean_text(&cap[4]);
        let opt_d = clean_text(&cap[5]);

        // Skip if too short (likely garbage)
        if q_text.len() < 10 || opt_a.len() < 1 || opt_b.len() < 1 {
            continue;
        }

        // Skip if it's a table of contents or header
        if q_text.contains("Chapter") || q_text.contains("Table of Contents") || q_text.contains("Page") {
            continue;
        }

        let (section, domain) = classify_question(&q_text);

        // We don't know the answer from PDF, so mark as "A" (will need manual correction or AI verification)
        db::insert_question(
            pool, &section, &domain, "General", source,
            2, // Medium difficulty default
            &q_text, &opt_a, &opt_b, &opt_c, &opt_d,
            "A", // Default - unknown answer
            "Extracted from PDF. Answer may need verification."
        ).await?;

        count += 1;
    }

    Ok(count)
}

/// Strategy 2: Find numbered questions with A) B) C) D) pattern variant
async fn extract_numbered_questions(text: &str, pool: &SqlitePool, source: &str) -> Result<usize> {
    let mut count = 0;

    // Alternative pattern: A) B) C) D) without parentheses around letter
    let mc_pattern2 = Regex::new(
        r"(?s)(\d+[\.\)]\s+.+?)\s*A\)\s*(.+?)\s*B\)\s*(.+?)\s*C\)\s*(.+?)\s*D\)\s*(.+?)(?:\n\n|\z|\d+[\.\)])"
    )?;

    for cap in mc_pattern2.captures_iter(text) {
        let q_text = clean_text(&cap[1]);
        let opt_a = clean_text(&cap[2]);
        let opt_b = clean_text(&cap[3]);
        let opt_c = clean_text(&cap[4]);
        let opt_d = clean_text(&cap[5]);

        if q_text.len() < 10 || opt_a.len() < 1 {
            continue;
        }

        if q_text.contains("Chapter") || q_text.contains("Table of Contents") {
            continue;
        }

        let (section, domain) = classify_question(&q_text);

        db::insert_question(
            pool, &section, &domain, "General", source,
            2, &q_text, &opt_a, &opt_b, &opt_c, &opt_d,
            "A", "Extracted from PDF. Answer may need verification."
        ).await?;

        count += 1;
    }

    Ok(count)
}

/// Try to classify a question into section/domain based on content
fn classify_question(text: &str) -> (String, String) {
    let lower = text.to_lowercase();

    // Math indicators
    let math_keywords = [
        "equation", "solve", "graph", "slope", "x =", "y =", "triangle",
        "circle", "area", "volume", "angle", "sin", "cos", "tan",
        "polynomial", "quadratic", "linear", "exponent", "logarithm",
        "integer", "fraction", "percent", "ratio", "probability",
        "median", "mean", "standard deviation", "function f(",
        "value of x", "value of y", "expression", "inequality",
        "perpendicular", "parallel", "vertex", "parabola", "factor",
        "simplify", "calculate", "how many", "what is the value",
    ];

    let is_math = math_keywords.iter().any(|kw| lower.contains(kw));

    if is_math {
        // Sub-classify math
        let algebra_kw = ["equation", "solve", "linear", "inequality", "slope", "system", "x =", "y ="];
        let advanced_kw = ["quadratic", "polynomial", "exponent", "logarithm", "factor", "vertex", "parabola"];
        let data_kw = ["probability", "median", "mean", "standard deviation", "percent", "ratio", "scatter"];
        let geo_kw = ["triangle", "circle", "area", "volume", "angle", "sin", "cos", "tan", "perpendicular"];

        if geo_kw.iter().any(|kw| lower.contains(kw)) {
            ("math".to_string(), "Geometry & Trigonometry".to_string())
        } else if data_kw.iter().any(|kw| lower.contains(kw)) {
            ("math".to_string(), "Problem Solving & Data Analysis".to_string())
        } else if advanced_kw.iter().any(|kw| lower.contains(kw)) {
            ("math".to_string(), "Advanced Math".to_string())
        } else {
            ("math".to_string(), "Algebra".to_string())
        }
    } else {
        // English sub-classification
        let grammar_kw = ["verb", "pronoun", "comma", "semicolon", "apostrophe", "punctuation",
                          "subject-verb", "who/whom", "which/that", "tense", "plural", "possessive"];
        let expression_kw = ["transition", "concise", "combine", "revision", "placement",
                            "tone", "sentence", "paragraph", "wordiness"];

        if grammar_kw.iter().any(|kw| lower.contains(kw)) {
            ("english".to_string(), "Standard English Conventions".to_string())
        } else if expression_kw.iter().any(|kw| lower.contains(kw)) {
            ("english".to_string(), "Expression of Ideas".to_string())
        } else if lower.contains("passage") || lower.contains("author") || lower.contains("purpose") {
            ("english".to_string(), "Craft and Structure".to_string())
        } else {
            ("english".to_string(), "Information and Ideas".to_string())
        }
    }
}

/// Clean extracted text: collapse whitespace, trim, remove control chars
fn clean_text(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect();

    // Collapse multiple whitespace
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(cleaned.trim(), " ").to_string()
}

/// Scan a directory for PDF files and extract questions from all of them
pub async fn extract_from_directory(pool: &SqlitePool, dir: &str) -> Result<usize> {
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
