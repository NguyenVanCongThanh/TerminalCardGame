use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, LineGauge, List, ListItem, ListState, Padding, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use crate::app::App;
use crate::images::renderer::ImageRenderer;
use crate::ui::theme;

const MENU_ITEMS: [(&str, &str); 5] = [
    ("1. Open Packs", "Choose a collection and tear open trading card packs"),
    ("2. Browse Universes", "Switch card themes & explore extensible universes"),
    ("3. My Collection", "Inspect owned cards, artwork, duplicates & completion"),
    ("4. Settings", "Configure terminal graphics protocol, quality & cache"),
    ("5. Exit", "Save progress and exit to terminal"),
];

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .margin(1)
        .split(area);

    render_menu_panel(frame, app, cols[0]);
    render_dashboard_panel(frame, app, cols[1]);
}

fn render_menu_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(8)])
        .split(area);

    let logo = vec![
        Line::from(Span::styled(
            "╭──────────────────────────────────────────╮",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(Span::styled(
            "│   ✦  TERMINAL TRADING CARD VAULT  ✦      │",
            theme::title_style(),
        )),
        Line::from(Span::styled(
            "│   Collect • Rip Packs • Inspect Artwork  │",
            theme::muted_style(),
        )),
        Line::from(Span::styled(
            "╰──────────────────────────────────────────╯",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Active Universe: ", theme::muted_style()),
            theme::pill_badge(
                app.active_universe()
                    .map(|u| format!("{} {}", u.badge, u.name))
                    .unwrap_or_else(|| "Pokémon TCG".to_string()),
                theme::BG_DARK,
                theme::ACCENT_GREEN,
            ),
        ]),
    ];

    let hero = Paragraph::new(logo)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .padding(Padding::horizontal(1)),
        );
    frame.render_widget(hero, rows[0]);

    let items: Vec<ListItem<'_>> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(idx, (title, desc))| {
            let active_row = !app.menu_focus_history && idx == app.menu_idx;
            let marker = if active_row { "▶ " } else { "  " };
            let title_style = if active_row {
                Style::default()
                    .fg(theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme::primary_style()
            };
            ListItem::new(vec![
                Line::from(Span::styled(format!("{marker}{title}"), title_style)),
                Line::from(Span::styled(format!("  {desc}"), theme::muted_style())),
            ])
        })
        .collect();

    let mut list_state = ListState::default();
    if !app.menu_focus_history {
        list_state.select(Some(app.menu_idx));
    }

    let list = List::new(items)
        .block(theme::panel_block("Navigation", !app.menu_focus_history))
        .highlight_style(Style::default().bg(theme::BG_SELECTED));

    frame.render_stateful_widget(list, rows[1], &mut list_state);
}

fn render_dashboard_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(7)])
        .split(area);

    render_collector_profile(frame, app, rows[0]);
    render_pack_history(frame, app, rows[1]);
}

fn render_collector_profile(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = theme::panel_block("Collector Profile & Set Progress", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let unique_owned = app.owned_cards.len();
    let total_copies: u32 = app.owned_cards.iter().map(|o| o.quantity).sum();
    let synced_sets = app
        .cached_card_counts
        .values()
        .filter(|&&c| c > 0)
        .count();
    let proto_name = ImageRenderer::protocol_display_name(app.renderer.active_protocol());

    let synced_progress: Vec<_> = app
        .collection_progress
        .iter()
        .filter(|p| p.total_cached_cards > 0)
        .take(3)
        .collect();

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(1),
        ])
        .split(inner);

    let stats_lines = vec![
        Line::from(vec![
            Span::styled("Unique Cards: ", theme::muted_style()),
            Span::styled(
                format!("{unique_owned}"),
                Style::default()
                    .fg(theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  Total Copies: ", theme::muted_style()),
            Span::styled(format!("{total_copies}"), theme::primary_style()),
        ]),
        Line::from(vec![
            Span::styled("Synced Sets : ", theme::muted_style()),
            Span::styled(
                format!("{synced_sets}/{}", app.collections.len()),
                Style::default().fg(theme::ACCENT_CYAN),
            ),
            Span::styled("  │  Cache: ", theme::muted_style()),
            Span::styled(
                format!(
                    "{} imgs ({:.0} KB)",
                    app.cache_stats.file_count,
                    app.cache_stats.total_bytes as f64 / 1024.0
                ),
                theme::primary_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Graphics    : ", theme::muted_style()),
            Span::styled(proto_name, Style::default().fg(theme::ACCENT_GREEN)),
        ]),
    ];
    frame.render_widget(Paragraph::new(stats_lines).wrap(Wrap { trim: true }), sections[0]);

    if synced_progress.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "Tip: Select 'Open Packs' to sync a set & track completion bars!",
                theme::muted_style(),
            ))
            .wrap(Wrap { trim: true }),
            sections[1],
        );
    } else {
        let gauge_constraints: Vec<Constraint> =
            vec![Constraint::Length(1); synced_progress.len().min(sections[1].height as usize)];
        let gauge_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(gauge_constraints)
            .split(sections[1]);

        for (row_rect, prog) in gauge_rows.iter().zip(synced_progress.iter()) {
            let ratio = if prog.total_cached_cards == 0 {
                0.0
            } else {
                (prog.unique_owned as f64 / prog.total_cached_cards as f64).clamp(0.0, 1.0)
            };
            let label = format!(
                "{:<14} {}/{} ({:.0}%)",
                prog.collection_name.chars().take(14).collect::<String>(),
                prog.unique_owned,
                prog.total_cached_cards,
                prog.cached_completion_pct()
            );
            let gauge = LineGauge::default()
                .ratio(ratio)
                .label(label)
                .filled_style(Style::default().fg(theme::ACCENT_GREEN))
                .unfilled_style(Style::default().fg(theme::BORDER_DIM));
            frame.render_widget(gauge, *row_rect);
        }
    }
}

fn render_pack_history(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let packs_opened = app.pack_history.len();
    let title = if app.pack_history.is_empty() {
        "Recent Pack History (0)".to_string()
    } else if app.menu_focus_history {
        format!("Recent Pack History ({packs_opened}) — [Enter] Inspect")
    } else {
        format!("Recent Pack History ({packs_opened}) — [Tab] Focus")
    };

    if app.pack_history.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No packs opened yet. Open a Demo Pack to start your collection!",
                theme::muted_style(),
            )),
        ])
        .wrap(Wrap { trim: true })
        .block(theme::panel_block(title, false));
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem<'_>> = app
        .pack_history
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let short_ts = entry
                .opened_at
                .get(..16)
                .unwrap_or(&entry.opened_at)
                .replace('T', " ");
            let marker = if app.menu_focus_history && i == app.history_idx {
                "▶ "
            } else {
                "✦ "
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme::ACCENT_GOLD)),
                Span::styled(entry.pack_name.clone(), theme::primary_style()),
                Span::styled(
                    format!(" [{}] ", entry.collection_id),
                    Style::default().fg(theme::ACCENT_CYAN),
                ),
                Span::styled(short_ts, theme::muted_style()),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if app.menu_focus_history {
        state.select(Some(app.history_idx));
    }

    let list = List::new(items)
        .block(theme::panel_block(title, app.menu_focus_history))
        .highlight_style(theme::selected_row_style());
    frame.render_stateful_widget(list, area, &mut state);

    if app.pack_history.len() > area.height.saturating_sub(2) as usize {
        let mut sb_state = ScrollbarState::new(app.pack_history.len()).position(app.history_idx);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut sb_state,
        );
    }
}
