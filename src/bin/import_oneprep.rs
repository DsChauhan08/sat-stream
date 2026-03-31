use sat_stream::config::Config;
use sat_stream::db;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnePrepChoice {
    text: String,
    letter: String,
    is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnePrepQuestion {
    module: String,
    domain: String,
    sub_domain: String,
    difficulty: i64,
    stimulus: String,
    stem: String,
    explanation: String,
    choices: Vec<OnePrepChoice>,
    media_json: String,
    source_id: String,
}

fn parse_difficulty(label: &str) -> i64 {
    match label {
        "E" | "easy" | "Easy" => 1,
        "M" | "medium" | "Medium" => 2,
        "H" | "hard" | "Hard" => 3,
        _ => 2,
    }
}

fn map_module(module: &str) -> &'static str {
    match module {
        "en" | "english" => "english",
        _ => "math",
    }
}

fn map_domain(raw: &str, module: &str) -> String {
    let norm = raw.trim();
    if module == "english" || module == "en" {
        match norm {
            "Craft and Structure" => "Craft and Structure".to_string(),
            "Information and Ideas" => "Information and Ideas".to_string(),
            "Standard English Conventions" => "Standard English Conventions".to_string(),
            "Expression of Ideas" => "Expression of Ideas".to_string(),
            _ => "Information and Ideas".to_string(),
        }
    } else {
        match norm {
            "Algebra" => "Algebra".to_string(),
            "Advanced Math" => "Advanced Math".to_string(),
            "Problem Solving & Data Analysis" => "Problem Solving & Data Analysis".to_string(),
            "Geometry & Trigonometry" | "Geometry and Trigonometry" => {
                "Geometry & Trigonometry".to_string()
            }
            _ => "Algebra".to_string(),
        }
    }
}

fn normalize_text_for_dedupe(s: &str) -> String {
    s.to_lowercase()
        .replace("\n", " ")
        .replace("\t", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_question_json(path: &Path) -> Option<OnePrepQuestion> {
    let raw = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;

    let module = v.get("module")?.as_str()?.to_string();
    let stimulus = v
        .get("stimulus")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();
    let stem = v
        .get("stem")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();
    let explanation = v
        .get("explanation")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();
    let difficulty = parse_difficulty(v.get("difficulty").and_then(|x| x.as_str()).unwrap_or("M"));

    let metadata = v.get("metadata").cloned().unwrap_or(serde_json::Value::Null);
    let domain_raw = metadata
        .get("pr_domain")
        .and_then(|x| x.as_str())
        .unwrap_or_default();
    let sub_domain = metadata
        .get("pr_skill")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let domain = map_domain(domain_raw, &module);

    let source_id = v
        .get("sourceId")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();

    let mut choices = Vec::new();
    if let Some(arr) = v.get("answerChoices").and_then(|x| x.as_array()) {
        for item in arr {
            choices.push(OnePrepChoice {
                text: item
                    .get("text")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                letter: item
                    .get("letter")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                is_correct: item
                    .get("isCorrect")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
            });
        }
    }

    // Extract any media URLs from stimulus or stem HTML if present
    let mut media = Vec::<serde_json::Value>::new();
    for blob in [&stimulus, &stem] {
        // crude but robust extraction for src="..."
        for cap in regex::Regex::new(r#"src=\"([^\"]+)\""#)
            .ok()?
            .captures_iter(blob)
        {
            let src = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            if src.is_empty() {
                continue;
            }
            media.push(serde_json::json!({
                "kind": "image",
                "url": src,
                "path": "",
                "caption": "Imported from OnePrep"
            }));
        }
    }

    Some(OnePrepQuestion {
        module,
        domain,
        sub_domain,
        difficulty,
        stimulus,
        stem,
        explanation,
        choices,
        media_json: serde_json::to_string(&media).unwrap_or_else(|_| "[]".to_string()),
        source_id,
    })
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin import_oneprep -- <directory-with-json-files>");
        std::process::exit(1);
    }

    let import_dir = PathBuf::from(&args[1]);
    if !import_dir.exists() {
        eprintln!("Directory not found: {}", import_dir.display());
        std::process::exit(1);
    }

    let db_path = Config::db_path();
    let pool = db::init_db(db_path.to_str().unwrap_or_default()).await?;

    // Build dedupe index from existing questions
    let existing = sqlx::query_as::<_, db::QuestionRow>(
        "SELECT id, section, domain, sub_domain, source, difficulty, passage, media_json, question_text, \
         option_a, option_b, option_c, option_d, correct_answer, explanation FROM questions"
    )
    .fetch_all(&pool)
    .await?;

    let mut dedupe = HashSet::new();
    for row in existing {
        let q: sat_stream::models::Question = row.into();
        dedupe.insert(normalize_text_for_dedupe(&q.question_text));
    }

    let mut total = 0usize;
    let mut inserted = 0usize;
    let mut skipped_duplicate = 0usize;
    let mut skipped_invalid = 0usize;

    for entry in fs::read_dir(&import_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        total += 1;

        let q = match load_question_json(&path) {
            Some(q) => q,
            None => {
                skipped_invalid += 1;
                continue;
            }
        };

        if q.choices.len() < 4 {
            skipped_invalid += 1;
            continue;
        }

        let key = normalize_text_for_dedupe(&q.stem);
        if dedupe.contains(&key) {
            skipped_duplicate += 1;
            continue;
        }

        let correct_letter = q
            .choices
            .iter()
            .find(|c| c.is_correct)
            .map(|c| c.letter.clone())
            .unwrap_or_else(|| "A".to_string());

        let section = map_module(&q.module);
        let passage = q.stimulus;
        let stem = q.stem;

        db::insert_question(
            &pool,
            section,
            &q.domain,
            &q.sub_domain,
            &format!("OnePrep ({})", q.source_id),
            q.difficulty,
            &passage,
            &q.media_json,
            &stem,
            &q.choices[0].text,
            &q.choices[1].text,
            &q.choices[2].text,
            &q.choices[3].text,
            &correct_letter,
            &q.explanation,
        )
        .await?;

        dedupe.insert(key);
        inserted += 1;
    }

    println!("Processed files: {}", total);
    println!("Inserted: {}", inserted);
    println!("Skipped duplicate: {}", skipped_duplicate);
    println!("Skipped invalid: {}", skipped_invalid);

    Ok(())
}
