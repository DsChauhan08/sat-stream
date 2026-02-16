mod app;
mod config;
mod db;
mod engine;
mod models;
mod seed;
mod ai;
mod ui;
mod pdf_extract;

use app::{App, Feedback, InputTarget, PersistedState, Screen};
use config::Config;
use engine::QuizMode;
use models::Answer;

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(33); // ~30fps

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Load config
    let config = Config::load().unwrap_or_default();

    // Initialize database
    let db_path = Config::db_path();
    let db_path_str = db_path.to_string_lossy().to_string();
    let pool = db::init_db(&db_path_str).await?;

    // Seed questions if database is empty
    seed::seed_if_empty(&pool).await?;

    // Initialize app state
    let mut app = App::new(config);

    // Load persisted state
    if let Some(state) = PersistedState::load() {
        app.session_questions = state.session_questions;
        app.session_correct = state.session_correct;
        app.section_filter = state.section_filter;
        app.domain_filter = state.domain_filter;
    }

    // Load overall stats
    let (total, correct, _) = db::get_overall_stats(&pool).await.unwrap_or((0, 0, 0));
    app.total_answered = total;
    app.total_correct = correct;
    app.current_streak = db::get_current_streak(&pool).await.unwrap_or(0);
    app.best_streak = app.current_streak;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &pool).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Save state on exit
    let _ = PersistedState::save(&app);
    if let Some(sid) = app.session_id {
        let _ = db::update_session(&pool, sid, app.session_questions, app.session_correct).await;
    }

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    pool: &sqlx::SqlitePool,
) -> Result<()> {
    let mut last_tick = Instant::now();

    loop {
        // Render
        terminal.draw(|frame| {
            ui::render(frame, app);
        })?;

        // Handle events with timeout
        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Global keys
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.running = false;
                }

                // Text input mode — intercept all keys
                if app.input_active {
                    match key.code {
                        KeyCode::Esc => {
                            app.close_input();
                        }
                        KeyCode::Enter => {
                            let value = app.input_buffer.clone();
                            let target = app.input_target;
                            app.close_input();
                            match target {
                                InputTarget::ApiKey => {
                                    if value.trim().is_empty() {
                                        app.config.gemini_api_key = None;
                                        app.ai_client = ai::AiClient::new(None);
                                        app.set_status("✓ API key cleared");
                                    } else {
                                        app.config.gemini_api_key = Some(value.trim().to_string());
                                        app.ai_client = ai::AiClient::new(app.config.gemini_api_key.clone());
                                        app.set_status("✓ API key saved — AI features enabled!");
                                    }
                                    let _ = app.config.save();
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    // Number keys for quick navigation
                    KeyCode::Char('1') if !matches!(app.screen, Screen::Quiz if !app.answered) => {
                        app.navigate(Screen::Home);
                    }
                    KeyCode::Char('2') if !matches!(app.screen, Screen::Quiz if !app.answered) => {
                        load_question(app, pool).await;
                        app.navigate(Screen::Quiz);
                    }
                    KeyCode::Char('3') if !matches!(app.screen, Screen::Quiz if !app.answered) => {
                        refresh_stats(app, pool).await;
                        app.navigate(Screen::Stats);
                    }
                    KeyCode::Char('4') if !matches!(app.screen, Screen::Quiz if !app.answered) => {
                        app.navigate(Screen::Settings);
                    }
                    KeyCode::Char('5') if !matches!(app.screen, Screen::Quiz if !app.answered) => {
                        app.navigate(Screen::Help);
                    }

                    // Theme cycling (global)
                    KeyCode::Char('t') | KeyCode::Char('T')
                        if app.screen == Screen::Settings =>
                    {
                        app.cycle_theme();
                    }

                    _ => {
                        // Screen-specific handling
                        match app.screen {
                            Screen::Home => handle_home_keys(app, pool, key.code).await,
                            Screen::Quiz => handle_quiz_keys(app, pool, key.code).await,
                            Screen::Stats => handle_stats_keys(app, key.code),
                            Screen::Review => handle_review_keys(app, key.code),
                            Screen::Settings => handle_settings_keys(app, pool, key.code).await,
                            Screen::Help => handle_help_keys(app, key.code),
                        }
                    }
                }
            }
        }

        // Tick
        if last_tick.elapsed() >= TICK_RATE {
            app.tick = app.tick.wrapping_add(1);

            // Feedback timer countdown
            if app.feedback_timer > 0 {
                app.feedback_timer -= 1;
                if app.feedback_timer == 0 {
                    app.feedback = Feedback::None;
                }
            }

            // Status message countdown
            if app.status_timer > 0 {
                app.status_timer -= 1;
                if app.status_timer == 0 {
                    app.status_message = None;
                }
            }

            // Timed mode countdown
            if app.screen == Screen::Quiz && !app.answered {
                if let Some(ref mut time) = app.time_remaining_secs {
                    if let Some(start) = app.question_start_time {
                        let elapsed = start.elapsed().as_secs();
                        if elapsed > 0 {
                            *time = time.saturating_sub(1);
                            if *time == 0 {
                                // Time's up — mark as wrong
                                if let Some(q) = &app.current_question {
                                    let _ = db::record_answer(pool, q.id, false, elapsed as i64).await;
                                    app.answered = true;
                                    app.feedback = Feedback::Wrong;
                                    app.feedback_timer = 60;
                                    app.session_questions += 1;
                                    app.total_answered += 1;
                                    app.current_streak = 0;
                                }
                            }
                        }
                    }
                }
            }

            last_tick = Instant::now();
        }

        if !app.running {
            return Ok(());
        }
    }
}

async fn handle_home_keys(app: &mut App, pool: &sqlx::SqlitePool, key: KeyCode) {
    if app.mode_selector_open {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.mode_selected > 0 {
                    app.mode_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let modes = QuizMode::all();
                if app.mode_selected < modes.len() - 1 {
                    app.mode_selected += 1;
                }
            }
            KeyCode::Enter => {
                let modes = QuizMode::all();
                app.quiz_mode = modes[app.mode_selected];
                app.mode_selector_open = false;
                // Start quiz with selected mode
                app.session_questions = 0;
                app.session_correct = 0;
                let sid = db::start_session(pool, app.quiz_mode.name()).await.unwrap_or(0);
                app.session_id = Some(sid);
                load_question(app, pool).await;
                app.navigate(Screen::Quiz);
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                app.mode_selector_open = false;
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.home_selected > 0 {
                app.home_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.home_selected < 3 {
                app.home_selected += 1;
            }
        }
        KeyCode::Enter => {
            match app.home_selected {
                0 => {
                    // Start Quiz — show mode selector
                    app.mode_selector_open = true;
                    app.mode_selected = 0;
                }
                1 => {
                    // View Analytics
                    refresh_stats(app, pool).await;
                    app.navigate(Screen::Stats);
                }
                2 => app.navigate(Screen::Settings),
                3 => app.navigate(Screen::Help),
                _ => {}
            }
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            app.mode_selector_open = true;
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.running = false;
        }
        KeyCode::Char('?') => app.navigate(Screen::Help),
        _ => {}
    }
}

async fn handle_quiz_keys(app: &mut App, pool: &sqlx::SqlitePool, key: KeyCode) {
    if app.answered {
        // After answering, waiting for next action
        match key {
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Next question
                app.answered = false;
                app.feedback = Feedback::None;
                app.ai_response = None;
                load_question(app, pool).await;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // AI explanation
                if !app.ai_loading {
                    if let Some(q) = &app.current_question {
                        let q_text = q.question_text.clone();
                        let correct = q.correct_answer.clone();
                        let domain = q.domain.clone();
                        let user_ans = Answer::from_index(app.selected_answer)
                            .map(|a| a.to_string())
                            .unwrap_or_default();

                        app.ai_loading = true;
                        let response = app.ai_client.explain_answer(
                            &q_text, &correct, &user_ans, &domain
                        ).await;
                        app.ai_loading = false;
                        app.ai_response = Some(match response {
                            ai::AiResponse::Success(text) => text,
                            ai::AiResponse::Offline(msg) => msg,
                            ai::AiResponse::Error(msg) => msg,
                        });
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                refresh_stats(app, pool).await;
                app.navigate(Screen::Review);
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                app.navigate(Screen::Home);
            }
            _ => {}
        }
        return;
    }

    // During question answering
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_answer > 0 {
                app.selected_answer -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_answer < 3 {
                app.selected_answer += 1;
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => app.selected_answer = 0,
        KeyCode::Char('b') | KeyCode::Char('B') => app.selected_answer = 1,
        KeyCode::Char('c') | KeyCode::Char('C') => app.selected_answer = 2,
        KeyCode::Char('d') | KeyCode::Char('D') => app.selected_answer = 3,
        KeyCode::Enter => {
            // Submit answer
            if let Some(q) = &app.current_question {
                let user_answer = Answer::from_index(app.selected_answer)
                    .map(|a| a.to_string())
                    .unwrap_or_default();

                let is_correct = user_answer == q.correct_answer;

                let time_spent = app.question_start_time
                    .map(|t| t.elapsed().as_secs() as i64)
                    .unwrap_or(0);

                let _ = db::record_answer(pool, q.id, is_correct, time_spent).await;

                app.session_questions += 1;
                app.total_answered += 1;

                if is_correct {
                    app.session_correct += 1;
                    app.total_correct += 1;
                    app.current_streak += 1;
                    if app.current_streak > app.best_streak {
                        app.best_streak = app.current_streak;
                    }
                    app.feedback = Feedback::Correct;
                } else {
                    app.current_streak = 0;
                    app.feedback = Feedback::Wrong;
                    // Store for review
                    app.wrong_answers.push((q.clone(), user_answer));
                    if app.wrong_answers.len() > 50 {
                        app.wrong_answers.remove(0); // Keep last 50
                    }
                }

                app.answered = true;
                app.feedback_timer = 90; // ~3 seconds at 30fps
            }
        }
        KeyCode::Char('h') | KeyCode::Char('H') => {
            // AI hint
            if !app.ai_loading {
                if let Some(q) = &app.current_question {
                    let q_text = q.question_text.clone();
                    let domain = q.domain.clone();
                    app.ai_loading = true;
                    let response = app.ai_client.get_hint(&q_text, &domain).await;
                    app.ai_loading = false;
                    app.ai_response = Some(match response {
                        ai::AiResponse::Success(text) => text,
                        ai::AiResponse::Offline(msg) => msg,
                        ai::AiResponse::Error(msg) => msg,
                    });
                }
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Skip question
            app.feedback = Feedback::None;
            app.ai_response = None;
            load_question(app, pool).await;
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.navigate(Screen::Home);
        }
        _ => {}
    }
}

fn handle_stats_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            app.stats_scroll = app.stats_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.stats_scroll += 1;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.navigate(Screen::Review);
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.navigate(Screen::Home);
        }
        _ => {}
    }
}

fn handle_review_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            app.review_scroll = app.review_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.review_scroll += 1;
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.navigate(Screen::Stats);
        }
        _ => {}
    }
}

