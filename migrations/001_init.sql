-- SAT-Stream Database Schema
-- Embedded via sqlx::migrate!()

CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    section TEXT NOT NULL CHECK(section IN ('math', 'english')),
    domain TEXT NOT NULL,
    sub_domain TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT 'SAT-Stream',
    difficulty INTEGER NOT NULL DEFAULT 2 CHECK(difficulty BETWEEN 1 AND 3),
    question_text TEXT NOT NULL,
    option_a TEXT NOT NULL,
    option_b TEXT NOT NULL,
    option_c TEXT NOT NULL,
    option_d TEXT NOT NULL,
    correct_answer TEXT NOT NULL CHECK(correct_answer IN ('A', 'B', 'C', 'D')),
    explanation TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_id INTEGER NOT NULL,
    is_correct INTEGER NOT NULL CHECK(is_correct IN (0, 1)),
    time_spent_secs INTEGER NOT NULL DEFAULT 0,
    answered_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (question_id) REFERENCES questions(id)
);

CREATE TABLE IF NOT EXISTS error_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_id INTEGER NOT NULL,
    error_type TEXT NOT NULL DEFAULT 'conceptual' CHECK(error_type IN ('conceptual', 'careless', 'time', 'unknown')),
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (question_id) REFERENCES questions(id)
);

CREATE TABLE IF NOT EXISTS study_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    start_time TEXT NOT NULL DEFAULT (datetime('now')),
    end_time TEXT,
    questions_answered INTEGER NOT NULL DEFAULT 0,
    correct_count INTEGER NOT NULL DEFAULT 0,
    domain_focus TEXT NOT NULL DEFAULT 'all'
);

CREATE TABLE IF NOT EXISTS spaced_repetition (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_id INTEGER NOT NULL UNIQUE,
    ease_factor REAL NOT NULL DEFAULT 2.5,
    interval_days INTEGER NOT NULL DEFAULT 1,
    repetitions INTEGER NOT NULL DEFAULT 0,
    next_review TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (question_id) REFERENCES questions(id)
);

CREATE INDEX IF NOT EXISTS idx_questions_section ON questions(section);
CREATE INDEX IF NOT EXISTS idx_questions_domain ON questions(domain);
CREATE INDEX IF NOT EXISTS idx_questions_difficulty ON questions(difficulty);
CREATE INDEX IF NOT EXISTS idx_user_progress_question ON user_progress(question_id);
CREATE INDEX IF NOT EXISTS idx_user_progress_date ON user_progress(answered_at);
CREATE INDEX IF NOT EXISTS idx_spaced_rep_next ON spaced_repetition(next_review);
