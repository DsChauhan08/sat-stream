use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[allow(dead_code)]
pub enum Section {
    Math,
    English,
}

impl Section {
    pub fn as_str(&self) -> &'static str {
        match self {
            Section::Math => "math",
            Section::English => "english",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "math" | "mathematics" => Section::Math,
            _ => Section::English,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Section::Math => "Mathematics",
            Section::English => "Reading & Writing",
        }
    }
}

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[allow(dead_code)]
pub enum Domain {
    // English domains
    CraftAndStructure,
    InformationAndIdeas,
    StandardEnglishConventions,
    ExpressionOfIdeas,
    // Math domains
    Algebra,
    AdvancedMath,
    ProblemSolvingAndDataAnalysis,
    GeometryAndTrigonometry,
}

impl Domain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Domain::CraftAndStructure => "Craft and Structure",
            Domain::InformationAndIdeas => "Information and Ideas",
            Domain::StandardEnglishConventions => "Standard English Conventions",
            Domain::ExpressionOfIdeas => "Expression of Ideas",
            Domain::Algebra => "Algebra",
            Domain::AdvancedMath => "Advanced Math",
            Domain::ProblemSolvingAndDataAnalysis => "Problem Solving & Data Analysis",
            Domain::GeometryAndTrigonometry => "Geometry & Trigonometry",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Craft and Structure" => Domain::CraftAndStructure,
            "Information and Ideas" => Domain::InformationAndIdeas,
            "Standard English Conventions" => Domain::StandardEnglishConventions,
            "Expression of Ideas" => Domain::ExpressionOfIdeas,
            "Algebra" => Domain::Algebra,
            "Advanced Math" => Domain::AdvancedMath,
            "Problem Solving & Data Analysis" => Domain::ProblemSolvingAndDataAnalysis,
            "Geometry & Trigonometry" => Domain::GeometryAndTrigonometry,
            _ => Domain::Algebra,
        }
    }

    pub fn section(&self) -> Section {
        match self {
            Domain::CraftAndStructure
            | Domain::InformationAndIdeas
            | Domain::StandardEnglishConventions
            | Domain::ExpressionOfIdeas => Section::English,
            _ => Section::Math,
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Domain::CraftAndStructure => "Craft",
            Domain::InformationAndIdeas => "Info",
            Domain::StandardEnglishConventions => "Conventions",
            Domain::ExpressionOfIdeas => "Expression",
            Domain::Algebra => "Algebra",
            Domain::AdvancedMath => "Adv Math",
            Domain::ProblemSolvingAndDataAnalysis => "Data",
            Domain::GeometryAndTrigonometry => "Geo/Trig",
        }
    }

    pub fn all() -> Vec<Domain> {
        vec![
            Domain::CraftAndStructure,
            Domain::InformationAndIdeas,
            Domain::StandardEnglishConventions,
            Domain::ExpressionOfIdeas,
            Domain::Algebra,
            Domain::AdvancedMath,
            Domain::ProblemSolvingAndDataAnalysis,
            Domain::GeometryAndTrigonometry,
        ]
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single SAT question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: i64,
    pub section: String,
    pub domain: String,
    pub sub_domain: String,
    pub source: String,
    pub difficulty: i64,
    pub question_text: String,
    pub option_a: String,
    pub option_b: String,
    pub option_c: String,
    pub option_d: String,
    pub correct_answer: String,
    pub explanation: String,
}

impl Question {
    pub fn difficulty_label(&self) -> &'static str {
        match self.difficulty {
            1 => "Easy",
            2 => "Medium",
            3 => "Hard",
            _ => "Unknown",
        }
    }

    pub fn difficulty_color(&self) -> ratatui::style::Color {
        match self.difficulty {
            1 => ratatui::style::Color::Green,
            2 => ratatui::style::Color::Yellow,
            3 => ratatui::style::Color::Red,
            _ => ratatui::style::Color::White,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UserProgress {
    pub id: i64,
    pub question_id: i64,
    pub is_correct: bool,
    pub time_spent_secs: i64,
    pub answered_at: String,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct DomainStats {
    pub domain: String,
    pub total_attempted: i64,
    pub total_correct: i64,
    pub accuracy: f64,
    pub avg_time_secs: f64,
}

/// Daily activity for heatmap
#[allow(dead_code)]
pub struct DailyActivity {
    pub date: String,
    pub questions_answered: i64,
    pub correct: i64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SpacedRepCard {
    pub question_id: i64,
    pub ease_factor: f64,
    pub interval_days: i64,
    pub repetitions: i64,
    pub next_review: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StudySession {
    pub id: i64,
    pub start_time: String,
    pub end_time: Option<String>,
    pub questions_answered: i64,
    pub correct_count: i64,
    pub domain_focus: String,
}

/// Answer choice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    A,
    B,
    C,
    D,
}

impl Answer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Answer::A => "A",
            Answer::B => "B",
            Answer::C => "C",
            Answer::D => "D",
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Answer::A),
            1 => Some(Answer::B),
            2 => Some(Answer::C),
            3 => Some(Answer::D),
            _ => None,
        }
    }
}

impl std::fmt::Display for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// SAT Mock Exam Section tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MockSection {
    ReadingWriting,
    Math,
    Break,
    Finished,
}

/// Tracking for a full mock exam session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockExamState {
    pub section: MockSection,
    pub module: u8, // 1 or 2
    pub questions: Vec<Question>,
    pub user_answers: Vec<Option<usize>>, // None if skipped/unanswered
    pub current_index: usize,
    pub time_remaining_secs: u64,
    pub rw_score: Option<u16>,
    pub math_score: Option<u16>,
    /// Precomputed shuffled option texts and correct index for each question
    #[serde(skip)]
    pub shuffled_options: Vec<MockShuffledOptions>,
}

/// Shuffled options for a single mock exam question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockShuffledOptions {
    pub texts: [String; 4],
    pub correct_index: usize,
}

impl MockExamState {
    pub fn new(questions: Vec<Question>, section: MockSection, duration_secs: u64) -> Self {
        let len = questions.len();
        Self {
            section,
            module: 1,
            questions,
            user_answers: vec![None; len],
            current_index: 0,
            time_remaining_secs: duration_secs,
            rw_score: None,
            math_score: None,
            shuffled_options: Vec::new(),
        }
    }

    pub fn new_with_shuffles(
        questions: Vec<Question>,
        shuffled_options: Vec<MockShuffledOptions>,
        section: MockSection,
        duration_secs: u64,
    ) -> Self {
        let len = questions.len();
        Self {
            section,
            module: 1,
            questions,
            user_answers: vec![None; len],
            current_index: 0,
            time_remaining_secs: duration_secs,
            rw_score: None,
            math_score: None,
            shuffled_options,
        }
    }
}
