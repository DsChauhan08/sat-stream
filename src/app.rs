use crate::ai::AiClient;
use crate::config::{Config, Theme};
use crate::engine::QuizMode;
use crate::models::{DailyActivity, DomainStats, Question};
use serde::{Deserialize, Serialize};

/// Current application screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Quiz,
    Stats,
    Review,
    Settings,
    Help,
    MockExam,
}

/// What the text input is targeting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    ApiKey,
}

/// Visual feedback state for answer animation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feedback {
    None,
    Correct,
    Wrong,
}

/// Application state
pub struct App {
    pub running: bool,
    pub screen: Screen,
    pub config: Config,
    pub ai_client: AiClient,

    // Quiz state
    pub current_question: Option<Question>,
    pub selected_answer: usize,     // 0-3 for A-D
    pub answered: bool,
    pub feedback: Feedback,
    pub feedback_timer: u8,         // Countdown for feedback display
    pub quiz_mode: QuizMode,
    pub section_filter: Option<String>,
    pub domain_filter: Option<String>,

    // Session tracking
    pub session_id: Option<i64>,
    pub session_questions: i64,
    pub session_correct: i64,
    pub current_streak: i64,
    pub best_streak: i64,
    pub total_answered: i64,
    pub total_correct: i64,

    // Timed mode
    pub time_remaining_secs: Option<u64>,
    pub question_start_time: Option<std::time::Instant>,

    // Mock Exam state
    pub mock_exam_state: Option<crate::models::MockExamState>,

    // SRS
    pub srs_due_count: i64,

    // Stats data (cached)
    pub domain_stats: Vec<DomainStats>,
    pub daily_activity: Vec<DailyActivity>,

    // AI state
    pub ai_response: Option<String>,
    pub ai_loading: bool,

    // UI state
    pub home_selected: usize,       // Home menu selection
    pub settings_selected: usize,   // Settings menu selection
    pub stats_scroll: u16,
    pub review_scroll: u16,
    pub ai_scroll: u16,
    pub wrong_answers: Vec<(Question, String)>, // (question, user_answer)

    // Mode selection
    pub mode_selector_open: bool,
    pub mode_selected: usize,

    // Text input state (for API key, etc.)
    pub input_active: bool,
    pub input_buffer: String,
    pub input_label: String,
    pub input_target: InputTarget,

    // Status message (shown briefly at bottom)
    pub status_message: Option<String>,
    pub status_timer: u8,

    // Ticker for animations
    pub tick: u64,
}

impl App {
    pub fn new(config: Config) -> Self {
        let ai_client = AiClient::new(config.gemini_api_key.clone());
        Self {
            running: true,
            screen: Screen::Home,
            config,
            ai_client,

            current_question: None,
            selected_answer: 0,
            answered: false,
            feedback: Feedback::None,
            feedback_timer: 0,
            quiz_mode: QuizMode::Infinite,
            section_filter: None,
            domain_filter: None,

            session_id: None,
            session_questions: 0,
            session_correct: 0,
            current_streak: 0,
            best_streak: 0,
            total_answered: 0,
            total_correct: 0,

            time_remaining_secs: None,
            question_start_time: None,

            mock_exam_state: None,
            srs_due_count: 0,

            domain_stats: Vec::new(),
            daily_activity: Vec::new(),

            ai_response: None,
            ai_loading: false,

            home_selected: 0,
            settings_selected: 0,
            stats_scroll: 0,
            review_scroll: 0,
            ai_scroll: 0,
            wrong_answers: Vec::new(),

            mode_selector_open: false,
            mode_selected: 0,

            input_active: false,
            input_buffer: String::new(),
            input_label: String::new(),
            input_target: InputTarget::ApiKey,

            status_message: None,
            status_timer: 0,

            tick: 0,
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.config.theme
    }

    pub fn accuracy(&self) -> f64 {
        if self.session_questions == 0 {
            0.0
        } else {
            (self.session_correct as f64 / self.session_questions as f64) * 100.0
        }
    }

    pub fn overall_accuracy(&self) -> f64 {
        if self.total_answered == 0 {
            0.0
        } else {
            (self.total_correct as f64 / self.total_answered as f64) * 100.0
        }
    }

    pub fn cycle_theme(&mut self) {
        let themes = Theme::all();
        let idx = themes.iter().position(|t| *t == self.config.theme).unwrap_or(0);
        self.config.theme = themes[(idx + 1) % themes.len()];
        let _ = self.config.save();
    }

    pub fn navigate(&mut self, screen: Screen) {
        self.screen = screen;
        self.ai_response = None;
        self.ai_scroll = 0;
    }

    pub fn open_input(&mut self, label: &str, target: InputTarget, prefill: &str) {
        self.input_active = true;
        self.input_label = label.to_string();
        self.input_target = target;
        self.input_buffer = prefill.to_string();
    }

    pub fn close_input(&mut self) {
        self.input_active = false;
        self.input_buffer.clear();
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
        self.status_timer = 90; // ~3 seconds at 30fps
    }
}

/// Persisted state for resume functionality
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedState {
    pub last_question_id: Option<i64>,
    pub quiz_mode: String,
    pub section_filter: Option<String>,
    pub domain_filter: Option<String>,
    pub session_questions: i64,
    pub session_correct: i64,
    pub mock_exam_state: Option<crate::models::MockExamState>,
}

impl PersistedState {
    pub fn save(app: &App) -> color_eyre::Result<()> {
        let state = PersistedState {
            last_question_id: app.current_question.as_ref().map(|q| q.id),
            quiz_mode: app.quiz_mode.name().to_string(),
            section_filter: app.section_filter.clone(),
            domain_filter: app.domain_filter.clone(),
            session_questions: app.session_questions,
            session_correct: app.session_correct,
            mock_exam_state: app.mock_exam_state.clone(),
        };
        let path = Config::state_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load() -> Option<PersistedState> {
        let path = Config::state_path();
        if path.exists() {
            let content = std::fs::read_to_string(path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }
}
