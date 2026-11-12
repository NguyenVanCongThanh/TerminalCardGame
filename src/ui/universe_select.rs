use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .margin(1)
        .split(area);

    let items: Vec<ListItem<'_>> = app
        .universes
        .iter()
        .enumerate()
        .map(|(i, u)| {
            let status = if u.is_implemented {
                Span::styled(" [ACTIVE]", Style::default().fg(theme::ACCENT_GREEN))
            } else {
                Span::styled(" [PLANNED]", theme::muted_style())
            };
            let prefix = if i == app.universe_idx { "▶ " } else { "  " };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{prefix}{} {}", u.badge, u.name),
                        if u.is_implemented {
                            Style::default()
                                .fg(theme::TEXT_PRIMARY)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            theme::muted_style()
                        },
                    ),
                    status,
                ]),
                Line::from(Span::styled(
                    format!("    {}", u.subtitle),
                    theme::muted_style(),
                )),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.universe_idx));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_FOCUS))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Select Card Universe ", theme::title_style())),
        )
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    let detail_lines = if let Some(u) = app.universes.get(app.universe_idx) {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}  {}", u.badge, u.name),
                Style::default()
                    .fg(theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}", u.subtitle),
                Style::default().fg(theme::ACCENT_CYAN),
            )),
            Line::from(""),
            Line::from(Span::styled(format!("  {}", u.description), theme::primary_style())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Provider ID : ", theme::muted_style()),
                Span::styled(u.provider_id.clone(), theme::primary_style()),
            ]),
            Line::from(vec![
                Span::styled("  Status      : ", theme::muted_style()),
                Span::styled(
                    if u.is_implemented {
                        "Ready to Play"
                    } else {
                        "Extensible Architecture Placeholder"
                    },
                    if u.is_implemented {
                        Style::default().fg(theme::ACCENT_GREEN)
                    } else {
                        theme::muted_style()
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  New universes plug in via the `CardDataProvider` trait without altering pack generation, persistence, or UI rendering.",
                theme::muted_style(),
            )),
        ]
    } else {
        vec![Line::from("No universes available.")]
    };

    let detail = Paragraph::new(detail_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Universe Details ", theme::title_style())),
        );
    frame.render_widget(detail, cols[1]);
}

