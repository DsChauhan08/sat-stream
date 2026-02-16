pub mod home;
pub mod quiz;
pub mod stats;
pub mod settings;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Padding},
    Frame,
};
use crate::app::{App, Screen};

/// Main render function dispatches to the correct screen
pub fn render(frame: &mut Frame, app: &App) {
    let theme = app.theme();

    // Clear background
    let bg_block = Block::default().style(Style::default().bg(theme.bg()));
    frame.render_widget(bg_block, frame.area());

    // Main layout: header + content + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Header
            Constraint::Min(1),       // Content
            Constraint::Length(3),    // Footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    
    match app.screen {
        Screen::Home => home::render(frame, app, chunks[1]),
        Screen::Quiz => quiz::render(frame, app, chunks[1]),
        Screen::Stats => stats::render(frame, app, chunks[1]),
        Screen::Review => stats::render_review(frame, app, chunks[1]),
        Screen::Settings => settings::render(frame, app, chunks[1]),
        Screen::Help => render_help(frame, app, chunks[1]),
    }

    render_footer(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    
    let screen_name = match app.screen {
        Screen::Home => "Home",
        Screen::Quiz => "Quiz",
        Screen::Stats => "Analytics",
        Screen::Review => "Review",
        Screen::Settings => "Settings",
        Screen::Help => "Help",
    };

    let mode_name = app.quiz_mode.name();
    let accuracy = app.accuracy();
    let streak = app.current_streak;

    // Animated sparkle
    let sparkle = match (app.tick / 8) % 4 {
        0 => "✦",
        1 => "✧",
        2 => "★",
        _ => "✦",
    };

    let header_text = vec![
        Span::styled(
            format!(" {} SAT-Stream ", sparkle),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(theme.dim())),
        Span::styled(
            screen_name,
            Style::default().fg(theme.text()),
        ),
        Span::styled(" │ ", Style::default().fg(theme.dim())),
        Span::styled(
            format!("Mode: {}", mode_name),
            Style::default().fg(theme.secondary()),
        ),
        Span::styled(" │ ", Style::default().fg(theme.dim())),
        Span::styled(
            format!("Accuracy: {:.0}%", accuracy),
            Style::default().fg(if accuracy >= 80.0 {
                theme.success()
            } else if accuracy >= 60.0 {
                theme.warning()
            } else {
                theme.error()
            }),
        ),
        Span::styled(" │ ", Style::default().fg(theme.dim())),
        Span::styled(
            format!("🔥 {}", streak),
            Style::default()
                .fg(theme.secondary())
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let header = Paragraph::new(Line::from(header_text))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.accent()))
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.surface())),
        );

    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let keys = match app.screen {
        Screen::Home => vec![
            ("↑↓", "Navigate"), ("Enter", "Select"), ("Q", "Quit"), ("?", "Help"),
        ],
        Screen::Quiz => {
            if app.answered {
                vec![
                    ("Enter", "Next"), ("E", "Explain (AI)"), ("R", "Review"),
                    ("Q", "Home"),
                ]
            } else {
                vec![
                    ("A-D", "Answer"), ("↑↓", "Select"), ("Enter", "Submit"),
                    ("H", "Hint (AI)"), ("S", "Skip"), ("Q", "Home"),
                ]
            }
        }
        Screen::Stats => vec![
            ("↑↓", "Scroll"), ("R", "Review Wrong"), ("Q", "Home"),
        ],
        Screen::Review => vec![
            ("↑↓", "Scroll"), ("Q", "Back"),
        ],
        Screen::Settings => vec![
            ("↑↓", "Navigate"), ("Enter", "Toggle"), ("T", "Theme"), ("Q", "Home"),
        ],
        Screen::Help => vec![
            ("Q", "Back"),
        ],
    };

    let spans: Vec<Span> = keys
        .iter()
        .enumerate()
        .flat_map(|(i, (key, action))| {
            let mut v = vec![
                Span::styled(
                    format!(" {} ", key),
                    Style::default()
                        .fg(theme.bg())
                        .bg(theme.accent())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {} ", action),
                    Style::default().fg(theme.text()),
                ),
            ];
            if i < keys.len() - 1 {
                v.push(Span::styled(" │ ", Style::default().fg(theme.dim())));
            }
            v
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.accent()))
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.surface())),
        );

    frame.render_widget(footer, area);
}

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let help_items = vec![
        ("Navigation", vec![
            ("↑ / ↓ / k / j", "Navigate menus / Select answer"),
            ("Enter", "Confirm selection / Submit answer"),
            ("1-5", "Quick navigate (Home/Quiz/Stats/Settings/Help)"),
            ("Q / Esc", "Go back / Quit from home"),
        ]),
        ("Quiz Controls", vec![
            ("A / B / C / D", "Select answer directly"),
            ("H", "Get AI hint (requires Gemini API key)"),
            ("S", "Skip current question"),
            ("E", "Get AI explanation (after answering)"),
            ("M", "Change quiz mode"),
        ]),
        ("General", vec![
            ("T", "Cycle color theme"),
            ("K", "Set Gemini API key"),
            ("?", "Show this help screen"),
            ("Ctrl+C", "Force quit"),
        ]),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  ⌨️  Keyboard Shortcuts",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    for (section, items) in &help_items {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  ─── {} ───", section),
                Style::default().fg(theme.secondary()).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        for (key, desc) in items {
            lines.push(Line::from(vec![
                Span::styled(format!("    {:20}", key), Style::default().fg(theme.accent())),
                Span::styled(*desc, Style::default().fg(theme.text())),
            ]));
        }
        lines.push(Line::from(""));
    }

    let help = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(" Help ")
                .title_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
                .padding(Padding::uniform(1))
                .style(Style::default().bg(theme.surface())),
        );

    frame.render_widget(help, area);
}

/// Utility: center a rect inside another
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
