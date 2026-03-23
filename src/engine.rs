use crate::models::{MockShuffledOptions, Question};
use crate::db;
use color_eyre::Result;
use rand::RngCore;
use rand::SeedableRng;
use sqlx::SqlitePool;


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

#[allow(dead_code)]
pub fn incorrectness_rating(wrong: i64, attempted: i64, difficulty_weight: f64) -> f64 {
    if attempted == 0 {
        return 0.0;
    }
    (wrong as f64 / attempted as f64) * difficulty_weight
}

#[allow(dead_code)]
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

/// Generate a Mock Exam module with shuffled options
pub async fn generate_mock_module(
    pool: &SqlitePool,
    section: crate::models::MockSection,
    module: u8,
) -> Result<(Vec<Question>, Vec<MockShuffledOptions>)> {
    let section_str = match section {
        crate::models::MockSection::ReadingWriting => "english",
        crate::models::MockSection::Math => "math",
        _ => return Err(color_eyre::eyre::eyre!("Invalid section for module generation")),
    };

    let count = match section {
        crate::models::MockSection::ReadingWriting => 27,
        crate::models::MockSection::Math => 22,
        _ => 0,
    };

    // Module 1 is always a mix of easy, medium, and hard.
    // Module 2 is adaptive based on the passed module parameter (1=Init, 2=Easy, 3=Hard)
    let difficulty_filter = match module {
        1 => "difficulty IN (1, 2, 3)", // Broad mix
        2 => "difficulty IN (1, 2)",    // Easy routing
        3 => "difficulty IN (2, 3)",    // Hard routing
        _ => "difficulty IN (1, 2, 3)",
    };

    let query = format!(
        "SELECT id, section, domain, sub_domain, source, difficulty, \
         question_text, option_a, option_b, option_c, option_d, \
         correct_answer, explanation FROM questions \
         WHERE section = '{}' AND {} \
         ORDER BY RANDOM() LIMIT {}",
        section_str, difficulty_filter, count
    );

    let rows = sqlx::query_as::<_, db::QuestionRow>(&query)
        .fetch_all(pool)
        .await?;

    if rows.len() < count as usize {
        return Err(color_eyre::eyre::eyre!("Not enough questions in database"));
    }

    let questions: Vec<Question> = rows.into_iter().map(|r| r.into()).collect();

    // Shuffle options for each question
    let mut shuffled = Vec::with_capacity(questions.len());
    for q in &questions {
        let mut rng = rand::rngs::StdRng::seed_from_u64(q.id as u64);
        let mut options = [
            q.option_a.clone(),
            q.option_b.clone(),
            q.option_c.clone(),
            q.option_d.clone(),
        ];
        let correct_original: usize = match q.correct_answer.as_str() {
            "A" => 0, "B" => 1, "C" => 2, "D" => 3, _ => 0,
        };
        let correct_text = options[correct_original].clone();

        for i in (1..4).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            options.swap(i, j);
        }

        let correct_index = options.iter().position(|o| *o == correct_text).unwrap_or(0);
        shuffled.push(MockShuffledOptions { texts: options, correct_index });
    }

    Ok((questions, shuffled))
}

#[allow(dead_code)]
pub fn calculate_scaled_score(
    _section: crate::models::MockSection,
    m1_questions: &[Question],
    m1_answers: &[Option<usize>],
    m2_questions: &[Question],
    m2_answers: &[Option<usize>],
) -> u16 {
    let mut raw_points = 0.0;
    let mut max_points = 0.0;

    // Helper to score a module
    let mut score_module = |qs: &[Question], ans: &[Option<usize>]| {
        for (i, q) in qs.iter().enumerate() {
            let weight = match q.difficulty {
                1 => 0.8,
                2 => 1.0,
                3 => 1.2,
                _ => 1.0,
            };
            max_points += weight;
            
            if let Some(user_idx) = ans.get(i).copied().flatten() {
                let user_str = crate::models::Answer::from_index(user_idx)
                    .map(|a| a.to_string())
                    .unwrap_or_default();
                if user_str == q.correct_answer {
                    raw_points += weight;
                }
            }
        }
    };

    score_module(m1_questions, m1_answers);
    score_module(m2_questions, m2_answers);

    if max_points == 0.0 {
        return 200;
    }

    let percentage: f64 = raw_points / max_points;
    
    // Scale 200 to 800
    let scaled: f64 = 200.0 + (percentage * 600.0);
    
    // Round to nearest 10
    (scaled / 10.0).round() as u16 * 10
}
