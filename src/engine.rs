use crate::models::{Question, Domain};
use crate::db;
use color_eyre::Result;
use sqlx::SqlitePool;
use rand::seq::SliceRandom;

/// Quiz engine modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuizMode {
    /// Random questions from all domains
    Infinite,
    /// Focus on weakest domains
    WeaknessFocus,
    /// Spaced repetition review
    SpacedReview,
    /// Timed SAT simulation
    TimedPractice,
}

impl QuizMode {
    pub fn name(&self) -> &'static str {
        match self {
            QuizMode::Infinite => "Infinite Stream",
            QuizMode::WeaknessFocus => "Weakness Focus",
            QuizMode::SpacedReview => "Spaced Review",
            QuizMode::TimedPractice => "Timed Practice",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            QuizMode::Infinite => "Random questions from all domains",
            QuizMode::WeaknessFocus => "Focus on your weakest areas",
            QuizMode::SpacedReview => "Review questions you got wrong",
            QuizMode::TimedPractice => "Simulate real SAT timing",
        }
    }

    pub fn all() -> Vec<QuizMode> {
        vec![
            QuizMode::Infinite,
            QuizMode::WeaknessFocus,
            QuizMode::SpacedReview,
            QuizMode::TimedPractice,
        ]
    }
}

/// Get the next question based on quiz mode
pub async fn next_question(
    pool: &SqlitePool,
    mode: QuizMode,
    section_filter: Option<&str>,
    domain_filter: Option<&str>,
) -> Result<Option<Question>> {
    match mode {
        QuizMode::Infinite => {
            db::get_random_question(pool, section_filter, domain_filter).await
        }
        QuizMode::WeaknessFocus => {
            // Get weakest domain and pull questions from there
            let stats = db::get_domain_stats(pool).await?;
            if let Some(weakest) = stats.first() {
                db::get_random_question(pool, None, Some(&weakest.domain)).await
            } else {
                db::get_random_question(pool, section_filter, domain_filter).await
            }
        }
        QuizMode::SpacedReview => {
            let due = db::get_due_questions(pool, 1).await?;
            if let Some(q) = due.into_iter().next() {
                Ok(Some(q))
            } else {
                // Fall back to random if no reviews due
                db::get_random_question(pool, section_filter, domain_filter).await
            }
        }
        QuizMode::TimedPractice => {
            db::get_random_question(pool, section_filter, domain_filter).await
        }
    }
}

/// Calculate incorrectness rating for a domain
pub fn incorrectness_rating(wrong: i64, attempted: i64, difficulty_weight: f64) -> f64 {
    if attempted == 0 {
        return 0.0;
    }
    (wrong as f64 / attempted as f64) * difficulty_weight
}

/// Get domain suggestions based on performance
pub async fn get_weak_domains(pool: &SqlitePool) -> Result<Vec<(String, f64)>> {
    let stats = db::get_domain_stats(pool).await?;
    let mut domains: Vec<(String, f64)> = stats
        .iter()
        .filter(|s| s.total_attempted >= 3) // Need at least 3 attempts
        .map(|s| {
            let rating = incorrectness_rating(
                s.total_attempted - s.total_correct,
                s.total_attempted,
                1.0,
            );
            (s.domain.clone(), rating)
        })
        .collect();

    domains.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(domains)
}
