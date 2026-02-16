use sat_stream::db;
use sat_stream::models::*;
use sat_stream::engine::*;
use sat_stream::config::*;
use sat_stream::seed;

/// Test database initialization and schema creation
#[tokio::test]
async fn test_db_init() {
    let pool = db::init_db(":memory:").await.unwrap();
    // Should succeed without errors
    let count = db::question_count(&pool).await.unwrap();
    assert_eq!(count, 0, "Fresh database should have 0 questions");
}

/// Test seeding populates all 8 domains
#[tokio::test]
async fn test_seed_questions() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let count = db::question_count(&pool).await.unwrap();
    assert!(count >= 100, "Should have at least 100 questions, got {}", count);
}

/// Test seed is idempotent (doesn't double-insert)
#[tokio::test]
async fn test_seed_idempotent() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();
    let count1 = db::question_count(&pool).await.unwrap();

    seed::seed_if_empty(&pool).await.unwrap();
    let count2 = db::question_count(&pool).await.unwrap();

    assert_eq!(count1, count2, "Seed should be idempotent");
}

/// Test random question retrieval
#[tokio::test]
async fn test_get_random_question() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = db::get_random_question(&pool, None, None).await.unwrap();
    assert!(q.is_some(), "Should return a question from seeded DB");

    let q = q.unwrap();
    assert!(!q.question_text.is_empty(), "Question text should not be empty");
    assert!(!q.option_a.is_empty(), "Option A should not be empty");
    assert!(!q.correct_answer.is_empty(), "Correct answer should not be empty");
}

/// Test section filtering
#[tokio::test]
async fn test_section_filter() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = db::get_random_question(&pool, Some("math"), None).await.unwrap();
    assert!(q.is_some(), "Should return a math question");
    assert_eq!(q.unwrap().section, "math", "Filtered question should be math");

    let q = db::get_random_question(&pool, Some("english"), None).await.unwrap();
    assert!(q.is_some(), "Should return an english question");
    assert_eq!(q.unwrap().section, "english", "Filtered question should be english");
}

/// Test domain filtering
#[tokio::test]
async fn test_domain_filter() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = db::get_random_question(&pool, None, Some("Algebra")).await.unwrap();
    assert!(q.is_some(), "Should return an Algebra question");
    assert_eq!(q.unwrap().domain, "Algebra");
}

/// Test answer recording
#[tokio::test]
async fn test_record_answer() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = db::get_random_question(&pool, None, None).await.unwrap().unwrap();

    // Record a correct answer
    db::record_answer(&pool, q.id, true, 30).await.unwrap();

    // Record a wrong answer
    db::record_answer(&pool, q.id, false, 45).await.unwrap();

    // Check stats
    let (total, correct, _) = db::get_overall_stats(&pool).await.unwrap();
    assert_eq!(total, 2, "Should have 2 total answers");
    assert_eq!(correct, 1, "Should have 1 correct answer");
}

/// Test domain stats
#[tokio::test]
async fn test_domain_stats() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = db::get_random_question(&pool, Some("math"), Some("Algebra")).await.unwrap().unwrap();
    db::record_answer(&pool, q.id, true, 20).await.unwrap();
    db::record_answer(&pool, q.id, false, 30).await.unwrap();

    let stats = db::get_domain_stats(&pool).await.unwrap();
    assert!(!stats.is_empty(), "Should have domain stats");

    let algebra = stats.iter().find(|s| s.domain == "Algebra");
    assert!(algebra.is_some(), "Should have Algebra stats");
    let a = algebra.unwrap();
    assert_eq!(a.total_attempted, 2);
}

/// Test sessions
#[tokio::test]
async fn test_sessions() {
    let pool = db::init_db(":memory:").await.unwrap();

    let sid = db::start_session(&pool, "Infinite Stream").await.unwrap();
    assert!(sid > 0, "Session ID should be positive");

    db::update_session(&pool, sid, 10, 7).await.unwrap();
    // Should succeed without errors
}

/// Test daily activity
#[tokio::test]
async fn test_daily_activity() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = db::get_random_question(&pool, None, None).await.unwrap().unwrap();
    db::record_answer(&pool, q.id, true, 20).await.unwrap();

    let activity = db::get_daily_activity(&pool, 7).await.unwrap();
    assert!(!activity.is_empty(), "Should have activity for today");
}

