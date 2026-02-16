use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Padding, Wrap},
    Frame,
};
use crate::app::{App, Feedback};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let question = match &app.current_question {
        Some(q) => q,
        None => {
            let loading = Paragraph::new(vec![
                Line::from(""),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "  Loading questions...",
                        Style::default().fg(theme.dim()).add_modifier(Modifier::ITALIC),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "  Press Q to go back to home",
                        Style::default().fg(theme.dim()),
                    ),
                ]),
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
            Constraint::Length(3),     // Question metadata
            Constraint::Min(6),        // Question text
            Constraint::Length(12),     // Options
            Constraint::Length(4),      // Feedback / AI response area
        ])
        .margin(1)
        .split(area);

    // Question metadata bar
    let meta_line = Line::from(vec![
        Span::styled(
            format!("  Question #{}", question.id),
            Style::default().fg(theme.text()).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme.dim())),
        Span::styled(
            &question.domain,
            Style::default().fg(theme.accent()),
        ),
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
                    .fg(if time < 30 { theme.error() } else { theme.secondary() })
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        },
    ]);

    let meta = Paragraph::new(meta_line)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.dim()))
                .style(Style::default().bg(theme.surface())),
        );
    frame.render_widget(meta, chunks[0]);

    // Question text
    let q_text = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {}", question.question_text),
                Style::default().fg(theme.text()),
            ),
        ]),
    ])
    .wrap(Wrap { trim: false })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Question ")
            .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
            .padding(Padding::horizontal(1))
            .style(Style::default().bg(theme.surface())),
    );
    frame.render_widget(q_text, chunks[1]);

    // Answer options
    let options = [
        ("A", &question.option_a),
        ("B", &question.option_b),
        ("C", &question.option_c),
        ("D", &question.option_d),
    ];

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

    for (i, (letter, text)) in options.iter().enumerate() {
        let is_selected = i == app.selected_answer;
        let is_correct = *letter == question.correct_answer.as_str();

        let (prefix_style, text_style) = if app.answered {
            if is_correct {
                (
                    Style::default().fg(theme.bg()).bg(theme.success()).add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.success()),
                )
            } else if is_selected && !is_correct {
                (
                    Style::default().fg(theme.bg()).bg(theme.error()).add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.error()).add_modifier(Modifier::CROSSED_OUT),
                )
            } else {
                (
                    Style::default().fg(theme.dim()),
                    Style::default().fg(theme.dim()),
                )
            }
        } else if is_selected {
            (
                Style::default().fg(theme.bg()).bg(theme.accent()).add_modifier(Modifier::BOLD),
                Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(theme.text()),
                Style::default().fg(theme.text()),
            )
        };

        let pointer = if is_selected && !app.answered { " ▶ " } else { "   " };

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
                    format!(" +1 Streak ({})", app.current_streak),
                    Style::default().fg(theme.secondary()),
                ),
            ])]
        }
        Feedback::Wrong => {
            vec![
                Line::from(vec![
                    Span::styled(
                        format!("  ✗ Incorrect. The answer is {}. ", question.correct_answer),
                        Style::default()
                            .fg(theme.error())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "Press E for AI explanation.",
                        Style::default().fg(theme.dim()),
                    ),
                ]),
            ]
        }
        Feedback::None => {
            if let Some(response) = &app.ai_response {
                vec![
                    Line::from(vec![
                        Span::styled(
                            "  🤖 AI: ",
                            Style::default().fg(theme.secondary()).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            response.chars().take(200).collect::<String>(),
                            Style::default().fg(theme.text()),
                        ),
                    ]),
                ]
            } else if app.ai_loading {
                let dots = ".".repeat(((app.tick / 6) % 4) as usize);
                vec![Line::from(vec![
                    Span::styled(
                        format!("  🤖 Thinking{}", dots),
                        Style::default().fg(theme.secondary()).add_modifier(Modifier::ITALIC),
                    ),
                ])]
            } else {
                vec![Line::from("")]
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
