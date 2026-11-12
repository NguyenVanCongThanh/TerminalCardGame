use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::domain::Rarity;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .margin(1)
        .split(area);

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spin = spinner[(app.tick as usize) % spinner.len()];

    let items: Vec<ListItem<'_>> = app
        .collections
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let cached = app.cached_card_counts.get(&col.id).copied().unwrap_or(0);
            let prog = app
                .collection_progress
                .iter()
                .find(|p| p.collection_id == col.id);
            let owned = prog.map(|p| p.unique_owned).unwrap_or(0);

            let badge = if app.syncing_collection_id.as_deref() == Some(&col.id) {
                Span::styled(
                    format!(" [{spin} SYNCING...]"),
                    Style::default().fg(theme::ACCENT_GOLD),
                )
            } else if cached > 0 {
                Span::styled(
                    format!(" [READY: {cached} cards • {owned} owned]"),
                    Style::default().fg(theme::ACCENT_GREEN),
                )
            } else {
                Span::styled(" [TAP ENTER TO SYNC]", Style::default().fg(theme::ACCENT_CYAN))
            };

            let prefix = if idx == app.collection_idx { "▶ " } else { "  " };
            let release = col.release_date.as_deref().unwrap_or("Unknown");
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{prefix}{}", col.name),
                        Style::default()
                            .fg(theme::TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    badge,
                ]),
                Line::from(Span::styled(
                    format!(
                        "    Series: {} • Released: {} • Full Set: {} cards",
                        col.series, release, col.total_cards_in_set
                    ),
                    theme::muted_style(),
                )),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.collection_idx));

    let title = if app.syncing_collections_list {
        format!(" Collections ({spin} Refreshing...) ")
    } else {
        " Select Collection / Card Set ".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_FOCUS))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(title, theme::title_style())),
        )
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    // Right panel: Collection details + rarity breakdown
    let mut lines = Vec::new();
    if let Some(col) = app.collections.get(app.collection_idx) {
        let cached = app.cached_card_counts.get(&col.id).copied().unwrap_or(0);
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", col.name),
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  Set Code: {}  •  Series: {}", col.id, col.series),
            Style::default().fg(theme::ACCENT_CYAN),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", col.description),
            theme::primary_style(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Cached Cards : ", theme::muted_style()),
            Span::styled(
                format!("{cached} (up to {} per sync)", app.settings.cards_per_sync),
                theme::primary_style(),
            ),
        ]));
        if let Some(ref synced) = col.synced_at {
            let short = synced.get(..19).unwrap_or(synced).replace('T', " ");
            lines.push(Line::from(vec![
                Span::styled("  Last Synced  : ", theme::muted_style()),
                Span::styled(short, theme::primary_style()),
            ]));
        }

        if cached > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Cached Pool Rarity Breakdown:",
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
            for &rarity in Rarity::all() {
                let n = app.cached_rarity_counts.get(&rarity).copied().unwrap_or(0);
                if n > 0 {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("    {:<6} {:<13} : ", rarity.short_badge(), rarity.label()),
                            theme::rarity_style(rarity, app.tick),
                        ),
                        Span::styled(format!("{n} cards"), theme::primary_style()),
                    ]));
                }
            }
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Press [Enter] or [s] to fetch real card metadata & artwork URLs from TCGdex!",
                Style::default().fg(theme::ACCENT_CYAN),
            )));
        }
    }

    let detail = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Collection Overview ", theme::title_style())),
        );
    frame.render_widget(detail, cols[1]);
}

