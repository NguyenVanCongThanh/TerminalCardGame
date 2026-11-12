use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme;
use crate::ui::widgets::render_card_artwork_frame;

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(10)])
        .margin(1)
        .split(area);

    render_filter_and_progress_bar(frame, app, outer[0]);

    let filtered_empty = app.filtered_owned_cards().is_empty();
    if filtered_empty {
        let msg = if app.owned_cards.is_empty() {
            "Your collection is empty! Return to Main Menu -> Open Packs to collect cards."
        } else {
            "No cards match the active filters. Press [Tab] or [f] to reset filters."
        };
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(format!("  {msg}"), theme::muted_style())),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Owned Cards ", theme::title_style())),
        );
        frame.render_widget(empty, outer[1]);
        return;
    }

    if app.grid_mode {
        render_grid_view(frame, app, outer[1]);
    } else {
        render_list_and_preview(frame, app, outer[1]);
    }
}

fn render_filter_and_progress_bar(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let set_label = app
        .collection_filter
        .as_deref()
        .and_then(|id| app.collections.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "All Sets".to_string());

    let rarity_label = app
        .rarity_filter
        .map(|r| r.label().to_string())
        .unwrap_or_else(|| "All Rarities".to_string());

    let view_label = if app.grid_mode {
        "2x2 Grid Preview"
    } else {
        "List + Large Preview"
    };

    let total_unique = app.owned_cards.len();
    let total_copies: u32 = app.owned_cards.iter().map(|o| o.quantity).sum();

    let progress_summary: String = app
        .collection_progress
        .iter()
        .filter(|p| p.total_cached_cards > 0)
        .map(|p| {
            format!(
                "{}: {}/{} ({:.0}%)",
                p.collection_id,
                p.unique_owned,
                p.total_cached_cards,
                p.cached_completion_pct()
            )
        })
        .collect::<Vec<_>>()
        .join("  •  ");

    let lines = vec![
        Line::from(vec![
            Span::styled("  Set [Tab]: ", theme::muted_style()),
            Span::styled(set_label, Style::default().fg(theme::ACCENT_CYAN)),
            Span::styled("  │  Rarity [f]: ", theme::muted_style()),
            Span::styled(rarity_label, Style::default().fg(theme::ACCENT_GOLD)),
            Span::styled("  │  View [v]: ", theme::muted_style()),
            Span::styled(view_label, Style::default().fg(theme::ACCENT_GREEN)),
            Span::styled(
                format!("  │  Showing {} ({total_unique} unique, {total_copies} total)", app.filtered_owned_cards().len()),
                theme::muted_style(),
            ),
        ]),
        Line::from(Span::styled(
            if progress_summary.is_empty() {
                "  Collection Progress: No sets synced yet".to_string()
            } else {
                format!("  Set Progress: {progress_summary}")
            },
            theme::muted_style(),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_DIM))
        .style(Style::default().bg(theme::BG_PANEL));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_list_and_preview(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    let filtered = app.filtered_owned_cards();
    let items: Vec<ListItem<'_>> = filtered
        .iter()
        .enumerate()
        .map(|(idx, owned)| {
            let prefix = if idx == app.owned_cursor { "▶ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme::ACCENT_GOLD)),
                Span::styled(
                    format!("{:<6} ", owned.card.rarity.short_badge()),
                    theme::rarity_style(owned.card.rarity, app.tick),
                ),
                Span::styled(
                    format!("{:<22} ", owned.card.name.chars().take(22).collect::<String>()),
                    theme::primary_style(),
                ),
                Span::styled(
                    format!("[{} #{}] ", owned.card.collection_id, owned.card.card_number),
                    theme::muted_style(),
                ),
                Span::styled(
                    format!("x{}", owned.quantity),
                    Style::default()
                        .fg(theme::ACCENT_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.owned_cursor));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_FOCUS))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Owned Cards ", theme::title_style())),
        )
        .highlight_style(theme::selected_row_style());
    frame.render_stateful_widget(list, cols[0], &mut state);

    if let Some(selected_card) = filtered.get(app.owned_cursor).map(|o| o.card.clone())
        && let Some(slot) = app.image_slots.get_mut("preview")
    {
        render_card_artwork_frame(frame, cols[1], &selected_card, slot, app.tick, true);
    }
}

fn render_grid_view(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let filtered_cards: Vec<(crate::domain::Card, u32)> = app
        .filtered_owned_cards()
        .into_iter()
        .map(|o| (o.card.clone(), o.quantity))
        .collect();
    let total = filtered_cards.len();
    let page_start = (app.owned_cursor / 4) * 4;
    let page_num = (app.owned_cursor / 4) + 1;
    let total_pages = total.div_ceil(4);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(area);

    let grid_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(grid_rows[0]);
    let bot_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(grid_rows[1]);

    let cells = [top_cols[0], top_cols[1], bot_cols[0], bot_cols[1]];

    for (offset, cell_area) in cells.into_iter().enumerate() {
        let abs_idx = page_start + offset;
        if let Some((card, qty)) = filtered_cards.get(abs_idx) {
            let focused = abs_idx == app.owned_cursor;
            let slot_key = format!("grid-{offset}");
            if let Some(slot) = app.image_slots.get_mut(&slot_key) {
                render_card_artwork_frame(frame, cell_area, card, slot, app.tick, focused);
            }
            // Small quantity badge on bottom-right corner of the cell border
            if cell_area.width > 12 && cell_area.height > 2 {
                let badge = Paragraph::new(Span::styled(
                    format!(" x{qty} "),
                    Style::default()
                        .bg(if focused {
                            theme::ACCENT_GOLD
                        } else {
                            theme::BG_SELECTED
                        })
                        .fg(if focused {
                            theme::BG_DARK
                        } else {
                            theme::TEXT_PRIMARY
                        })
                        .add_modifier(Modifier::BOLD),
                ));
                let badge_area = Rect::new(
                    cell_area.x + cell_area.width.saturating_sub(7),
                    cell_area.y + cell_area.height.saturating_sub(1),
                    6,
                    1,
                );
                frame.render_widget(badge, badge_area);
            }
        } else {
            let empty_box = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL));
            frame.render_widget(empty_box, cell_area);
        }
    }

    let pager = Paragraph::new(Line::from(Span::styled(
        format!(
            "  Grid Page {page_num}/{total_pages}  •  Selected: #{}  •  Press [Enter] for full details",
            app.owned_cursor + 1
        ),
        theme::muted_style(),
    )))
    .wrap(Wrap { trim: true });
    frame.render_widget(pager, outer[1]);
}
