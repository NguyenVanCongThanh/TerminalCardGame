use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Bar, BarChart, BarGroup, LineGauge, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Wrap,
};

use crate::app::{AppContext, CollectionSelectScreen};
use crate::domain::Rarity;
use crate::ui::theme;

pub fn render(
    frame: &mut Frame<'_>,
    ctx: &AppContext,
    screen: &CollectionSelectScreen,
    area: Rect,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .margin(1)
        .split(area);

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spin = spinner[(ctx.tick as usize) % spinner.len()];

    let items: Vec<ListItem<'_>> = ctx
        .collections
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let cached = ctx.cached_card_counts.get(&col.id).copied().unwrap_or(0);
            let prog = ctx
                .collection_progress
                .iter()
                .find(|p| p.collection_id == col.id);
            let owned = prog.map(|p| p.unique_owned).unwrap_or(0);

            let badge = if ctx.syncing_collection_id.as_deref() == Some(&col.id) {
                theme::pill_badge(format!("{spin} SYNCING"), theme::BG_DARK, theme::ACCENT_GOLD)
            } else if cached > 0 {
                theme::pill_badge(
                    format!("READY {cached} • {owned} owned"),
                    theme::BG_DARK,
                    theme::ACCENT_GREEN,
                )
            } else {
                theme::pill_badge("ENTER TO SYNC", theme::BG_DARK, theme::ACCENT_CYAN)
            };

            let prefix = if idx == screen.collection_idx {
                "▶ "
            } else {
                "  "
            };
            let release = col.release_date.as_deref().unwrap_or("Unknown");
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{prefix}{} ", col.name),
                        Style::default()
                            .fg(theme::TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    badge,
                ]),
                Line::from(Span::styled(
                    format!(
                        "  Series: {} • Released: {} • Full Set: {} cards",
                        col.series, release, col.total_cards_in_set
                    ),
                    theme::muted_style(),
                )),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(screen.collection_idx));

    let title = if ctx.syncing_collections_list {
        format!("Collections ({spin} Refreshing...)")
    } else {
        "Select Collection / Card Set".to_string()
    };

    let list = List::new(items)
        .block(theme::panel_block(title, true))
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    if ctx.collections.len() * 2 > cols[0].height.saturating_sub(2) as usize {
        let mut sb = ScrollbarState::new(ctx.collections.len()).position(screen.collection_idx);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            cols[0],
            &mut sb,
        );
    }

    // Right panel: Overview + Completion Gauge + Rarity BarChart
    let right_block = theme::panel_block("Collection Overview", false);
    let right_inner = right_block.inner(cols[1]);
    frame.render_widget(right_block, cols[1]);

    let Some(col) = ctx.collections.get(screen.collection_idx) else {
        return;
    };

    let cached = ctx.cached_card_counts.get(&col.id).copied().unwrap_or(0);
    let prog = ctx
        .collection_progress
        .iter()
        .find(|p| p.collection_id == col.id);
    let owned = prog.map(|p| p.unique_owned).unwrap_or(0);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(7),
            Constraint::Length(if cached > 0 { 2 } else { 0 }),
            Constraint::Length(if cached > 0 { 9 } else { 3 }),
        ])
        .split(right_inner);

    let mut lines = vec![
        Line::from(Span::styled(
            col.name.clone(),
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("Set Code: {}  •  Series: {}", col.id, col.series),
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(""),
        Line::from(Span::styled(col.description.clone(), theme::primary_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Cached Pool : ", theme::muted_style()),
            Span::styled(
                format!("{cached} cards (sync limit: {})", ctx.settings.cards_per_sync),
                theme::primary_style(),
            ),
        ]),
    ];
    if let Some(ref synced) = col.synced_at {
        let short = synced.get(..19).unwrap_or(synced).replace('T', " ");
        lines.push(Line::from(vec![
            Span::styled("Last Synced : ", theme::muted_style()),
            Span::styled(short, theme::primary_style()),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), sections[0]);

    if cached > 0 {
        let ratio = (owned as f64 / cached.max(1) as f64).clamp(0.0, 1.0);
        let gauge = LineGauge::default()
            .ratio(ratio)
            .label(format!(
                "Set Completion: {owned}/{cached} ({:.0}%)",
                ratio * 100.0
            ))
            .filled_style(Style::default().fg(theme::ACCENT_GREEN))
            .unfilled_style(Style::default().fg(theme::BORDER_DIM));
        frame.render_widget(gauge, sections[1]);

        let bars: Vec<Bar<'_>> = Rarity::all()
            .iter()
            .map(|&rarity| {
                let count = ctx.cached_rarity_counts.get(&rarity).copied().unwrap_or(0) as u64;
                let color = theme::rarity_color(rarity, ctx.tick);
                let short_lbl = match rarity {
                    Rarity::Common => "Com",
                    Rarity::Uncommon => "Unc",
                    Rarity::Rare => "Rare",
                    Rarity::HoloRare => "Holo",
                    Rarity::UltraRare => "Ult",
                    Rarity::SecretRare => "Sec",
                };
                Bar::default()
                    .value(count)
                    .label(Line::from(short_lbl))
                    .style(Style::default().fg(color))
                    .value_style(
                        Style::default()
                            .fg(theme::BG_DARK)
                            .bg(color)
                            .add_modifier(Modifier::BOLD),
                    )
            })
            .collect();

        let chart = BarChart::default()
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::TOP)
                    .border_style(Style::default().fg(theme::BORDER_DIM))
                    .title(Span::styled(" Pool Rarity Breakdown ", theme::muted_style())),
            )
            .data(BarGroup::default().bars(&bars))
            .bar_width(4)
            .bar_gap(2);
        frame.render_widget(chart, sections[2]);
    } else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "Press [Enter] or [s] to fetch card metadata & artwork from TCGdex!",
                Style::default().fg(theme::ACCENT_CYAN),
            ))
            .wrap(Wrap { trim: true }),
            sections[2],
        );
    }
}
