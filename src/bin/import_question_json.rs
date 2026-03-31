use sat_stream::config::Config;
use sat_stream::db;
use serde::Deserialize;
use sqlx::Row;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
struct ImportQuestion {
    source: String,
    section: String,
    domain: String,
    sub_domain: String,
    difficulty: i64,
    passage: String,
    question_text: String,
    options: Vec<String>,
    correct_answer: String,
    explanation: String,
    media_json: Option<String>,
}

fn normalize_text_for_dedupe(s: &str) -> String {
    s.to_lowercase()
        .replace('\n', " ")
        .replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn valid_section(section: &str) -> bool {
    section == "math" || section == "english"
}

fn valid_answer(ans: &str) -> bool {
    matches!(ans, "A" | "B" | "C" | "D")
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin import_question_json -- <directory-with-json-files>");
        std::process::exit(1);
    }

    let import_dir = PathBuf::from(&args[1]);
    if !import_dir.exists() {
        eprintln!("Directory not found: {}", import_dir.display());
        std::process::exit(1);
    }

    let db_path = Config::db_path();
    let pool = db::init_db(db_path.to_str().unwrap_or_default()).await?;

    let rows = sqlx::query("SELECT question_text FROM questions")
        .fetch_all(&pool)
        .await?;
    let mut dedupe = HashSet::new();
    for row in rows {
        let q: String = row.get("question_text");
        dedupe.insert(normalize_text_for_dedupe(&q));
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

        let raw = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(_) => {
                skipped_invalid += 1;
                continue;
            }
        };
        let q: ImportQuestion = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => {
                skipped_invalid += 1;
                continue;
            }
        };

        if !valid_section(&q.section)
            || q.options.len() != 4
            || !valid_answer(&q.correct_answer)
            || q.question_text.trim().is_empty()
        {
            skipped_invalid += 1;
            continue;
        }

        let key = normalize_text_for_dedupe(&q.question_text);
        if dedupe.contains(&key) {
            skipped_duplicate += 1;
            continue;
        }

        let media_json = q.media_json.unwrap_or_else(|| "[]".to_string());

        db::insert_question(
            &pool,
            &q.section,
            &q.domain,
            &q.sub_domain,
            &q.source,
            q.difficulty,
            &q.passage,
            &media_json,
            &q.question_text,
            &q.options[0],
            &q.options[1],
            &q.options[2],
            &q.options[3],
            &q.correct_answer,
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
