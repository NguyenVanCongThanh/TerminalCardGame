use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Cell, List, ListItem, ListState, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    TableState,
};

use crate::app::App;
use crate::ui::theme;
use crate::ui::widgets::BoosterPackWidget;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .margin(1)
        .split(area);

    let set_name = app
        .active_collection()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| app.selected_collection_id.clone());

    let items: Vec<ListItem<'_>> = app
        .pack_configs
        .iter()
        .enumerate()
        .map(|(i, cfg)| {
            let prefix = if i == app.pack_idx { "▶ " } else { "  " };
            let (tier_badge, tier_bg) = match cfg.id.as_str() {
                "collector-5" => ("COLLECTOR", theme::ACCENT_PURPLE),
                "booster-10" => ("STANDARD", theme::ACCENT_GREEN),
                _ => ("DEMO", theme::ACCENT_CYAN),
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{prefix}{} ", cfg.name),
                        Style::default()
                            .fg(theme::ACCENT_GOLD)
                            .add_modifier(Modifier::BOLD),
                    ),
                    theme::pill_badge(tier_badge, theme::BG_DARK, tier_bg),
                ]),
                Line::from(Span::styled(
                    format!("  {} cards • {}", cfg.card_count(), cfg.description),
                    theme::muted_style(),
                )),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.pack_idx));

    let list = List::new(items)
        .block(theme::panel_block(format!("Choose Pack ({set_name})"), true))
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    // Right: Booster Pack foil preview + Slot Odds Table
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(8)])
        .split(cols[1]);

    let selected_pack = app.pack_configs.get(app.pack_idx);
    let pack_block = theme::panel_block("Sealed Booster Preview", false);
    let pack_inner = pack_block.inner(right_rows[0]);
    frame.render_widget(pack_block, right_rows[0]);

    if let Some(cfg) = selected_pack {
        frame.render_widget(
            BoosterPackWidget {
                pack_id: &cfg.id,
                pack_name: &cfg.name,
                set_name: &set_name,
                card_count: cfg.card_count(),
                tick: app.tick,
                rip_ratio: None,
            },
            pack_inner,
        );
    }

    let mut table_rows = Vec::new();
    let total_slots = selected_pack.map(|c| c.slots.len()).unwrap_or(0);
    if let Some(cfg) = selected_pack {
        for (idx, slot) in cfg.slots.iter().enumerate() {
            let total: u32 = slot.rarity_weights.iter().map(|(_, w)| *w).sum();
            let odds: Vec<Span<'_>> = slot
                .rarity_weights
                .iter()
                .enumerate()
                .flat_map(|(w_idx, (r, w))| {
                    let pct = if total > 0 {
                        (*w as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    let mut spans = vec![Span::styled(
                        format!("{} {:.0}%", r.short_badge(), pct),
                        theme::rarity_style(*r, app.tick),
                    )];
                    if w_idx + 1 < slot.rarity_weights.len() {
                        spans.push(Span::styled(" │ ", theme::muted_style()));
                    }
                    spans
                })
                .collect();

            table_rows.push(Row::new(vec![
                Cell::from(format!("#{}", idx + 1)).style(theme::muted_style()),
                Cell::from(slot.label.clone()).style(theme::primary_style()),
                Cell::from(Line::from(odds)),
            ]));
        }
    }

    let header = Row::new(vec!["#", "Slot", "Rarity Odds"])
        .style(theme::title_style())
        .bottom_margin(0);

    let mut tbl_state = TableState::default();
    let table = Table::new(
        table_rows,
        [
            Constraint::Length(4),
            Constraint::Length(16),
            Constraint::Min(18),
        ],
    )
    .header(header)
    .block(theme::panel_block("Configurable Pack Rules", false));
    frame.render_stateful_widget(table, right_rows[1], &mut tbl_state);

    if total_slots > right_rows[1].height.saturating_sub(3) as usize {
        let mut sb = ScrollbarState::new(total_slots).position(0);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            right_rows[1],
            &mut sb,
        );
    }
}
