use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState, Tabs, Wrap,
};

use crate::app::{AppContext, MyCollectionScreen};
use crate::domain::Card;
use crate::ui::theme;
use crate::ui::widgets::render_card_artwork_frame_guarded;

pub fn render(
    frame: &mut Frame<'_>,
    ctx: &mut AppContext,
    screen: &mut MyCollectionScreen,
    area: Rect,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(10)])
        .margin(1)
        .split(area);

    render_filter_and_search_bar(frame, ctx, screen, outer[0]);

    let filtered_empty = screen.filtered_owned_cards(&ctx.owned_cards).is_empty();
    if filtered_empty {
        let msg = if ctx.owned_cards.is_empty() {
            "Your collection is empty! Return to Main Menu -> Open Packs to collect cards."
        } else {
            "No cards match the active filters/search. Press [Tab], [f], or [/] to adjust."
        };
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(msg, theme::muted_style())),
        ])
        .wrap(Wrap { trim: true })
        .block(theme::panel_block("Owned Cards", false));
        frame.render_widget(empty, outer[1]);
        return;
    }

    if screen.grid_mode {
        render_grid_view(frame, ctx, screen, outer[1]);
    } else {
        render_list_and_preview(frame, ctx, screen, outer[1]);
    }
}

fn render_filter_and_search_bar(
    frame: &mut Frame<'_>,
    ctx: &AppContext,
    screen: &MyCollectionScreen,
    area: Rect,
) {
    let block = theme::panel_block("Filter, Search & Vault Progress", screen.search_active);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Row 1: Set filter Tabs (keep labels concise when many tabs or narrow width)
    let avail_tab_w = rows[0].width as usize;
    let est_full_w: usize = 10
        + ctx
            .collections
            .iter()
            .map(|c| c.name.chars().count() + 3)
            .sum::<usize>();
    let compact_tabs = est_full_w > avail_tab_w;
    let mut tab_titles = vec![Line::from(" All Sets ")];
    let mut selected_tab = 0usize;
    for (idx, col) in ctx.collections.iter().enumerate() {
        let label = if compact_tabs {
            format!(" {} ", col.id)
        } else {
            format!(" {} ", col.name)
        };
        tab_titles.push(Line::from(label));
        if screen.collection_filter.as_deref() == Some(col.id.as_str()) {
            selected_tab = idx + 1;
        }
    }
    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(theme::muted_style())
        .highlight_style(
            Style::default()
                .fg(theme::BG_DARK)
                .bg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("│", theme::muted_style()));
    frame.render_widget(tabs, rows[0]);

    // Row 2: Search input + Rarity + Sort + View badges
    let rarity_label = screen
        .rarity_filter
        .map(|r| r.label().to_string())
        .unwrap_or_else(|| "All".to_string());
    let view_label = if screen.grid_mode { "Grid" } else { "List" };
    let max_query_chars: usize = if rows[1].width < 88 { 12 } else { 20 };
    let query_tail: String = {
        let count = screen.search_query.chars().count();
        if count > max_query_chars {
            let skip = count - max_query_chars.saturating_sub(1);
            format!("…{}", screen.search_query.chars().skip(skip).collect::<String>())
        } else {
            screen.search_query.clone()
        }
    };
    let search_display = if screen.search_active {
        format!(" / {query_tail}█ ")
    } else if screen.search_query.is_empty() {
        " [/] Search... ".to_string()
    } else {
        format!(" / {query_tail} [x] ")
    };
    let search_style = if screen.search_active {
        Style::default()
            .fg(theme::BG_DARK)
            .bg(theme::ACCENT_GOLD)
            .add_modifier(Modifier::BOLD)
    } else if !screen.search_query.is_empty() {
        Style::default()
            .fg(theme::ACCENT_GOLD)
            .bg(theme::BG_SELECTED)
    } else {
        Style::default().fg(theme::TEXT_MUTED).bg(theme::BG_DARK)
    };

    let controls_line = Line::from(vec![
        Span::styled(search_display, search_style),
        Span::raw("  "),
        Span::styled("Rarity [f]: ", theme::muted_style()),
        Span::styled(rarity_label, Style::default().fg(theme::ACCENT_GOLD)),
        Span::styled("  │  Sort [s]: ", theme::muted_style()),
        Span::styled(
            screen.sort_mode.label(),
            Style::default().fg(theme::ACCENT_CYAN),
        ),
        Span::styled("  │  View [v]: ", theme::muted_style()),
        Span::styled(view_label, Style::default().fg(theme::ACCENT_GREEN)),
    ]);
    frame.render_widget(Paragraph::new(controls_line), rows[1]);

    // Row 3: Count & active set progress
    let total_unique = ctx.owned_cards.len();
    let total_copies: u32 = ctx.owned_cards.iter().map(|o| o.quantity).sum();
    let shown = screen.filtered_owned_cards(&ctx.owned_cards).len();
    let progress_summary: String = ctx
        .collection_progress
        .iter()
        .filter(|p| p.total_cached_cards > 0)
        .map(|p| {
            format!(
                "{}:{}/{}({:.0}%)",
                p.collection_id,
                p.unique_owned,
                p.total_cached_cards,
                p.cached_completion_pct()
            )
        })
        .collect::<Vec<_>>()
        .join(" • ");

    let stats_text = if progress_summary.is_empty() {
        format!("Showing {shown} ({total_unique} unique, {total_copies} total)")
    } else {
        format!("Showing {shown} ({total_unique} unique, {total_copies} total)  │  {progress_summary}")
    };
    frame.render_widget(
        Paragraph::new(Span::styled(stats_text, theme::muted_style())),
        rows[2],
    );
}

