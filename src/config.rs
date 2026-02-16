use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub gemini_api_key: Option<String>,
    pub theme: Theme,
    pub questions_per_session: usize,
    pub timed_mode: bool,
    pub math_time_per_question_secs: u64,
    pub english_time_per_question_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Default,
    Dark,
    Solarized,
    Gruvbox,
}

impl Theme {
    pub fn all() -> Vec<Theme> {
        vec![Theme::Default, Theme::Dark, Theme::Solarized, Theme::Gruvbox]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Theme::Default => "Midnight Blue",
            Theme::Dark => "Deep Dark",
            Theme::Solarized => "Solarized",
            Theme::Gruvbox => "Gruvbox",
        }
    }

    /// Primary accent color
    pub fn accent(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(100, 149, 237),   // Cornflower blue
            Theme::Dark => ratatui::style::Color::Rgb(139, 92, 246),       // Violet
            Theme::Solarized => ratatui::style::Color::Rgb(38, 139, 210),  // Sol blue
            Theme::Gruvbox => ratatui::style::Color::Rgb(215, 153, 33),    // Gruvbox yellow
        }
    }

    /// Secondary accent
    pub fn secondary(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(255, 215, 0),     // Gold
            Theme::Dark => ratatui::style::Color::Rgb(236, 72, 153),       // Pink
            Theme::Solarized => ratatui::style::Color::Rgb(181, 137, 0),   // Sol yellow
            Theme::Gruvbox => ratatui::style::Color::Rgb(152, 151, 26),    // Gruvbox green
        }
    }

    /// Background
    pub fn bg(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(15, 18, 30),
            Theme::Dark => ratatui::style::Color::Rgb(10, 10, 15),
            Theme::Solarized => ratatui::style::Color::Rgb(0, 43, 54),
            Theme::Gruvbox => ratatui::style::Color::Rgb(40, 40, 40),
        }
    }

    /// Surface / panel background
    pub fn surface(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(22, 27, 44),
            Theme::Dark => ratatui::style::Color::Rgb(18, 18, 25),
            Theme::Solarized => ratatui::style::Color::Rgb(7, 54, 66),
            Theme::Gruvbox => ratatui::style::Color::Rgb(50, 48, 47),
        }
    }

    /// Text color
    pub fn text(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(220, 225, 240),
            Theme::Dark => ratatui::style::Color::Rgb(230, 230, 240),
            Theme::Solarized => ratatui::style::Color::Rgb(131, 148, 150),
            Theme::Gruvbox => ratatui::style::Color::Rgb(235, 219, 178),
        }
    }

    /// Dim / muted text
    pub fn dim(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(100, 110, 140),
            Theme::Dark => ratatui::style::Color::Rgb(90, 90, 110),
            Theme::Solarized => ratatui::style::Color::Rgb(88, 110, 117),
            Theme::Gruvbox => ratatui::style::Color::Rgb(146, 131, 116),
        }
    }

    /// Success color
    pub fn success(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(46, 213, 115),
            Theme::Dark => ratatui::style::Color::Rgb(34, 197, 94),
            Theme::Solarized => ratatui::style::Color::Rgb(133, 153, 0),
            Theme::Gruvbox => ratatui::style::Color::Rgb(184, 187, 38),
        }
    }

    /// Error color
    pub fn error(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(255, 71, 87),
            Theme::Dark => ratatui::style::Color::Rgb(239, 68, 68),
            Theme::Solarized => ratatui::style::Color::Rgb(220, 50, 47),
            Theme::Gruvbox => ratatui::style::Color::Rgb(251, 73, 52),
        }
    }

    /// Warning color
    pub fn warning(&self) -> ratatui::style::Color {
        match self {
            Theme::Default => ratatui::style::Color::Rgb(255, 159, 67),
            Theme::Dark => ratatui::style::Color::Rgb(251, 191, 36),
            Theme::Solarized => ratatui::style::Color::Rgb(203, 75, 22),
            Theme::Gruvbox => ratatui::style::Color::Rgb(254, 128, 25),
        }
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gemini_api_key: None,
            theme: Theme::Default,
            questions_per_session: 20,
            timed_mode: false,
            math_time_per_question_secs: 95,
            english_time_per_question_secs: 71,
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("sat-stream")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn data_dir() -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("sat-stream")
    }

    pub fn db_path() -> PathBuf {
        Self::data_dir().join("sat-stream.db")
    }

    pub fn state_path() -> PathBuf {
        Self::data_dir().join("state.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}
