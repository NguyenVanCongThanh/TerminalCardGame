use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
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
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{prefix}{}", cfg.name),
                    Style::default()
                        .fg(theme::ACCENT_GOLD)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("    {} cards • {}", cfg.card_count(), cfg.description),
                    theme::muted_style(),
                )),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.pack_idx));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_FOCUS))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(
                    format!(" Choose Pack ({set_name}) "),
                    theme::title_style(),
                )),
        )
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    // Right: Booster Pack foil preview + Slot Odds Table
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(8)])
        .split(cols[1]);

    let sparkle = if (app.tick / 6).is_multiple_of(2) { "✦" } else { "✧" };
    let pack_name = app
        .pack_configs
        .get(app.pack_idx)
        .map(|p| p.name.as_str())
        .unwrap_or("Booster Pack");

    let foil_art = vec![
        Line::from(Span::styled(
            "┌──────────────────────────┐",
            Style::default().fg(theme::ACCENT_GOLD),
        )),
        Line::from(Span::styled(
            "│ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(Span::styled(
            format!("│  {sparkle}  {:<18} │", set_name.chars().take(18).collect::<String>()),
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("│     {:<19} │", pack_name.chars().take(19).collect::<String>()),
            theme::primary_style(),
        )),
        Line::from(Span::styled(
            "│      ★ SEALED FOIL ★     │",
            Style::default().fg(theme::ACCENT_GREEN),
        )),
        Line::from(Span::styled(
            "│ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(Span::styled(
            "└──────────────────────────┘",
            Style::default().fg(theme::ACCENT_GOLD),
        )),
        Line::from(Span::styled(
            "Press [Enter] or [Space] to Rip Open!",
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let pack_box = Paragraph::new(foil_art)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Sealed Booster ", theme::title_style())),
        );
    frame.render_widget(pack_box, right_rows[0]);

    let mut slot_lines = Vec::new();
    if let Some(cfg) = app.pack_configs.get(app.pack_idx) {
        for slot in &cfg.slots {
            let total: u32 = slot.rarity_weights.iter().map(|(_, w)| *w).sum();
            let odds: Vec<Span<'_>> = slot
                .rarity_weights
                .iter()
                .enumerate()
                .flat_map(|(idx, (r, w))| {
                    let pct = if total > 0 {
                        (*w as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    let mut spans = vec![Span::styled(
                        format!("{} {:.0}%", r.short_badge(), pct),
                        theme::rarity_style(*r, app.tick),
                    )];
                    if idx + 1 < slot.rarity_weights.len() {
                        spans.push(Span::styled(" | ", theme::muted_style()));
                    }
                    spans
                })
                .collect();

            let mut line_spans = vec![Span::styled(
                format!("  {:<13} : ", slot.label),
                theme::muted_style(),
            )];
            line_spans.extend(odds);
            slot_lines.push(Line::from(line_spans));
        }
    }

    let odds_box = Paragraph::new(slot_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Configurable Pack Rules ", theme::title_style())),
        );
    frame.render_widget(odds_box, right_rows[1]);
}