fn render_list_and_preview(
    frame: &mut Frame<'_>,
    ctx: &mut AppContext,
    screen: &MyCollectionScreen,
    area: Rect,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(54), Constraint::Percentage(46)])
        .split(area);

    let tick = ctx.tick;
    let suppress = ctx.should_suppress_artwork();
    let filtered = screen.filtered_owned_cards(&ctx.owned_cards);
    let total = filtered.len();
    let rows: Vec<Row<'_>> = filtered
        .iter()
        .enumerate()
        .map(|(idx, owned)| {
            let prefix = if idx == screen.owned_cursor { "▶" } else { " " };
            let market = owned
                .card
                .metadata
                .attributes
                .get("Market Trend")
                .or_else(|| owned.card.metadata.attributes.get("Market Price"))
                .cloned()
                .unwrap_or_else(|| "-".to_string());

            Row::new(vec![
                Cell::from(prefix).style(Style::default().fg(theme::ACCENT_GOLD)),
                Cell::from(owned.card.rarity.short_badge())
                    .style(theme::rarity_style(owned.card.rarity, tick)),
                Cell::from(owned.card.name.clone()).style(theme::primary_style()),
                Cell::from(format!("{} #{}", owned.card.collection_id, owned.card.card_number))
                    .style(theme::muted_style()),
                Cell::from(format!("x{}", owned.quantity)).style(
                    Style::default()
                        .fg(theme::ACCENT_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(market).style(theme::muted_style()),
            ])
        })
        .collect();

    let mut state = TableState::default();
    state.select(Some(screen.owned_cursor));

    let set_col_w = if cols[0].width < 46 { 9 } else { 11 };
    let val_col_w = if cols[0].width < 46 { 6 } else { 8 };
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(set_col_w),
            Constraint::Length(4),
            Constraint::Length(val_col_w),
        ],
    )
    .header(Row::new(vec!["", "Tier", "Card Name", "Set #", "Qty", "Value"]).style(theme::title_style()))
    .row_highlight_style(theme::selected_row_style())
    .block(theme::panel_block("Owned Cards", !screen.search_active));
    frame.render_stateful_widget(table, cols[0], &mut state);

    if total > cols[0].height.saturating_sub(3) as usize {
        let mut sb = ScrollbarState::new(total).position(screen.owned_cursor);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            cols[0],
            &mut sb,
        );
    }

    let selected_card = filtered.get(screen.owned_cursor).map(|o| o.card.clone());
    if let Some(selected_card) = selected_card
        && let Some(slot) = ctx.image_slots.get_mut("preview")
    {
        render_card_artwork_frame_guarded(
            frame,
            cols[1],
            &selected_card,
            slot,
            tick,
            true,
            None,
            suppress,
        );
    }
}

fn render_grid_view(
    frame: &mut Frame<'_>,
    ctx: &mut AppContext,
    screen: &mut MyCollectionScreen,
    area: Rect,
) {
    // Responsive grid dimensions based on terminal size
    let (grid_cols, grid_rows) = if area.width >= 156 && area.height >= 24 {
        (3usize, 2usize)
    } else if area.height < 18 {
        (2usize, 1usize)
    } else {
        (2usize, 2usize)
    };
    let page_size = grid_cols * grid_rows;
    screen.sync_grid_page_size(page_size, ctx);

    let tick = ctx.tick;
    let suppress = ctx.should_suppress_artwork();
    let page_start = (screen.owned_cursor / page_size) * page_size;
    let page_num = (screen.owned_cursor / page_size) + 1;

    let (total, page_cards): (usize, Vec<(Card, u32)>) = {
        let filtered = screen.filtered_owned_cards(&ctx.owned_cards);
        let total = filtered.len();
        let slice = filtered
            .into_iter()
            .skip(page_start)
            .take(page_size)
            .map(|o| (o.card.clone(), o.quantity))
            .collect();
        (total, slice)
    };
    let total_pages = total.div_ceil(page_size);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(area);

    let row_constraints = vec![Constraint::Percentage((100 / grid_rows) as u16); grid_rows];
    let col_constraints = vec![Constraint::Percentage((100 / grid_cols) as u16); grid_cols];

    let row_rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(outer[0]);

    let mut cells = Vec::with_capacity(page_size);
    for r in row_rects.iter() {
        let col_rects = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints.clone())
            .split(*r);
        cells.extend(col_rects.iter().copied());
    }

    for (offset, cell_area) in cells.into_iter().enumerate() {
        let abs_idx = page_start + offset;
        if let Some((card, qty)) = page_cards.get(offset) {
            let focused = abs_idx == screen.owned_cursor;
            let slot_key = format!("grid-{offset}");
            let badge = format!("x{qty}");
            if let Some(slot) = ctx.image_slots.get_mut(&slot_key) {
                render_card_artwork_frame_guarded(
                    frame,
                    cell_area,
                    card,
                    slot,
                    tick,
                    focused,
                    Some(&badge),
                    suppress,
                );
            }
        } else {
            let empty_box = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL));
            frame.render_widget(empty_box, cell_area);
        }
    }

    let pager = Paragraph::new(Line::from(Span::styled(
        format!(
            " Grid Page {page_num}/{total_pages} ({grid_cols}×{grid_rows})  •  Card {}/{total}  •  [Enter] Details",
            screen.owned_cursor + 1
        ),
        theme::muted_style(),
    )))
    .wrap(Wrap { trim: true });
    frame.render_widget(pager, outer[1]);
}
