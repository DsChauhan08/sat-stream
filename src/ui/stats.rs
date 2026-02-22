use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Padding, Wrap},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Title
            Constraint::Length(10),     // Heatmap
            Constraint::Length(2),      // Spacer
            Constraint::Min(8),        // Domain breakdown
        ])
        .margin(1)
        .split(area);

    // Analytics title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  📊 Performance Analytics",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(theme.dim())),
        Span::styled(
            format!("Total: {} answered", app.total_answered),
            Style::default().fg(theme.text()),
        ),
        Span::styled("  │  ", Style::default().fg(theme.dim())),
        Span::styled(
            format!("Accuracy: {:.1}%", app.overall_accuracy()),
            Style::default().fg(if app.overall_accuracy() >= 80.0 {
                theme.success()
            } else {
                theme.warning()
            }),
        ),
        Span::styled(
            format!("🔥 Best streak: {}", app.best_streak),
            Style::default().fg(theme.secondary()),
        ),
        Span::styled("  │  ", Style::default().fg(theme.dim())),
        Span::styled(
            format!("🧠 SRS Due: {}", app.srs_due_count),
            Style::default()
                .fg(if app.srs_due_count > 0 { theme.warning() } else { theme.dim() })
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(title, chunks[0]);

    // Heatmap (contribution graph style)
    render_heatmap(frame, app, chunks[1]);

    // Domain breakdown
    render_domain_breakdown(frame, app, chunks[3]);
}

fn render_heatmap(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            "  Study Activity (Last 12 Weeks)",
            Style::default().fg(theme.text()).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Build heatmap grid: 7 rows (days) x N columns (weeks)
    let weeks = 12;
    let days_per_week = 7;
    let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    // Create activity map from data
    let mut activity_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for da in &app.daily_activity {
        activity_map.insert(da.date.clone(), da.questions_answered);
    }

    // Generate heatmap rows
    let today = chrono::Local::now().date_naive();
    for day_offset in (0..days_per_week).step_by(2) {
        let mut cells: Vec<Span> = Vec::new();
        cells.push(Span::styled(
            format!("  {} ", day_labels[day_offset]),
            Style::default().fg(theme.dim()),
        ));

        for week in (0..weeks).rev() {
            let days_ago = week * 7 + (6 - day_offset as i64);
            let date = today - chrono::Duration::days(days_ago);
            let date_str = date.format("%Y-%m-%d").to_string();
            let count = activity_map.get(&date_str).copied().unwrap_or(0);

            let cell_color = if count == 0 {
                theme.surface()
            } else if count <= 5 {
                Color::Rgb(30, 80, 30)
            } else if count <= 15 {
                Color::Rgb(40, 140, 40)
            } else if count <= 30 {
                Color::Rgb(50, 200, 50)
            } else {
                Color::Rgb(80, 255, 80)
            };

            cells.push(Span::styled("██", Style::default().fg(cell_color)));
            cells.push(Span::raw(" "));
        }
        lines.push(Line::from(cells));
    }

    // Legend
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("        Less ", Style::default().fg(theme.dim())),
        Span::styled("██", Style::default().fg(theme.surface())),
        Span::raw(" "),
        Span::styled("██", Style::default().fg(Color::Rgb(30, 80, 30))),
        Span::raw(" "),
        Span::styled("██", Style::default().fg(Color::Rgb(40, 140, 40))),
        Span::raw(" "),
        Span::styled("██", Style::default().fg(Color::Rgb(50, 200, 50))),
        Span::raw(" "),
        Span::styled("██", Style::default().fg(Color::Rgb(80, 255, 80))),
        Span::styled(" More", Style::default().fg(theme.dim())),
    ]));

    let heatmap = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(" 📅 Activity Heatmap ")
                .title_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.surface())),
        );
    frame.render_widget(heatmap, area);
}

fn render_domain_breakdown(frame: &mut Frame, app: &App, area: Rect) {
    let _theme = app.theme();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // English domains
    render_section_stats(frame, app, chunks[0], "📝 Reading & Writing", &[
        "Craft and Structure",
        "Information and Ideas",
        "Standard English Conventions",
        "Expression of Ideas",
    ]);

    // Math domains
    render_section_stats(frame, app, chunks[1], "🔢 Mathematics", &[
        "Algebra",
        "Advanced Math",
        "Problem Solving & Data Analysis",
        "Geometry & Trigonometry",
    ]);
}

fn render_section_stats(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    domains: &[&str],
) {
    let theme = app.theme();
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for domain_name in domains {
        let stat = app.domain_stats.iter().find(|s| s.domain == *domain_name);

        let (accuracy, attempted, bar_width) = match stat {
            Some(s) => (s.accuracy * 100.0, s.total_attempted, (s.accuracy * 20.0) as usize),
            None => (0.0, 0, 0),
        };

        let bar_color = if accuracy >= 80.0 {
            theme.success()
        } else if accuracy >= 60.0 {
            theme.warning()
        } else if attempted > 0 {
            theme.error()
        } else {
            theme.dim()
        };

        // Domain name (shortened)
        let short_name = if domain_name.len() > 22 {
            format!("{}...", &domain_name[..19])
        } else {
            domain_name.to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:22}", short_name),
                Style::default().fg(theme.text()),
            ),
        ]));

        // Progress bar
        let filled = "█".repeat(bar_width);
        let empty = "░".repeat(20 - bar_width);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(filled, Style::default().fg(bar_color)),
            Span::styled(empty, Style::default().fg(theme.surface())),
            Span::styled(
                format!(" {:.0}% ", accuracy),
                Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({})", attempted),
                Style::default().fg(theme.dim()),
            ),
        ]));
        lines.push(Line::from(""));
    }

    let section = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(format!(" {} ", title))
                .title_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.surface())),
        );
    frame.render_widget(section, area);
}

/// Render the review screen showing wrong answers
pub fn render_review(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    if app.wrong_answers.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  ✨ No wrong answers to review! Keep it up!",
                    Style::default().fg(theme.success()).add_modifier(Modifier::BOLD),
                ),
            ]),
        ]);
        frame.render_widget(empty, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            format!("  📋 Wrong Answers Review ({} questions)", app.wrong_answers.len()),
            Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    for (i, (q, user_answer)) in app.wrong_answers.iter().enumerate() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", i + 1),
                Style::default().fg(theme.secondary()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &q.domain,
                Style::default().fg(theme.accent()),
            ),
            Span::styled(" │ ", Style::default().fg(theme.dim())),
            Span::styled(
                q.difficulty_label(),
                Style::default().fg(q.difficulty_color()),
            ),
        ]));

        // Question text (truncated)
        let q_preview: String = q.question_text.chars().take(80).collect();
        lines.push(Line::from(vec![
            Span::styled(
                format!("     {}", q_preview),
                Style::default().fg(theme.text()),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("     Your: ", Style::default().fg(theme.error())),
            Span::styled(user_answer, Style::default().fg(theme.error())),
            Span::styled("  │  Correct: ", Style::default().fg(theme.success())),
            Span::styled(&q.correct_answer, Style::default().fg(theme.success()).add_modifier(Modifier::BOLD)),
        ]));

        if !q.explanation.is_empty() {
            let exp_preview: String = q.explanation.chars().take(100).collect();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("     💡 {}", exp_preview),
                    Style::default().fg(theme.dim()).add_modifier(Modifier::ITALIC),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }

    let review = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.review_scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent()))
                .title(" Review ")
                .title_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
                .padding(Padding::uniform(1))
                .style(Style::default().bg(theme.surface())),
        );
    frame.render_widget(review, area);
}
