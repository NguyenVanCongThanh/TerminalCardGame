use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, LineGauge, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState, Wrap,
};

use crate::app::{App, PackPhase};
use crate::domain::PulledCard;
use crate::ui::theme;
use crate::ui::widgets::{
    BoosterPackWidget, PackStripWidget, render_card_artwork_frame, render_card_back,
    render_sparkle_border_overlay,
};

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let Some(ref state) = app.pack_opening else {
        return;
    };

    match state.phase.clone() {
        PackPhase::Ripping { ticks, max_ticks } => {
            render_ripping_animation(
                frame,
                &state.result.collection_name,
                &state.result.pack_config.id,
                &state.result.pack_config.name,
                state.result.cards.len(),
                ticks,
                max_ticks,
                app.tick,
                area,
            );
        }
        PackPhase::Revealing {
            cursor,
            revealed,
            flash_ticks,
        } => {
            let cards = state.result.cards.clone();
            let col_name = state.result.collection_name.clone();
            let pack_name = state.result.pack_config.name.clone();
            render_reveal_view(
                frame,
                app,
                &col_name,
                &pack_name,
                &cards,
                &revealed,
                cursor,
                flash_ticks,
                area,
            );
        }
        PackPhase::Summary { cursor } => {
            let cards = state.result.cards.clone();
            let col_name = state.result.collection_name.clone();
            let pack_name = state.result.pack_config.name.clone();
            render_summary_view(frame, app, &col_name, &pack_name, &cards, cursor, area);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_ripping_animation(
    frame: &mut Frame<'_>,
    collection_name: &str,
    pack_id: &str,
    pack_name: &str,
    card_count: usize,
    ticks: u8,
    max_ticks: u8,
    global_tick: u64,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme::ACCENT_GOLD))
        .style(Style::default().bg(theme::BG_PANEL))
        .title(Span::styled(
            format!(" Tearing Open {collection_name} — {pack_name} "),
            theme::title_style(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(11),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner);

    let ratio = (ticks as f64 / max_ticks.max(1) as f64).clamp(0.0, 1.0);

    frame.render_widget(
        BoosterPackWidget {
            pack_id,
            pack_name,
            set_name: collection_name,
            card_count,
            tick: global_tick,
            rip_ratio: Some(ratio),
        },
        rows[1],
    );

    let gauge_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(rows[2])[1];

    let gauge = LineGauge::default()
        .ratio(ratio)
        .label(format!("Ripping Foil... {:.0}%", ratio * 100.0))
        .filled_style(Style::default().fg(theme::ACCENT_GOLD))
        .unfilled_style(Style::default().fg(theme::BORDER_DIM));
    frame.render_widget(gauge, gauge_area);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "Press [Space] or [Enter] to skip animation",
            theme::muted_style(),
        ))
        .alignment(Alignment::Center),
        rows[3],
    );
}

#[allow(clippy::too_many_arguments)]
fn render_reveal_view(
    frame: &mut Frame<'_>,
    app: &mut App,
    collection_name: &str,
    pack_name: &str,
    cards: &[PulledCard],
    revealed: &[bool],
    cursor: usize,
    flash_ticks: u8,
    area: Rect,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(12)])
        .margin(1)
        .split(area);

    let revealed_count = revealed.iter().filter(|&&r| r).count();
    let strip_title = format!(
        "{collection_name} • {pack_name} │ Revealed {revealed_count}/{}",
        cards.len()
    );
    let strip_border_color = if flash_ticks > 0 && flash_ticks.is_multiple_of(2) {
        theme::ACCENT_GOLD
    } else {
        theme::BORDER_DIM
    };
    let strip_block = theme::colored_panel_block(strip_title, strip_border_color);
    let strip_inner = strip_block.inner(rows[0]);
    frame.render_widget(strip_block, rows[0]);

    frame.render_widget(
        PackStripWidget {
            cards,
            revealed,
            cursor,
            tick: app.tick,
        },
        strip_inner,
    );

    // Main split: Left = Card Artwork / Card Back, Right = Metadata & Reveal Highlights
    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(rows[1]);

    let Some(pulled) = cards.get(cursor) else {
        return;
    };
    let is_revealed = revealed.get(cursor).copied().unwrap_or(false);

    if !is_revealed {
        render_card_back(frame, main_cols[0], app.tick, &pulled.slot_label);
        render_unrevealed_info(frame, pulled, cursor, cards.len(), main_cols[1]);
    } else {
        if let Some(slot) = app.image_slots.get_mut("main") {
            render_card_artwork_frame(
                frame,
                main_cols[0],
                &pulled.card,
                slot,
                app.tick,
                flash_ticks > 0 || pulled.card.rarity.is_hit(),
            );
            if flash_ticks > 0 {
                let flash_color = if pulled.card.rarity.is_hit() {
                    theme::rarity_color(pulled.card.rarity, app.tick)
                } else {
                    theme::ACCENT_GOLD
                };
                render_sparkle_border_overlay(
                    frame.buffer_mut(),
                    main_cols[0],
                    flash_color,
                    app.tick,
                );
            }
        }
        render_revealed_metadata(
            frame,
            pulled,
            cursor,
            cards.len(),
            app.tick,
            main_cols[1],
        );
    }
}

