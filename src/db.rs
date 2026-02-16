use color_eyre::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool, Row};
use crate::models::{Question, DomainStats, DailyActivity, SpacedRepCard};

/// Initialize database: create file, run migrations, return pool
pub async fn init_db(db_path: &str) -> Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let url = format!("sqlite:{}?mode=rwc", db_path);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    // Run embedded migrations
    sqlx::query(include_str!("../migrations/001_init.sql"))
        .execute(&pool)
        .await
        .ok(); // Ignore if tables already exist

    Ok(pool)
}

/// Get total question count
pub async fn question_count(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM questions")
        .fetch_one(pool)
        .await?;
    Ok(row.get::<i64, _>("cnt"))
}

/// Get a random question, optionally filtered by section/domain
pub async fn get_random_question(
    pool: &SqlitePool,
    section_filter: Option<&str>,
    domain_filter: Option<&str>,
) -> Result<Option<Question>> {
    let mut query = String::from(
        "SELECT id, section, domain, sub_domain, source, difficulty, \
         question_text, option_a, option_b, option_c, option_d, \
         correct_answer, explanation FROM questions"
    );
    let mut conditions = Vec::new();

    if section_filter.is_some() {
        conditions.push("section = ?1");
    }
    if domain_filter.is_some() {
        conditions.push(if section_filter.is_some() { "domain = ?2" } else { "domain = ?1" });
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }
    query.push_str(" ORDER BY RANDOM() LIMIT 1");

    let mut q = sqlx::query_as::<_, QuestionRow>(&query);
    if let Some(s) = section_filter {
        q = q.bind(s);
    }
    if let Some(d) = domain_filter {
        q = q.bind(d);
    }

    let row = q.fetch_optional(pool).await?;
    Ok(row.map(|r| r.into()))
}

/// Get questions due for spaced repetition review
pub async fn get_due_questions(pool: &SqlitePool, limit: i64) -> Result<Vec<Question>> {
    let rows = sqlx::query_as::<_, QuestionRow>(
        "SELECT q.id, q.section, q.domain, q.sub_domain, q.source, q.difficulty, \
         q.question_text, q.option_a, q.option_b, q.option_c, q.option_d, \
         q.correct_answer, q.explanation \
         FROM questions q \
         INNER JOIN spaced_repetition sr ON q.id = sr.question_id \
         WHERE sr.next_review <= datetime('now') \
         ORDER BY sr.next_review ASC LIMIT ?1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Record an answer
pub async fn record_answer(
    pool: &SqlitePool,
    question_id: i64,
    is_correct: bool,
    time_spent_secs: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO user_progress (question_id, is_correct, time_spent_secs) VALUES (?1, ?2, ?3)"
    )
    .bind(question_id)
    .bind(is_correct as i64)
    .bind(time_spent_secs)
    .execute(pool)
    .await?;

    // Update spaced repetition
    if !is_correct {
        update_spaced_rep(pool, question_id, false).await?;
    } else {
        update_spaced_rep(pool, question_id, true).await?;
    }

    Ok(())
}

/// Update or create spaced repetition entry using SM-2 algorithm
async fn update_spaced_rep(pool: &SqlitePool, question_id: i64, correct: bool) -> Result<()> {
    let existing = sqlx::query("SELECT * FROM spaced_repetition WHERE question_id = ?1")
        .bind(question_id)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = existing {
        let mut ef: f64 = row.get("ease_factor");
        let mut interval: i64 = row.get("interval_days");
        let mut reps: i64 = row.get("repetitions");

        if correct {
            reps += 1;
            match reps {
                1 => interval = 1,
                2 => interval = 3,
                _ => interval = (interval as f64 * ef) as i64,
            }
            ef = (ef + 0.1 - 0.08).max(1.3); // SM-2 ease adjustment for correct
        } else {
            reps = 0;
            interval = 1;
            ef = (ef - 0.3).max(1.3); // Decrease ease on wrong answer
        }

        let next_review = format!("datetime('now', '+{} days')", interval);
        sqlx::query(&format!(
            "UPDATE spaced_repetition SET ease_factor = ?1, interval_days = ?2, \
             repetitions = ?3, next_review = {} WHERE question_id = ?4",
            next_review
        ))
        .bind(ef)
        .bind(interval)
        .bind(reps)
        .bind(question_id)
        .execute(pool)
        .await?;
    } else if !correct {
        // Only create SR entry for wrong answers
        sqlx::query(
            "INSERT INTO spaced_repetition (question_id, ease_factor, interval_days, repetitions, next_review) \
             VALUES (?1, 2.5, 1, 0, datetime('now', '+1 days'))"
        )
        .bind(question_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Get domain-level performance statistics
pub async fn get_domain_stats(pool: &SqlitePool) -> Result<Vec<DomainStats>> {
    let rows = sqlx::query(
        "SELECT q.domain, \
         COUNT(up.id) as total_attempted, \
         SUM(up.is_correct) as total_correct, \
         AVG(CAST(up.is_correct AS REAL)) as accuracy, \
         AVG(up.time_spent_secs) as avg_time \
         FROM user_progress up \
         JOIN questions q ON up.question_id = q.id \
         GROUP BY q.domain \
         ORDER BY accuracy ASC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| DomainStats {
        domain: r.get("domain"),
        total_attempted: r.get("total_attempted"),
        total_correct: r.get::<i64, _>("total_correct"),
        accuracy: r.get::<f64, _>("accuracy"),
        avg_time_secs: r.get::<f64, _>("avg_time"),
    }).collect())
}

/// Get daily activity for heatmap (last N days)
pub async fn get_daily_activity(pool: &SqlitePool, days: i64) -> Result<Vec<DailyActivity>> {
    let rows = sqlx::query(
        "SELECT DATE(answered_at) as date, \
         COUNT(*) as questions_answered, \
         SUM(is_correct) as correct \
         FROM user_progress \
         WHERE answered_at >= datetime('now', ?1) \
         GROUP BY DATE(answered_at) \
         ORDER BY date ASC"
    )
    .bind(format!("-{} days", days))
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| DailyActivity {
        date: r.get("date"),
        questions_answered: r.get("questions_answered"),
        correct: r.get::<i64, _>("correct"),
    }).collect())
}

