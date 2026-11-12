use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph, Wrap};

use crate::app::{App, PackPhase};
use crate::domain::PulledCard;
use crate::ui::theme;
use crate::ui::widgets::{render_card_artwork_frame, render_card_back};

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let Some(ref state) = app.pack_opening else {
        return;
    };

    match state.phase.clone() {
        PackPhase::Ripping { ticks, max_ticks } => {
            render_ripping_animation(
                frame,
                &state.result.collection_name,
                &state.result.pack_config.name,
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
                false,
                area,
            );
        }
        PackPhase::Summary { cursor } => {
            let cards = state.result.cards.clone();
            let col_name = state.result.collection_name.clone();
            let pack_name = state.result.pack_config.name.clone();
            let all_revealed = vec![true; cards.len()];
            render_reveal_view(
                frame,
                app,
                &col_name,
                &pack_name,
                &cards,
                &all_revealed,
                cursor,
                0,
                true,
                area,
            );
        }
    }
}

fn render_ripping_animation(
    frame: &mut Frame<'_>,
    collection_name: &str,
    pack_name: &str,
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
            Constraint::Min(3),
            Constraint::Length(11),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(inner);

    let ratio = (ticks as f64 / max_ticks.max(1) as f64).clamp(0.0, 1.0);
    let tear_gap = ((ratio * 8.0).round() as usize).min(8);
    let left_cut = " ".repeat(tear_gap);
    let sparkles = if global_tick.is_multiple_of(2) {
        "✦  ✧   ★   ✧  ✦"
    } else {
        "✧  ★   ✦   ★  ✧"
    };

    let foil_lines = vec![
        Line::from(Span::styled(
            sparkles,
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("{left_cut}╭──────────────────────╮"),
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(Span::styled(
            format!("{left_cut}│ ▓▓▓ TEARING FOIL ▓▓▓ │"),
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "├──────────────────────┤",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(Span::styled(
            format!("│ {:^20} │", collection_name.chars().take(20).collect::<String>()),
            theme::primary_style(),
        )),
        Line::from(Span::styled(
            "│    ★ MYSTERY PACK ★  │",
            Style::default().fg(theme::ACCENT_GREEN),
        )),
        Line::from(Span::styled(
            "╰──────────────────────╯",
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press [Space] to skip animation",
            theme::muted_style(),
        )),
    ];

    frame.render_widget(
        Paragraph::new(foil_lines).alignment(Alignment::Center),
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

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .bg(theme::BG_DARK),
        )
        .ratio(ratio)
        .label(format!("{:.0}%", ratio * 100.0));
    frame.render_widget(gauge, gauge_area);
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
    summary_mode: bool,
    area: Rect,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(12)])
        .margin(1)
        .split(area);

    // Top card strip showing revealed vs unrevealed cards
    let strip_spans: Vec<Span<'_>> = cards
        .iter()
        .enumerate()
        .flat_map(|(i, pulled)| {
            let is_cur = i == cursor;
            let is_rev = revealed.get(i).copied().unwrap_or(false);
            let label = if is_rev {
                format!(
                    " {}. {} [{}] ",
                    i + 1,
                    pulled.card.name.chars().take(12).collect::<String>(),
                    pulled.card.rarity.short_badge()
                )
            } else {
                format!(" {}. [ ??? ] ", i + 1)
            };

            let style = if is_cur {
                Style::default()
                    .bg(if is_rev {
                        theme::rarity_color(pulled.card.rarity, app.tick)
                    } else {
                        theme::ACCENT_GOLD
                    })
                    .fg(theme::BG_DARK)
                    .add_modifier(Modifier::BOLD)
            } else if is_rev {
                theme::rarity_style(pulled.card.rarity, app.tick)
            } else {
                theme::muted_style()
            };

            vec![Span::styled(label, style), Span::raw(" ")]
        })
        .collect();

    let status_tag = if summary_mode {
        "PACK COMPLETE — ALL CARDS ADDED TO COLLECTION"
    } else {
        "SEQUENTIAL REVEAL"
    };

    let strip_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if flash_ticks > 0 && flash_ticks.is_multiple_of(2) {
            theme::ACCENT_GOLD
        } else {
            theme::BORDER_DIM
        }))
        .style(Style::default().bg(theme::BG_PANEL))
        .title(Span::styled(
            format!(" {collection_name} • {pack_name} │ {status_tag} "),
            theme::title_style(),
        ));

    frame.render_widget(
        Paragraph::new(Line::from(strip_spans))
            .wrap(Wrap { trim: false })
            .block(strip_block),
        rows[0],
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
                flash_ticks > 0,
            );
        }
        render_revealed_metadata(
            frame,
            pulled,
            cursor,
            cards.len(),
            summary_mode,
            app.tick,
            main_cols[1],
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
            format!("  Card {} of {total}", cursor + 1),
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  Pack Slot: {}", pulled.slot_label),
            Style::default().fg(theme::ACCENT_CYAN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  The card artwork is pre-loaded in the background.",
            theme::muted_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  • [Space] / [Enter] : Flip this card!",
            theme::primary_style(),
        )),
        Line::from(Span::styled(
            "  • [a]               : Reveal all remaining cards",
            theme::primary_style(),
        )),
        Line::from(Span::styled(
            "  • [←] / [→]         : Switch card slot",
            theme::primary_style(),
        )),
    ];

    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Face-Down Card ", theme::title_style())),
        );
    frame.render_widget(panel, area);
}