fn render_summary_view(
    frame: &mut Frame<'_>,
    app: &mut App,
    collection_name: &str,
    pack_name: &str,
    cards: &[PulledCard],
    cursor: usize,
    area: Rect,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .margin(1)
        .split(area);

    let hits = cards.iter().filter(|c| c.card.rarity.is_hit()).count();
    let new_cards = cards.iter().filter(|c| c.is_new).count();
    let best_rarity = cards
        .iter()
        .map(|c| c.card.rarity)
        .max()
        .unwrap_or(crate::domain::Rarity::Common);

    let summary_header = Line::from(vec![
        theme::pill_badge("PACK COMPLETE", theme::BG_DARK, theme::ACCENT_GREEN),
        Span::raw("  "),
        Span::styled(
            format!("{collection_name} • {pack_name}"),
            theme::primary_style(),
        ),
        Span::styled("  │  ", theme::muted_style()),
        Span::styled(
            format!("Hits: {hits}"),
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", theme::muted_style()),
        Span::styled(
            format!("New: {new_cards}"),
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", theme::muted_style()),
        Span::styled(
            format!("Best: {}", best_rarity.label()),
            theme::rarity_style(best_rarity, app.tick),
        ),
    ]);
    let header_block = theme::colored_panel_block("Pack Pull Summary", theme::ACCENT_GOLD);
    frame.render_widget(Paragraph::new(summary_header).block(header_block), rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(54), Constraint::Percentage(46)])
        .split(rows[1]);

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(3)])
        .split(cols[0]);

    let table_rows: Vec<Row<'_>> = cards
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let marker = if i == cursor { "▶" } else { " " };
            let status = if p.is_new {
                Span::styled(
                    "NEW!",
                    Style::default()
                        .fg(theme::ACCENT_GREEN)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(format!("x{}", p.new_quantity), theme::muted_style())
            };
            Row::new(vec![
                Cell::from(format!("{marker} {}", i + 1)).style(theme::muted_style()),
                Cell::from(p.card.rarity.short_badge())
                    .style(theme::rarity_style(p.card.rarity, app.tick)),
                Cell::from(p.card.name.clone()).style(theme::primary_style()),
                Cell::from(p.slot_label.clone()).style(theme::muted_style()),
                Cell::from(status),
            ])
        })
        .collect();

    let mut tbl_state = TableState::default();
    tbl_state.select(Some(cursor));

    let table = Table::new(
        table_rows,
        [
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Min(14),
            Constraint::Length(13),
            Constraint::Length(5),
        ],
    )
    .header(
        Row::new(vec!["#", "Tier", "Card Name", "Slot", "Vault"])
            .style(theme::title_style()),
    )
    .row_highlight_style(theme::selected_row_style())
    .block(theme::panel_block("Pulled Cards (All Added to Vault)", true));
    frame.render_stateful_widget(table, left_rows[0], &mut tbl_state);

    if cards.len() > left_rows[0].height.saturating_sub(3) as usize {
        let mut sb = ScrollbarState::new(cards.len()).position(cursor);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            left_rows[0],
            &mut sb,
        );
    }

    let action_bar = Paragraph::new(Line::from(vec![
        theme::pill_badge("Enter/d", theme::BG_DARK, theme::ACCENT_CYAN),
        Span::raw(" Inspect  "),
        theme::pill_badge("o", theme::BG_DARK, theme::ACCENT_GOLD),
        Span::raw(" Open Another  "),
        theme::pill_badge("c", theme::BG_DARK, theme::ACCENT_GREEN),
        Span::raw(" My Collection"),
    ]))
    .alignment(Alignment::Center)
    .block(theme::panel_block("Actions", false));
    frame.render_widget(action_bar, left_rows[1]);

    if let Some(pulled) = cards.get(cursor)
        && let Some(slot) = app.image_slots.get_mut("main")
    {
        let badge = if pulled.is_new {
            "★ NEW!".to_string()
        } else {
            format!("OWNED x{}", pulled.new_quantity)
        };
        crate::ui::widgets::render_card_artwork_frame_with_badge(
            frame,
            cols[1],
            &pulled.card,
            slot,
            app.tick,
            true,
            Some(&badge),
        );
    }
}

