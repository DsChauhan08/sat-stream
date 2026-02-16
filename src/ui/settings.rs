use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Padding, List, ListItem},
    Frame,
};
use crate::app::App;
use crate::config::Theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),     // Title
            Constraint::Length(2),     // Spacer
            Constraint::Min(10),       // Settings list
        ])
        .margin(1)
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  ⚙️  Settings & Configuration",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(title, chunks[0]);

    let settings_items = vec![
        (
            "Color Theme",
            theme.name().to_string(),
            "Press T to cycle through themes",
            0,
        ),
        (
            "Gemini API Key",
            if app.config.gemini_api_key.is_some() {
                "✓ Configured".to_string()
            } else {
                "✗ Not set".to_string()
            },
            "Press K to set your API key",
            1,
        ),
        (
            "Timed Mode",
            if app.config.timed_mode { "ON" } else { "OFF" }.to_string(),
            "Enforce SAT timing constraints per question",
            2,
        ),
        (
            "Math Time/Question",
            format!("{} seconds", app.config.math_time_per_question_secs),
            "SAT average: 95 seconds per math question",
            3,
        ),
        (
            "English Time/Question",
            format!("{} seconds", app.config.english_time_per_question_secs),
            "SAT average: 71 seconds per R&W question",
            4,
        ),
        (
            "Questions/Session",
            format!("{}", app.config.questions_per_session),
            "Number of questions per study session",
            5,
        ),
    ];

    let items: Vec<ListItem> = settings_items
        .iter()
        .map(|(label, value, desc, idx)| {
            let selected = *idx == app.settings_selected;
            let pointer = if selected { " ▶ " } else { "   " };
            let label_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text())
            };

            let value_color = if value.starts_with('✓') {
                theme.success()
            } else if value.starts_with('✗') {
                theme.error()
            } else if value == "ON" {
                theme.success()
            } else if value == "OFF" {
                theme.dim()
            } else {
                theme.secondary()
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(pointer, Style::default().fg(theme.accent())),
                    Span::styled(*label, label_style),
                    Span::styled("  →  ", Style::default().fg(theme.dim())),
                    Span::styled(value.as_str(), Style::default().fg(value_color).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(*desc, Style::default().fg(theme.dim()).add_modifier(Modifier::ITALIC)),
                ]),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(" Configuration ")
                .title_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
                .padding(Padding::uniform(1))
                .style(Style::default().bg(theme.surface())),
        );
    frame.render_widget(list, chunks[2]);
}
