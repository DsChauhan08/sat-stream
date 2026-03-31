use crate::app::{App, Feedback};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let question = match &app.current_question {
        Some(q) => q,
        None => {
            let loading = Paragraph::new(vec![
                Line::from(""),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "  Loading questions...",
                    Style::default()
                        .fg(theme.dim())
                        .add_modifier(Modifier::ITALIC),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "  Press Q to go back to home",
                    Style::default().fg(theme.dim()),
                )]),
            ]);
            frame.render_widget(loading, area);
            return;
        }
    };

    // Determine border color for feedback animation
    let border_color = match app.feedback {
        Feedback::Correct => theme.success(),
        Feedback::Wrong => theme.error(),
        Feedback::None => theme.accent(),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Question metadata
            Constraint::Min(6),     // Question text (or passage + question)
            Constraint::Length(10), // Options
            Constraint::Length(5),  // Feedback / AI response area
        ])
        .margin(1)
        .split(area);

    // Question metadata bar
    let meta_line = Line::from(vec![
        Span::styled(
            format!("  Question #{}", question.id),
            Style::default()
                .fg(theme.text())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme.dim())),
        Span::styled(&question.domain, Style::default().fg(theme.accent())),
        Span::styled("  │  ", Style::default().fg(theme.dim())),
        Span::styled(
            question.difficulty_label(),
            Style::default().fg(question.difficulty_color()),
        ),
        Span::styled("  │  ", Style::default().fg(theme.dim())),
        Span::styled(
            format!("Source: {}", question.source),
            Style::default().fg(theme.dim()),
        ),
        if let Some(time) = app.time_remaining_secs {
            Span::styled(
                format!("  │  ⏱ {}:{:02}", time / 60, time % 60),
                Style::default()
                    .fg(if time < 30 {
                        theme.error()
                    } else {
                        theme.secondary()
                    })
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        },
    ]);

    let meta = Paragraph::new(meta_line).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.dim()))
            .style(Style::default().bg(theme.surface())),
    );
    frame.render_widget(meta, chunks[0]);

    // Question text (with passage if present)
    let mut q_lines: Vec<Line> = Vec::new();

    if !question.passage.is_empty() {
        // Show passage in a distinct style
        q_lines.push(Line::from(vec![Span::styled(
            "  ── Passage ──────────────────────────────────",
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
            "  ─────────────────────────────────────────────",
            Style::default().fg(theme.dim()),
        )]));
        q_lines.push(Line::from(""));
    }

    q_lines.push(Line::from(vec![Span::styled(
        format!("  {}", question.question_text),
        Style::default().fg(theme.text()),
    )]));

    let q_text = Paragraph::new(q_lines)
        .wrap(Wrap { trim: false })
        .scroll((app.passage_scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(if question.passage.is_empty() {
                    " Question ".to_string()
                } else {
                    format!(" Question (↑↓ scroll) ")
                })
                .title_style(
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                )
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.surface())),
        );
    frame.render_widget(q_text, chunks[1]);

    // Answer options (use shuffled if available)
    let option_letters = ["A", "B", "C", "D"];
    let option_texts: Vec<String> = if let Some(ref opts) = app.shuffled_options {
        opts.texts.to_vec()
    } else {
        vec![
            question.option_a.clone(),
            question.option_b.clone(),
            question.option_c.clone(),
            question.option_d.clone(),
        ]
    };
    let correct_idx = app
        .shuffled_options
        .as_ref()
        .map(|o| o.correct_index)
        .unwrap_or(0);

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
        let is_selected = i == app.selected_answer;
        let is_correct = i == correct_idx;

        let (prefix_style, text_style) = if app.answered {
            if is_correct {
                (
                    Style::default()
                        .fg(theme.bg())
                        .bg(theme.success())
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.success()),
                )
            } else if is_selected && !is_correct {
                (
                    Style::default()
                        .fg(theme.bg())
                        .bg(theme.error())
                        .add_modifier(Modifier::BOLD),
                    Style::default()
                        .fg(theme.error())
                        .add_modifier(Modifier::CROSSED_OUT),
                )
            } else {
                (
                    Style::default().fg(theme.dim()),
                    Style::default().fg(theme.dim()),
                )
            }
        } else if is_selected {
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

        let pointer = if is_selected && !app.answered {
            " ▶ "
        } else {
            "   "
        };

        let option_line = Line::from(vec![
            Span::styled(pointer, Style::default().fg(theme.accent())),
            Span::styled(format!(" {} ", letter), prefix_style),
            Span::styled(format!("  {}", text), text_style),
        ]);

        let option = Paragraph::new(option_line);
        frame.render_widget(option, option_chunks[i]);
    }

    // Feedback / AI response
    let feedback_content = match app.feedback {
        Feedback::Correct => {
            let sparkles = match (app.tick / 4) % 3 {
                0 => "✨ 🎉 ✨",
                1 => "🎉 ✨ 🎉",
                _ => "✨ ✨ ✨",
            };
            vec![Line::from(vec![
                Span::styled(
                    format!("  {}  Correct! ", sparkles),
                    Style::default()
                        .fg(theme.success())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("+{} streak  ", app.current_streak),
                    Style::default().fg(theme.secondary()),
                ),
                Span::styled(
                    "Press Enter for next →",
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::BOLD),
                ),
            ])]
        }
        Feedback::Wrong => {
            let correct_letter = option_letters[correct_idx];
            vec![
                Line::from(vec![
                    Span::styled(
                        format!("  ✗ Incorrect. The answer is {}. ", correct_letter),
                        Style::default()
                            .fg(theme.error())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "Press E for AI explanation.",
                        Style::default().fg(theme.dim()),
                    ),
                ]),
                Line::from(vec![Span::styled(
                    "  Press Enter for next →",
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::BOLD),
                )]),
            ]
        }
        Feedback::None => {
            if let Some(response) = &app.ai_response {
                vec![Line::from(vec![
                    Span::styled(
                        "  🤖 AI: ",
                        Style::default()
                            .fg(theme.secondary())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        response.chars().take(200).collect::<String>(),
                        Style::default().fg(theme.text()),
                    ),
                ])]
            } else if app.ai_loading {
                let dots = ".".repeat(((app.tick / 6) % 4) as usize);
                vec![Line::from(vec![Span::styled(
                    format!("  🤖 Thinking{}", dots),
                    Style::default()
                        .fg(theme.secondary())
                        .add_modifier(Modifier::ITALIC),
                )])]
            } else {
                vec![Line::from(vec![
                    Span::styled("  ⌨ ", Style::default().fg(theme.accent())),
                    Span::styled(
                        "Press A/B/C/D or ↑↓ to select, then ",
                        Style::default().fg(theme.dim()),
                    ),
                    Span::styled(
                        "Enter",
                        Style::default()
                            .fg(theme.accent())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to submit", Style::default().fg(theme.dim())),
                ])]
            }
        }
    };

    let feedback = Paragraph::new(feedback_content)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.bg())),
        );
    frame.render_widget(feedback, chunks[3]);
}