fn render_unrevealed_info(
    frame: &mut Frame<'_>,
    pulled: &PulledCard,
    cursor: usize,
    total: usize,
    area: Rect,
) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Card {} of {total}", cursor + 1),
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("Pack Slot: {}", pulled.slot_label),
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Artwork is pre-loaded in the background.",
            theme::muted_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "• [Space] / [Enter] : Flip this card!",
            theme::primary_style(),
        )),
        Line::from(Span::styled(
            "• [a]               : Reveal all & view summary",
            theme::primary_style(),
        )),
        Line::from(Span::styled(
            "• [←] / [→]         : Switch card slot",
            theme::primary_style(),
        )),
    ];

    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(theme::panel_block("Face-Down Card", false));
    frame.render_widget(panel, area);
}

fn render_revealed_metadata(
    frame: &mut Frame<'_>,
    pulled: &PulledCard,
    cursor: usize,
    total: usize,
    tick: u64,
    area: Rect,
) {
    let card = &pulled.card;
    let r_color = theme::rarity_color(card.rarity, tick);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", card.name),
                Style::default()
                    .fg(r_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("#{}", card.card_number), theme::muted_style()),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{} ({}) ", card.rarity.label(), card.raw_rarity),
                theme::rarity_style(card.rarity, tick),
            ),
            if pulled.is_new {
                theme::pill_badge("★ NEW!", theme::BG_DARK, theme::ACCENT_GREEN)
            } else {
                theme::pill_badge(
                    format!("OWNED x{}", pulled.new_quantity),
                    theme::ACCENT_CYAN,
                    theme::BG_SELECTED,
                )
            },
        ]),
        Line::from(Span::styled(
            format!("Slot: {}  •  Card {} / {total}", pulled.slot_label, cursor + 1),
            theme::muted_style(),
        )),
        Line::from(""),
    ];

    if let Some(ref sub) = card.metadata.subtitle {
        lines.push(Line::from(vec![
            Span::styled("Classification : ", theme::muted_style()),
            Span::styled(sub.clone(), theme::primary_style()),
        ]));
    }
    if let Some(ref creator) = card.metadata.creator {
        lines.push(Line::from(vec![
            Span::styled("Illustrator    : ", theme::muted_style()),
            Span::styled(creator.clone(), theme::primary_style()),
        ]));
    }
    for (k, v) in &card.metadata.attributes {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<14} : ", k), theme::muted_style()),
            Span::styled(v.clone(), theme::primary_style()),
        ]));
    }

    if !card.metadata.highlights.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Abilities & Moves:",
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )));
        for h in card.metadata.highlights.iter().take(4) {
            lines.push(Line::from(Span::styled(
                format!("  • {h}"),
                theme::primary_style(),
            )));
        }
    }

    if let Some(ref flavor) = card.metadata.flavor_text {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("\"{flavor}\""),
            theme::muted_style().add_modifier(Modifier::ITALIC),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press [Space] or [Enter] for next card  •  [a] Reveal All",
        Style::default().fg(theme::ACCENT_GOLD),
    )));

    let border_color = if card.rarity.is_hit() {
        r_color
    } else {
        theme::BORDER_DIM
    };
    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(theme::colored_panel_block("Card Pull Details", border_color));
    frame.render_widget(panel, area);
}
