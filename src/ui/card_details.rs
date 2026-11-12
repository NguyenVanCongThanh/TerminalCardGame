use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

use crate::app::{AppContext, CardDetailsScreen};
use crate::ui::theme;
use crate::ui::widgets::render_card_artwork_frame;

pub fn render(
    frame: &mut Frame<'_>,
    ctx: &mut AppContext,
    screen: &mut CardDetailsScreen,
    area: Rect,
) {
    let Some(item) = screen.cards.get(screen.idx).cloned() else {
        return;
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .margin(1)
        .split(area);

    let tick = ctx.tick;
    if let Some(slot) = ctx.image_slots.get_mut("main") {
        render_card_artwork_frame(frame, cols[0], &item.card, slot, tick, true);
    }

    let card = &item.card;
    let r_color = theme::rarity_color(card.rarity, tick);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                card.name.clone(),
                Style::default()
                    .fg(r_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   #{} in {}", card.card_number, card.collection_id),
                theme::muted_style(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            theme::pill_badge(
                format!("{} • {}", card.rarity.label(), card.raw_rarity),
                theme::BG_DARK,
                r_color,
            ),
            Span::raw("  "),
            theme::pill_badge(
                format!("OWNED x{}", item.quantity),
                theme::BG_DARK,
                theme::ACCENT_GREEN,
            ),
        ]),
    ];

    if !card.metadata.tags.is_empty() {
        let mut tag_spans = vec![Span::styled("Tags: ", theme::muted_style())];
        for (i, tag) in card.metadata.tags.iter().enumerate() {
            if i > 0 {
                tag_spans.push(Span::raw(" "));
            }
            tag_spans.push(theme::pill_badge(
                tag.clone(),
                theme::BG_DARK,
                theme::tag_color(tag),
            ));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(tag_spans));
    }

    lines.push(Line::from(""));
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
    if let Some(ref note) = item.slot_note {
        lines.push(Line::from(vec![
            Span::styled("Pull Origin    : ", theme::muted_style()),
            Span::styled(note.clone(), Style::default().fg(theme::ACCENT_GOLD)),
        ]));
    }
    if let Some(ref acq) = item.first_acquired_at {
        let short = acq.get(..19).unwrap_or(acq).replace('T', " ");
        lines.push(Line::from(vec![
            Span::styled("Acquired       : ", theme::muted_style()),
            Span::styled(short, theme::primary_style()),
        ]));
    }

    if !card.metadata.attributes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Attributes & Market",
            theme::title_style(),
        )));
        for (k, v) in &card.metadata.attributes {
            let val_style = if k.contains("Market") {
                Style::default()
                    .fg(theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme::primary_style()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<14} : ", k), theme::muted_style()),
                Span::styled(v.clone(), val_style),
            ]));
        }
    }

    if !card.metadata.highlights.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Abilities & Attacks",
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )));
        for h in &card.metadata.highlights {
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

    let title = format!(
        "Card Inspector ({} / {})",
        screen.idx + 1,
        screen.cards.len()
    );

    let total_lines = lines.len() as u16;
    let visible_h = cols[1].height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(visible_h);
    screen.scroll = screen.scroll.min(max_scroll);

    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .scroll((screen.scroll, 0))
        .block(theme::colored_panel_block(title, r_color));
    frame.render_widget(panel, cols[1]);

    if total_lines > visible_h {
        let mut sb = ScrollbarState::new(max_scroll as usize + 1).position(screen.scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            cols[1],
            &mut sb,
        );
    }
}
