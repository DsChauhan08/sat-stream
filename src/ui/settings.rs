use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Padding, List, ListItem, Clear, Wrap},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),     // Title
            Constraint::Length(1),     // Spacer
            Constraint::Min(10),       // Settings list
            Constraint::Length(2),     // Status message
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
            "🎨 Color Theme",
            theme.name().to_string(),
            "Enter to cycle │ changes instantly",
            0,
        ),
        (
            "🔑 Gemini API Key",
            if app.config.gemini_api_key.is_some() {
                "✓ Configured".to_string()
            } else {
                "✗ Not set".to_string()
            },
            "Enter to set │ enables AI hints & explanations",
            1,
        ),
        (
            "⏱️  Timed Mode",
            if app.config.timed_mode { "ON" } else { "OFF" }.to_string(),
            "Enter to toggle │ enforces SAT timing constraints",
            2,
        ),
        (
            "🧮 Math Time/Question",
            format!("{}s", app.config.math_time_per_question_secs),
            "◀/▶ to adjust │ SAT average: 95s",
            3,
        ),
        (
            "📝 English Time/Question",
            format!("{}s", app.config.english_time_per_question_secs),
            "◀/▶ to adjust │ SAT average: 71s",
            4,
        ),
        (
            "🤖 Import PDF Questions (AI)",
            "pdftotext + llama.cpp".to_string(),
            "Enter to extract │ needs: llama-cli + .gguf model",
            5,
        ),
        (
            "🔢 Questions Per Session",
            format!("{}", app.config.questions_per_session),
            "◀/▶ to adjust │ range: 5–100",
            6,
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

    // Status message
    if let Some(ref msg) = app.status_message {
        let color = if msg.starts_with('✓') {
            theme.success()
        } else if msg.starts_with('✗') {
            theme.error()
        } else {
            theme.warning()
        };
        let status = Paragraph::new(Line::from(vec![
            Span::styled(msg.as_str(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ])).alignment(Alignment::Center);
        frame.render_widget(status, chunks[3]);
    }

    // Text input popup if active
    if app.input_active {
        render_input_popup(frame, app);
    }
}

/// Renders a centered text input popup
fn render_input_popup(frame: &mut Frame, app: &App) {
    let theme = app.theme();
    let area = frame.area();

    // Center a popup
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = 7u16;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Label
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Help
        ])
        .margin(1)
        .split(popup_area);

    // Background
    let bg = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(format!(" {} ", app.input_label))
        .title_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.surface()));
    frame.render_widget(bg, popup_area);

    // Show the current input with a cursor
    let display = if app.input_buffer.is_empty() {
        "Type here...".to_string()
    } else {
        // Mask API key partially
        let buf = &app.input_buffer;
        if buf.len() > 8 {
            format!("{}...{}", &buf[..4], &buf[buf.len()-4..])
        } else {
            buf.clone()
        }
    };

    let cursor_char = if app.tick % 30 < 15 { "█" } else { " " };

    let input_line = Paragraph::new(Line::from(vec![
        Span::styled(
            &display,
            Style::default().fg(if app.input_buffer.is_empty() { theme.dim() } else { theme.text() }),
        ),
        Span::styled(cursor_char, Style::default().fg(theme.accent())),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.dim()))
        .style(Style::default().bg(theme.bg())));
    frame.render_widget(input_line, inner_chunks[1]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
        Span::styled(" save  ", Style::default().fg(theme.dim())),
        Span::styled("Esc", Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", Style::default().fg(theme.dim())),
    ])).alignment(Alignment::Center);
    frame.render_widget(help, inner_chunks[2]);
}