/// Get overall stats
pub async fn get_overall_stats(pool: &SqlitePool) -> Result<(i64, i64, i64)> {
    let row = sqlx::query(
        "SELECT COUNT(*) as total, \
         COALESCE(SUM(is_correct), 0) as correct \
         FROM user_progress"
    )
    .fetch_optional(pool)
    .await?;

    let (total, correct) = match row {
        Some(r) => (r.get::<i64, _>("total"), r.get::<i64, _>("correct")),
        None => (0, 0),
    };

    let streak = get_current_streak(pool).await.unwrap_or(0);
    Ok((total, correct, streak))
}

/// Get current streak
pub async fn get_current_streak(pool: &SqlitePool) -> Result<i64> {
    let rows = sqlx::query(
        "SELECT is_correct FROM user_progress ORDER BY answered_at DESC LIMIT 100"
    )
    .fetch_all(pool)
    .await?;

    let mut streak = 0i64;
    for r in &rows {
        let correct: i64 = r.get("is_correct");
        if correct == 1 {
            streak += 1;
        } else {
            break;
        }
    }
    Ok(streak)
}

/// Record a study session start
pub async fn start_session(pool: &SqlitePool, domain_focus: &str) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO study_sessions (domain_focus) VALUES (?1)"
    )
    .bind(domain_focus)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Update study session
pub async fn update_session(
    pool: &SqlitePool,
    session_id: i64,
    questions: i64,
    correct: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE study_sessions SET questions_answered = ?1, correct_count = ?2, \
         end_time = datetime('now') WHERE id = ?3"
    )
    .bind(questions)
    .bind(correct)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert a question into the database
pub async fn insert_question(
    pool: &SqlitePool,
    section: &str,
    domain: &str,
    sub_domain: &str,
    source: &str,
    difficulty: i64,
    question_text: &str,
    option_a: &str,
    option_b: &str,
    option_c: &str,
    option_d: &str,
    correct_answer: &str,
    explanation: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO questions (section, domain, sub_domain, source, difficulty, \
         question_text, option_a, option_b, option_c, option_d, correct_answer, explanation) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
    )
    .bind(section)
    .bind(domain)
    .bind(sub_domain)
    .bind(source)
    .bind(difficulty)
    .bind(question_text)
    .bind(option_a)
    .bind(option_b)
    .bind(option_c)
    .bind(option_d)
    .bind(correct_answer)
    .bind(explanation)
    .execute(pool)
    .await?;
    Ok(())
}

// Internal helper struct for sqlx deserialization
#[derive(sqlx::FromRow)]
struct QuestionRow {
    id: i64,
    section: String,
    domain: String,
    sub_domain: String,
    source: String,
    difficulty: i64,
    question_text: String,
    option_a: String,
    option_b: String,
    option_c: String,
    option_d: String,
    correct_answer: String,
    explanation: String,
}

impl From<QuestionRow> for Question {
    fn from(r: QuestionRow) -> Self {
        Question {
            id: r.id,
            section: r.section,
            domain: r.domain,
            sub_domain: r.sub_domain,
            source: r.source,
            difficulty: r.difficulty,
            question_text: r.question_text,
            option_a: r.option_a,
            option_b: r.option_b,
            option_c: r.option_c,
            option_d: r.option_d,
            correct_answer: r.correct_answer,
            explanation: r.explanation,
        }
    }
}