async fn handle_settings_keys(app: &mut App, pool: &sqlx::SqlitePool, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.settings_selected > 0 {
                app.settings_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.settings_selected < 6 {
                app.settings_selected += 1;
            }
        }
        KeyCode::Enter => {
            match app.settings_selected {
                0 => app.cycle_theme(),       // Theme
                1 => {                        // API Key
                    let current = app.config.gemini_api_key.clone().unwrap_or_default();
                    app.open_input("Gemini API Key", InputTarget::ApiKey, &current);
                }
                2 => {                        // Timed mode toggle
                    app.config.timed_mode = !app.config.timed_mode;
                    let _ = app.config.save();
                }
                5 => {                        // Import from PDFs (AI)
                    let cwd = std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."));
                    let cwd_str = cwd.to_string_lossy().to_string();

                    // Check for PDFs first
                    let pdf_count = std::fs::read_dir(&cwd_str)
                        .map(|entries| entries
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().extension()
                                .map(|ext| ext.to_ascii_lowercase() == "pdf")
                                .unwrap_or(false))
                            .count())
                        .unwrap_or(0);

                    if pdf_count == 0 {
                        app.set_status("✗ No PDF files found in current directory");
                    } else {
                        app.set_status(&format!("⏳ Found {} PDFs — extracting with AI (Qwen2.5:1.5b)...", pdf_count));

                        match pdf_extract::extract_from_directory(pool, &cwd_str).await {
                            Ok(count) => {
                                let total = db::question_count(pool).await.unwrap_or(0);
                                app.set_status(&format!(
                                    "✓ Extracted {} new questions! Total: {}",
                                    count, total
                                ));
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                if msg.contains("Ollama is not running") {
                                    app.set_status("✗ Start Ollama first: ollama serve && ollama pull qwen2.5:1.5b");
                                } else if msg.contains("not found") {
                                    app.set_status("✗ Run: ollama pull qwen2.5:1.5b");
                                } else {
                                    app.set_status(&format!("✗ {}", msg));
                                }
                            }
                        }
                    }
                }
                6 => {                        // Questions per session
                }
                _ => {}
            }
        }
        KeyCode::Left | KeyCode::Right => {
            match app.settings_selected {
                3 => {  // Math time
                    if key == KeyCode::Left {
                        app.config.math_time_per_question_secs = app.config.math_time_per_question_secs.saturating_sub(5).max(30);
                    } else {
                        app.config.math_time_per_question_secs = (app.config.math_time_per_question_secs + 5).min(300);
                    }
                    let _ = app.config.save();
                }
                4 => {  // English time
                    if key == KeyCode::Left {
                        app.config.english_time_per_question_secs = app.config.english_time_per_question_secs.saturating_sub(5).max(30);
                    } else {
                        app.config.english_time_per_question_secs = (app.config.english_time_per_question_secs + 5).min(300);
                    }
                    let _ = app.config.save();
                }
                6 => {  // Questions per session
                    if key == KeyCode::Left {
                        app.config.questions_per_session = app.config.questions_per_session.saturating_sub(5).max(5);
                    } else {
                        app.config.questions_per_session = (app.config.questions_per_session + 5).min(100);
                    }
                    let _ = app.config.save();
                }
                _ => {}
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.navigate(Screen::Home);
        }
        _ => {}
    }
}

