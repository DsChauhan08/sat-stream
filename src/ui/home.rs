use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Padding, List, ListItem},
    Frame,
};
use crate::app::App;
use crate::engine::QuizMode;

const LOGO: &str = r#"
   _____ ___  ______     _____ __                           
  / ___//   |/_  __/    / ___// /_________  ____ _____ ___  
  \__ \/ /| | / /  ____\__ \/ __/ ___/ _ \/ __ `/ __ `__ \ 
 ___/ / ___ |/ /  /___/___/ / /_/ /  /  __/ /_/ / / / / / / 
/____/_/  |_/_/       /____/\__/_/   \___/\__,_/_/ /_/ /_/  
"#;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),     // Logo
            Constraint::Length(3),     // Tagline
            Constraint::Length(2),     // Spacer
            Constraint::Length(12),    // Menu
            Constraint::Min(0),       // Stats summary
        ])
        .split(area);

    // Logo with animated color
    let logo_color = match (app.tick / 12) % 3 {
        0 => theme.accent(),
        1 => theme.secondary(),
        _ => theme.text(),
    };

    let logo = Paragraph::new(LOGO)
        .style(Style::default().fg(logo_color).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(logo, chunks[0]);

    // Tagline
    let tagline = Paragraph::new(Line::from(vec![
        Span::styled(
            "   Your infinite SAT prep companion ",
            Style::default().fg(theme.dim()).add_modifier(Modifier::ITALIC),
        ),
        Span::styled(
            "• ",
            Style::default().fg(theme.accent()),
        ),
        Span::styled(
            "Powered by AI",
            Style::default().fg(theme.secondary()),
        ),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(tagline, chunks[1]);

    // Menu items
    let menu_items = vec![
        ("🚀", "Start Quiz", "Begin an infinite stream of SAT questions"),
        ("📊", "View Analytics", "See your performance dashboard & heatmap"),
        ("⚙️ ", "Settings", "Configure themes, API keys, and preferences"),
        ("❓", "Help", "View all keyboard shortcuts"),
    ];

    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, (icon, title, desc))| {
            let selected = i == app.home_selected;
            let style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text())
            };

            let pointer = if selected { " ▶ " } else { "   " };
            let bg = if selected { theme.surface() } else { theme.bg() };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(pointer, Style::default().fg(theme.accent())),
                    Span::styled(format!("{} ", icon), style),
                    Span::styled(*title, style),
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(*desc, Style::default().fg(theme.dim())),
                ]),
                Line::from(""),
            ])
        })
        .collect();

    let menu = List::new(items)
        .block(
            Block::default()
                .padding(Padding::horizontal(4))
        );
    frame.render_widget(menu, chunks[3]);

    // Quick stats summary
    if app.total_answered > 0 {
        let stats_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  📈 ", Style::default()),
                Span::styled(
                    format!("{} questions answered", app.total_answered),
                    Style::default().fg(theme.text()),
                ),
                Span::styled("  •  ", Style::default().fg(theme.dim())),
                Span::styled(
                    format!("{:.1}% accuracy", app.overall_accuracy()),
                    Style::default().fg(if app.overall_accuracy() >= 80.0 {
                        theme.success()
                    } else {
                        theme.warning()
                    }),
                ),
                Span::styled("  •  ", Style::default().fg(theme.dim())),
                Span::styled(
                    format!("🔥 {} streak", app.current_streak),
                    Style::default().fg(theme.secondary()),
                ),
            ]),
        ];

        let stats = Paragraph::new(stats_lines)
            .block(Block::default().padding(Padding::horizontal(2)));
        frame.render_widget(stats, chunks[4]);
    }

    // Mode selector popup
    if app.mode_selector_open {
        render_mode_selector(frame, app, area);
    }
}

fn render_mode_selector(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let popup_area = super::centered_rect(50, 50, area);

    // Clear area behind popup
    let clear = Block::default().style(Style::default().bg(theme.bg()));
    frame.render_widget(clear, popup_area);

    let modes = QuizMode::all();
    let items: Vec<ListItem> = modes
        .iter()
        .enumerate()
        .map(|(i, mode)| {
            let selected = i == app.mode_selected;
            let pointer = if selected { " ▶ " } else { "   " };
            let style = if selected {
                Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text())
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(pointer, Style::default().fg(theme.accent())),
                    Span::styled(mode.name(), style),
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(mode.description(), Style::default().fg(theme.dim())),
                ]),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Select Quiz Mode ")
                .title_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .padding(Padding::uniform(1))
                .style(Style::default().bg(theme.surface())),
        );

    frame.render_widget(list, popup_area);
}