/// Test streak counting
#[tokio::test]
async fn test_streak() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = db::get_random_question(&pool, None, None).await.unwrap().unwrap();

    // Record 3 correct answers
    db::record_answer(&pool, q.id, true, 10).await.unwrap();
    db::record_answer(&pool, q.id, true, 10).await.unwrap();
    db::record_answer(&pool, q.id, true, 10).await.unwrap();

    let streak = db::get_current_streak(&pool).await.unwrap();
    assert_eq!(streak, 3, "Streak should be 3 after 3 correct");

    // Break the streak
    db::record_answer(&pool, q.id, false, 10).await.unwrap();
    let streak = db::get_current_streak(&pool).await.unwrap();
    assert_eq!(streak, 0, "Streak should be 0 after a wrong answer");
}

/// Test quiz engine modes
#[tokio::test]
async fn test_quiz_engine_infinite() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    let q = next_question(&pool, QuizMode::Infinite, None, None).await.unwrap();
    assert!(q.is_some(), "Infinite mode should return a question");
}

#[tokio::test]
async fn test_quiz_engine_weakness_focus() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    // Even without prior answers, should fall back to random
    let q = next_question(&pool, QuizMode::WeaknessFocus, None, None).await.unwrap();
    assert!(q.is_some(), "WeaknessFocus should return a question");
}

#[tokio::test]
async fn test_quiz_engine_spaced_review() {
    let pool = db::init_db(":memory:").await.unwrap();
    seed::seed_if_empty(&pool).await.unwrap();

    // Without wrong answers, should fall back to random
    let q = next_question(&pool, QuizMode::SpacedReview, None, None).await.unwrap();
    assert!(q.is_some(), "SpacedReview should fall back to random");
}

/// Test config system
#[test]
fn test_config_defaults() {
    let config = Config::default();
    assert_eq!(config.theme, Theme::Default);
    assert_eq!(config.math_time_per_question_secs, 95);
    assert_eq!(config.english_time_per_question_secs, 71);
    assert_eq!(config.questions_per_session, 20);
    assert!(!config.timed_mode);
}

/// Test theme cycling
#[test]
fn test_theme_cycling() {
    let themes = Theme::all();
    assert_eq!(themes.len(), 4, "Should have 4 themes");
    assert_eq!(themes[0], Theme::Default);
    assert_eq!(themes[1], Theme::Dark);
    assert_eq!(themes[2], Theme::Solarized);
    assert_eq!(themes[3], Theme::Gruvbox);
}

/// Test theme colors are distinct
#[test]
fn test_theme_colors() {
    for theme in Theme::all() {
        // Each theme should have non-identical bg and text colors
        assert_ne!(theme.bg(), theme.text(), "Theme {} bg and text should differ", theme.name());
        assert_ne!(theme.bg(), theme.accent(), "Theme {} bg and accent should differ", theme.name());
    }
}

/// Test question model helpers
#[test]
fn test_question_difficulty_label() {
    let mut q = Question {
        id: 1,
        section: "math".to_string(),
        domain: "Algebra".to_string(),
        sub_domain: "Linear Equations".to_string(),
        source: "Test".to_string(),
        difficulty: 1,
        question_text: "Test?".to_string(),
        option_a: "A".to_string(),
        option_b: "B".to_string(),
        option_c: "C".to_string(),
        option_d: "D".to_string(),
        correct_answer: "A".to_string(),
        explanation: "Because.".to_string(),
    };

    assert_eq!(q.difficulty_label(), "Easy");
    q.difficulty = 2;
    assert_eq!(q.difficulty_label(), "Medium");
    q.difficulty = 3;
    assert_eq!(q.difficulty_label(), "Hard");
}

/// Test Answer model
#[test]
fn test_answer_from_index() {
    assert_eq!(Answer::from_index(0), Some(Answer::A));
    assert_eq!(Answer::from_index(1), Some(Answer::B));
    assert_eq!(Answer::from_index(2), Some(Answer::C));
    assert_eq!(Answer::from_index(3), Some(Answer::D));
    assert_eq!(Answer::from_index(4), None);
}

/// Test quiz mode names
#[test]
fn test_quiz_mode_names() {
    assert_eq!(QuizMode::Infinite.name(), "Infinite Stream");
    assert_eq!(QuizMode::WeaknessFocus.name(), "Weakness Focus");
    assert_eq!(QuizMode::SpacedReview.name(), "Spaced Review");
    assert_eq!(QuizMode::TimedPractice.name(), "Timed Practice");
}

/// Test all quiz modes have descriptions
#[test]
fn test_quiz_mode_descriptions() {
    for mode in QuizMode::all() {
        assert!(!mode.description().is_empty(), "Mode {:?} should have a description", mode);
    }
}
