use crate::app::App;
use crate::models::MockSection;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let state = match &app.mock_exam_state {
        Some(s) => s,
        None => return,
    };

    let question = &state.questions[state.current_index];
    let user_answer = state.user_answers[state.current_index];

    // Use shuffled options if available
    let option_letters = ["A", "B", "C", "D"];
    let option_texts: Vec<String> = if state.current_index < state.shuffled_options.len() {
        state.shuffled_options[state.current_index].texts.to_vec()
    } else {
        vec![
            question.option_a.clone(),
            question.option_b.clone(),
            question.option_c.clone(),
            question.option_d.clone(),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Top bar (Timer & Section)
            Constraint::Min(6),     // Question text
            Constraint::Length(12), // Options
            Constraint::Length(3),  // Progress dots/bar
        ])
        .margin(1)
        .split(area);

    // Top Metadata Bar
    let section_name = match state.section {
        MockSection::ReadingWriting => "Reading & Writing",
        MockSection::Math => "Math",
        _ => "",
    };
    let time_remaining = state.time_remaining_secs;

    let meta_line = Line::from(vec![
        Span::styled(
            format!("  {} Module {}  ", section_name, state.module),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(theme.dim())),
        Span::styled(
            format!(
                "  Question {} of {}  ",
                state.current_index + 1,
                state.questions.len()
            ),
            Style::default().fg(theme.text()),
        ),
        Span::styled("│", Style::default().fg(theme.dim())),
        Span::styled(
            format!("  ⏱ {}:{:02}  ", time_remaining / 60, time_remaining % 60),
            Style::default()
                .fg(if time_remaining < 300 {
                    theme.warning()
                } else {
                    theme.secondary()
                })
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let meta = Paragraph::new(meta_line)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.dim()))
                .style(Style::default().bg(theme.surface())),
        );
    frame.render_widget(meta, chunks[0]);

    // Question text (with passage if present)
    let mut q_lines: Vec<Line> = Vec::new();

    if !question.passage.is_empty() {
        q_lines.push(Line::from(vec![Span::styled(
            "  ── Passage ──",
            Style::default().fg(theme.dim()),
        )]));
        for line in question.passage.split('\n') {
            q_lines.push(Line::from(vec![Span::styled(
                format!("  {}", line),
                Style::default()
                    .fg(theme.text())
                    .add_modifier(Modifier::ITALIC),
            )]));
        }
        q_lines.push(Line::from(vec![Span::styled(
            "  ─────────────",
            Style::default().fg(theme.dim()),
        )]));
        q_lines.push(Line::from(""));
    }

    q_lines.push(Line::from(vec![Span::styled(
        format!("  {}", question.question_text),
        Style::default().fg(theme.text()),
    )]));

    let q_text = Paragraph::new(q_lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.dim()))
            .title(" Question ")
            .padding(Padding::horizontal(1))
            .style(Style::default().bg(theme.surface())),
    );
    frame.render_widget(q_text, chunks[1]);

    // Answer options (use shuffled if available)
    let option_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(chunks[2]);

    for i in 0..4 {
        let letter = option_letters[i];
        let text = &option_texts[i];
        let is_selected = Some(i) == user_answer;

        let (prefix_style, text_style) = if is_selected {
            (
                Style::default()
                    .fg(theme.bg())
                    .bg(theme.accent())
                    .add_modifier(Modifier::BOLD),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(theme.text()),
                Style::default().fg(theme.text()),
            )
        };

        let pointer = if is_selected { " ▶ " } else { "   " };

        let option_line = Line::from(vec![
            Span::styled(pointer, Style::default().fg(theme.accent())),
            Span::styled(format!(" {} ", letter), prefix_style),
            Span::styled(format!("  {}", text), text_style),
        ]);

        let option = Paragraph::new(option_line);
        frame.render_widget(option, option_chunks[i]);
    }

    // Bottom Navigation Dots
    let mut dot_spans = Vec::new();
    for i in 0..state.questions.len() {
        let is_current = i == state.current_index;
        let is_answered = state.user_answers[i].is_some();

        let style = if is_current {
            Style::default().fg(theme.bg()).bg(theme.accent()) // Inverse for current
        } else if is_answered {
            Style::default().fg(theme.accent()) // Blue if answered
        } else {
            Style::default().fg(theme.dim()) // Grey if unanswered
        };

        let dot = if is_current { " ■ " } else { " ● " };
        dot_spans.push(Span::styled(dot, style));
    }

    let progress = Paragraph::new(Line::from(dot_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.bg())),
        );
    frame.render_widget(progress, chunks[3]);
}
