mod app;
mod config;
mod db;
mod engine;
mod models;
mod seed;
mod ai;
mod ui;
mod pdf_extract;

use app::{App, AiReceiver, Feedback, InputTarget, PersistedState, Screen, ShuffledOptions};
use config::Config;
use engine::QuizMode;

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::RngCore;
use rand::SeedableRng;
use ratatui::prelude::*;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

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
                            Screen::MockExam => handle_mock_exam_keys(app, key.code),
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
                if app.status_timer == 1 && app.screen == Screen::MockExam {
                    // Trigger Mock Exam module transition
                    if let Some(state) = app.mock_exam_state.clone() {
                        let next_module = state.module + 1;
                        let (next_sec, is_finished) = if state.section == models::MockSection::ReadingWriting {
                            if next_module > 2 {
                                (models::MockSection::Math, false) // Actually we'd do a Break here, but simplify for now
                            } else {
                                (models::MockSection::ReadingWriting, false)
                            }
                        } else {
                            if next_module > 2 {
                                (models::MockSection::Finished, true)
                            } else {
                                (models::MockSection::Math, false)
                            }
                        };

                        if is_finished {
                            app.set_status("🎉 Exam Finished! Go to Analytics to see your score.");
                            app.mock_exam_state = None;
                            app.navigate(Screen::Home);
                        } else {
                            let mod_num = if next_module > 2 { 1 } else { next_module };
                            // Basic difficulty routing mockup: if they answered > 60% of M1 questions right, give Hard M2 (3). Else Easy M2 (2).
                            // A real implementation would score M1 here before generating M2.
                            let mut routing = mod_num;
                            if mod_num == 2 {
                               let answered_correct = state.questions.iter().enumerate().filter(|(i, _)| {
                                   if let Some(ans_idx) = state.user_answers[*i] {
                                       if *i < state.shuffled_options.len() {
                                           ans_idx == state.shuffled_options[*i].correct_index
                                       } else {
                                           false
                                       }
                                   } else { false }
                               }).count();
                               routing = if answered_correct >= (state.questions.len() / 2) { 3 } else { 2 };
                            }

                            if let Ok((questions, shuffles)) = engine::generate_mock_module(pool, next_sec, routing).await {
                                app.mock_exam_state = Some(models::MockExamState::new_with_shuffles(
                                    questions,
                                    shuffles,
                                    next_sec,
                                    if next_sec == models::MockSection::Math { 35 * 60 } else { 32 * 60 },
                                ));
                                app.mock_exam_state.as_mut().unwrap().module = mod_num;
                            }
                        }
                    }
                }
                if app.status_timer == 0 {
                    app.status_message = None;
                }
            }

            // Timed mode countdown (use elapsed time, not frame ticks)
            if app.screen == Screen::Quiz && !app.answered {
                if let Some(start) = app.question_start_time {
                    let total_time = app.time_total_secs.unwrap_or(0);
                    if total_time > 0 {
                        let elapsed = start.elapsed().as_secs();
                        if elapsed >= total_time {
                            // Time's up — mark as wrong
                            if let Some(q) = &app.current_question {
                                let _ = db::record_answer(pool, q.id, false, elapsed as i64).await;
                                app.answered = true;
                                app.feedback = Feedback::Wrong;
                                app.feedback_timer = 60;
                                app.session_questions += 1;
                                app.total_answered += 1;
                                app.current_streak = 0;
                                app.time_remaining_secs = Some(0);
                            }
                        } else {
                            app.time_remaining_secs = Some(total_time - elapsed);
                        }
                    }
                }
            }

            // Drain AI response channel
            if let Some(ref mut receiver) = app.ai_receiver {
                if let Ok(response) = receiver.rx.try_recv() {
                    app.ai_loading = false;
                    app.ai_response = Some(match response {
                        ai::AiResponse::Success(text) => text,
                        ai::AiResponse::Offline(msg) => msg,
                        ai::AiResponse::Error(msg) => msg,
                    });
                    app.ai_receiver = None;
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
            if app.home_selected < 4 {
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
                    // Mock Exam
                    if let Ok((questions, shuffles)) = engine::generate_mock_module(pool, models::MockSection::ReadingWriting, 1).await {
                        app.mock_exam_state = Some(models::MockExamState::new_with_shuffles(
                            questions,
                            shuffles,
                            models::MockSection::ReadingWriting,
                            32 * 60, // 32 minutes for RW Module 1
                        ));
                        app.navigate(Screen::MockExam);
                    } else {
                        app.set_status("✗ Not enough questions in bank to generate Mock Exam");
                    }
                }
                2 => {
                    // View Analytics
                    refresh_stats(app, pool).await;
                    app.navigate(Screen::Stats);
                }
                3 => app.navigate(Screen::Settings),
                4 => app.navigate(Screen::Help),
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
    // ===== STATE 2: Already answered, waiting for user to press Enter for next =====
    if app.answered {
        match key {
            KeyCode::Enter => {
                // Only way to go to next question
                app.answered = false;
                app.feedback = Feedback::None;
                app.ai_response = None;
                load_question(app, pool).await;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // AI explanation (non-blocking)
                if !app.ai_loading {
                    if let Some(q) = &app.current_question {
                        let q_text = q.question_text.clone();
                        let correct_text = app.shuffled_options.as_ref()
                            .map(|o| o.texts[o.correct_index].clone())
                            .unwrap_or_else(|| q.correct_answer.clone());
                        let domain = q.domain.clone();
                        let user_ans = app.shuffled_options.as_ref()
                            .map(|o| o.texts[app.selected_answer].clone())
                            .unwrap_or_default();

                        let ai_client = app.ai_client.clone_for_spawn();
                        app.ai_loading = true;
                        let (tx, rx) = mpsc::unbounded_channel();
                        app.ai_receiver = Some(AiReceiver { rx });
                        tokio::spawn(async move {
                            let response = ai_client.explain_answer(
                                &q_text, &correct_text, &user_ans, &domain
                            ).await;
                            let _ = tx.send(response);
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

    // ===== STATE 1: Answering the question =====
    match key {
        // Scroll passage with Page Up/Down
        KeyCode::PageUp => {
            app.passage_scroll = app.passage_scroll.saturating_sub(3);
        }
        KeyCode::PageDown => {
            app.passage_scroll = app.passage_scroll.saturating_add(3);
        }
        // Navigate options with arrow keys or j/k
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
        // Direct select with letter keys
        KeyCode::Char('a') | KeyCode::Char('A') => app.selected_answer = 0,
        KeyCode::Char('b') | KeyCode::Char('B') => app.selected_answer = 1,
        KeyCode::Char('c') | KeyCode::Char('C') => app.selected_answer = 2,
        KeyCode::Char('d') | KeyCode::Char('D') => app.selected_answer = 3,
        // SUBMIT answer with Enter - only key that locks in the answer
        KeyCode::Enter => {
            if let Some(q) = &app.current_question {
                let is_correct = if let Some(ref opts) = app.shuffled_options {
                    app.selected_answer == opts.correct_index
                } else {
                    false
                };

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
                    let user_text = app.shuffled_options.as_ref()
                        .map(|o| o.texts[app.selected_answer].clone())
                        .unwrap_or_default();
                    app.wrong_answers.push((q.clone(), user_text));
                    if app.wrong_answers.len() > 50 {
                        app.wrong_answers.remove(0);
                    }
                }

                // Lock the question - user MUST press Enter again to proceed
                app.answered = true;
                app.feedback_timer = 90; // ~3 seconds at 30fps
            }
        }
        // AI hint (non-blocking, doesn't advance)
        KeyCode::Char('h') | KeyCode::Char('H') => {
            if !app.ai_loading {
                if let Some(q) = &app.current_question {
                    let q_text = q.question_text.clone();
                    let domain = q.domain.clone();
                    let ai_client = app.ai_client.clone_for_spawn();
                    app.ai_loading = true;
                    let (tx, rx) = mpsc::unbounded_channel();
                    app.ai_receiver = Some(AiReceiver { rx });
                    tokio::spawn(async move {
                        let response = ai_client.get_hint(&q_text, &domain).await;
                        let _ = tx.send(response);
                    });
                }
            }
        }
        // Quit to home
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
                        app.set_status(&format!("⏳ Found {} PDFs — extracting with llama.cpp...", pdf_count));

                        match pdf_extract::extract_from_directory(pool, &cwd_str).await {
                            Ok((count, summary)) => {
                                let total = db::question_count(pool).await.unwrap_or(0);
                                app.set_status(&format!(
                                    "✓ {} new questions (total: {}) — {}",
                                    count, total, summary
                                ));
                            }
                            Err(e) => {
                                app.set_status(&format!("✗ {}", e));
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

fn handle_mock_exam_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.mock_exam_state = None; // Abort exam
            app.navigate(Screen::Home);
            return;
        }
        KeyCode::Enter => {
            let unanswered = app.mock_exam_state.as_ref()
                .map(|s| s.user_answers.iter().filter(|a| a.is_none()).count())
                .unwrap_or(0);
            
            app.status_timer = 2; // trigger fast transition

            if unanswered > 0 {
                app.set_status(&format!("⚠ {} question(s) left blank. Press Enter again to force submit.", unanswered));
                // We'd normally track a double-tap here, but for now let's just force submit
            } else {
                app.set_status("⏳ Grading module & loading next section...");
            }
            return;
        }
        _ => {}
    }

    if let Some(mock_state) = app.mock_exam_state.as_mut() {
        match key {
            KeyCode::Right | KeyCode::Char('l') => {
                if mock_state.current_index < mock_state.questions.len() - 1 {
                    mock_state.current_index += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if mock_state.current_index > 0 {
                    mock_state.current_index -= 1;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => mock_state.user_answers[mock_state.current_index] = Some(0),
            KeyCode::Char('b') | KeyCode::Char('B') => mock_state.user_answers[mock_state.current_index] = Some(1),
            KeyCode::Char('c') | KeyCode::Char('C') => mock_state.user_answers[mock_state.current_index] = Some(2),
            KeyCode::Char('d') | KeyCode::Char('D') => mock_state.user_answers[mock_state.current_index] = Some(3),
            KeyCode::Char(' ') => {
                // Un-answer (clear selection) if pressed space
                mock_state.user_answers[mock_state.current_index] = None;
            }
            _ => {}
        }
    }
}

async fn load_question(app: &mut App, pool: &sqlx::SqlitePool) {
    let section = app.section_filter.as_deref();
    let domain = app.domain_filter.as_deref();

    match engine::next_question(pool, app.quiz_mode, section, domain).await {
        Ok(Some(q)) => {
            // Shuffle answer options using question ID as seed for consistency
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

            // Fisher-Yates shuffle
            for i in (1..4).rev() {
                let j = (rng.next_u64() as usize) % (i + 1);
                options.swap(i, j);
            }

            // Find where correct answer ended up
            let correct_index = options.iter().position(|o| *o == correct_text).unwrap_or(0);

            app.shuffled_options = Some(ShuffledOptions {
                texts: options,
                correct_index,
            });

            app.current_question = Some(q);
            app.selected_answer = 0;
            app.answered = false;
            app.feedback = Feedback::None;
            app.ai_response = None;
            app.ai_receiver = None;
            app.ai_loading = false;
            app.passage_scroll = 0;

            // Set up timer if timed mode
            if app.config.timed_mode {
                let time = if app.current_question.as_ref().map(|q| q.section.as_str()) == Some("math") {
                    app.config.math_time_per_question_secs
                } else {
                    app.config.english_time_per_question_secs
                };
                app.time_remaining_secs = Some(time);
                app.time_total_secs = Some(time);
            } else {
                app.time_remaining_secs = None;
                app.time_total_secs = None;
            }
            app.question_start_time = Some(Instant::now());
        }
        Ok(None) => {
            app.current_question = None;
            app.shuffled_options = None;
        }
        Err(_) => {
            app.current_question = None;
            app.shuffled_options = None;
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
    app.srs_due_count = db::get_due_questions_count(pool).await.unwrap_or(0);
}