fn handle_help_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.navigate(Screen::Home);
        }
        _ => {}
    }
}

async fn load_question(app: &mut App, pool: &sqlx::SqlitePool) {
    let section = app.section_filter.as_deref();
    let domain = app.domain_filter.as_deref();

    match engine::next_question(pool, app.quiz_mode, section, domain).await {
        Ok(Some(q)) => {
            app.current_question = Some(q);
            app.selected_answer = 0;
            app.answered = false;
            app.feedback = Feedback::None;
            app.ai_response = None;

            // Set up timer if timed mode
            if app.config.timed_mode {
                let time = if app.current_question.as_ref().map(|q| q.section.as_str()) == Some("math") {
                    app.config.math_time_per_question_secs
                } else {
                    app.config.english_time_per_question_secs
                };
                app.time_remaining_secs = Some(time);
            } else {
                app.time_remaining_secs = None;
            }
            app.question_start_time = Some(Instant::now());
        }
        Ok(None) => {
            app.current_question = None;
        }
        Err(_) => {
            app.current_question = None;
        }
    }
}

async fn refresh_stats(app: &mut App, pool: &sqlx::SqlitePool) {
    if let Ok(stats) = db::get_domain_stats(pool).await {
        app.domain_stats = stats;
    }
    if let Ok(activity) = db::get_daily_activity(pool, 84).await { // 12 weeks
        app.daily_activity = activity;
    }
    app.current_streak = db::get_current_streak(pool).await.unwrap_or(0);
}
