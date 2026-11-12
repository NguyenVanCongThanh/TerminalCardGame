use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

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
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
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
            " ╭──────────────────────────────────────────╮",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(Span::styled(
            " │   ✦  TERMINAL TRADING CARD VAULT  ✦      │",
            theme::title_style(),
        )),
        Line::from(Span::styled(
            " │   Collect • Rip Packs • Inspect Artwork  │",
            theme::muted_style(),
        )),
        Line::from(Span::styled(
            " ╰──────────────────────────────────────────╯",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Active Universe: ", theme::muted_style()),
            Span::styled(
                app.active_universe()
                    .map(|u| format!("{} {}", u.badge, u.name))
                    .unwrap_or_else(|| "Pokémon TCG".to_string()),
                Style::default()
                    .fg(theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
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
                .style(Style::default().bg(theme::BG_PANEL)),
        );
    frame.render_widget(hero, rows[0]);

    let items: Vec<ListItem<'_>> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(idx, (title, desc))| {
            let marker = if idx == app.menu_idx { "▶ " } else { "  " };
            let title_style = if idx == app.menu_idx {
                Style::default()
                    .fg(theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme::primary_style()
            };
            ListItem::new(vec![
                Line::from(Span::styled(format!("{marker}{title}"), title_style)),
                Line::from(Span::styled(format!("    {desc}"), theme::muted_style())),
            ])
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.menu_idx));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_FOCUS))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Navigation ", theme::title_style())),
        )
        .highlight_style(Style::default().bg(theme::BG_SELECTED));

    frame.render_stateful_widget(list, rows[1], &mut list_state);
}

fn render_dashboard_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(11), Constraint::Min(7)])
        .split(area);

    let unique_owned = app.owned_cards.len();
    let total_copies: u32 = app.owned_cards.iter().map(|o| o.quantity).sum();
    let packs_opened = app.pack_history.len();
    let synced_sets = app
        .cached_card_counts
        .values()
        .filter(|&&c| c > 0)
        .count();
    let proto_name = ImageRenderer::protocol_display_name(app.renderer.active_protocol());
    let (fw, fh) = app.renderer.font_size();

    let stats_lines = vec![
        Line::from(vec![
            Span::styled("  Unique Cards Owned : ", theme::muted_style()),
            Span::styled(
                unique_owned.to_string(),
                Style::default()
                    .fg(theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Total Copies Owned : ", theme::muted_style()),
            Span::styled(total_copies.to_string(), theme::primary_style()),
        ]),
        Line::from(vec![
            Span::styled("  Synced Card Sets   : ", theme::muted_style()),
            Span::styled(
                format!("{synced_sets} / {}", app.collections.len()),
                Style::default().fg(theme::ACCENT_CYAN),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Cached Artwork     : ", theme::muted_style()),
            Span::styled(
                format!(
                    "{} files ({:.1} KB)",
                    app.cache_stats.file_count,
                    app.cache_stats.total_bytes as f64 / 1024.0
                ),
                theme::primary_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Graphics Backend   : ", theme::muted_style()),
            Span::styled(
                format!("{proto_name} ({fw}x{fh}px cell)"),
                Style::default().fg(theme::ACCENT_GREEN),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Tip: Select 'Open Packs' to sync a Pokémon set & rip your first pack!",
            theme::muted_style(),
        )),
    ];

    let stats_block = Paragraph::new(stats_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Collector Profile ", theme::title_style())),
        );
    frame.render_widget(stats_block, rows[0]);

    let mut hist_lines = Vec::new();
    if app.pack_history.is_empty() {
        hist_lines.push(Line::from(""));
        hist_lines.push(Line::from(Span::styled(
            "  No packs opened yet. Open a Demo Pack to start your collection!",
            theme::muted_style(),
        )));
    } else {
        for entry in app.pack_history.iter().take(8) {
            let short_ts = entry.opened_at.get(..16).unwrap_or(&entry.opened_at).replace('T', " ");
            hist_lines.push(Line::from(vec![
                Span::styled("  ✦ ", Style::default().fg(theme::ACCENT_GOLD)),
                Span::styled(entry.pack_name.clone(), theme::primary_style()),
                Span::styled(
                    format!(" [{}] ", entry.collection_id),
                    Style::default().fg(theme::ACCENT_CYAN),
                ),
                Span::styled(short_ts, theme::muted_style()),
            ]));
        }
    }

    let hist_title = format!(" Recent Pack History ({packs_opened}) ");
    let hist_block = Paragraph::new(hist_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(hist_title, theme::title_style())),
        );
    frame.render_widget(hist_block, rows[1]);
}