fn render_revealed_metadata(
    frame: &mut Frame<'_>,
    pulled: &PulledCard,
    cursor: usize,
    total: usize,
    summary_mode: bool,
    tick: u64,
    area: Rect,
) {
    let card = &pulled.card;
    let r_color = theme::rarity_color(card.rarity, tick);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("  {} ", card.name),
                Style::default()
                    .fg(r_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("#{}", card.card_number),
                theme::muted_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {} ({}) ", card.rarity.label(), card.raw_rarity),
                theme::rarity_style(card.rarity, tick),
            ),
            if pulled.is_new {
                Span::styled(
                    " ★ NEW! ",
                    Style::default()
                        .bg(theme::ACCENT_GREEN)
                        .fg(theme::BG_DARK)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!(" OWNED x{} ", pulled.new_quantity),
                    Style::default()
                        .bg(theme::BG_SELECTED)
                        .fg(theme::ACCENT_CYAN),
                )
            },
        ]),
        Line::from(Span::styled(
            format!("  Slot: {}  •  Card {} / {total}", pulled.slot_label, cursor + 1),
            theme::muted_style(),
        )),
        Line::from(""),
    ];

    if let Some(ref sub) = card.metadata.subtitle {
        lines.push(Line::from(vec![
            Span::styled("  Classification : ", theme::muted_style()),
            Span::styled(sub.clone(), theme::primary_style()),
        ]));
    }
    if let Some(ref creator) = card.metadata.creator {
        lines.push(Line::from(vec![
            Span::styled("  Illustrator    : ", theme::muted_style()),
            Span::styled(creator.clone(), theme::primary_style()),
        ]));
    }
    for (k, v) in &card.metadata.attributes {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<14} : ", k), theme::muted_style()),
            Span::styled(v.clone(), theme::primary_style()),
        ]));
    }

    if !card.metadata.highlights.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Abilities & Moves:",
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )));
        for h in card.metadata.highlights.iter().take(4) {
            lines.push(Line::from(Span::styled(
                format!("    • {h}"),
                theme::primary_style(),
            )));
        }
    }

    if let Some(ref flavor) = card.metadata.flavor_text {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  \"{flavor}\""),
            theme::muted_style(),
        )));
    }

    lines.push(Line::from(""));
    if summary_mode {
        lines.push(Line::from(Span::styled(
            "  [←/→] Browse Pulls  •  [d] Full Details  •  [o] Open Another  •  [c] My Collection",
            Style::default().fg(theme::ACCENT_GOLD),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Press [Space] or [Enter] for next card  •  [a] Reveal All",
            Style::default().fg(theme::ACCENT_GOLD),
        )));
    }

    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if card.rarity.is_hit() {
                    r_color
                } else {
                    theme::BORDER_DIM
                }))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Card Pull Details ", theme::title_style())),
        );
    frame.render_widget(panel, area);
}
