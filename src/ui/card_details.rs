use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme;
use crate::ui::widgets::render_card_artwork_frame;

pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let Some(item) = app.detail_cards.get(app.detail_idx).cloned() else {
        return;
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .margin(1)
        .split(area);

    if let Some(slot) = app.image_slots.get_mut("main") {
        render_card_artwork_frame(frame, cols[0], &item.card, slot, app.tick, true);
    }

    let card = &item.card;
    let r_color = theme::rarity_color(card.rarity, app.tick);
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {}", card.name),
                Style::default()
                    .fg(r_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   (#{} in {})", card.card_number, card.collection_id),
                theme::muted_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Rarity      : ", theme::muted_style()),
            Span::styled(
                format!("{} ({})", card.rarity.label(), card.raw_rarity),
                theme::rarity_style(card.rarity, app.tick),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Owned       : ", theme::muted_style()),
            Span::styled(
                format!("x{}", item.quantity),
                Style::default()
                    .fg(theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    if let Some(ref note) = item.slot_note {
        lines.push(Line::from(vec![
            Span::styled("  Pull Note   : ", theme::muted_style()),
            Span::styled(note.clone(), Style::default().fg(theme::ACCENT_GOLD)),
        ]));
    }
    if let Some(ref acq) = item.first_acquired_at {
        let short = acq.get(..19).unwrap_or(acq).replace('T', " ");
        lines.push(Line::from(vec![
            Span::styled("  Acquired    : ", theme::muted_style()),
            Span::styled(short, theme::primary_style()),
        ]));
    }

    if let Some(ref sub) = card.metadata.subtitle {
        lines.push(Line::from(vec![
            Span::styled("  Type/Stage  : ", theme::muted_style()),
            Span::styled(sub.clone(), theme::primary_style()),
        ]));
    }
    if !card.metadata.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Tags        : ", theme::muted_style()),
            Span::styled(card.metadata.tags.join(", "), Style::default().fg(theme::ACCENT_CYAN)),
        ]));
    }
    if let Some(ref creator) = card.metadata.creator {
        lines.push(Line::from(vec![
            Span::styled("  Creator     : ", theme::muted_style()),
            Span::styled(creator.clone(), theme::primary_style()),
        ]));
    }

    if !card.metadata.attributes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Attributes:",
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        )));
        for (k, v) in &card.metadata.attributes {
            lines.push(Line::from(vec![
                Span::styled(format!("    {:<14} : ", k), theme::muted_style()),
                Span::styled(v.clone(), theme::primary_style()),
            ]));
        }
    }

    if !card.metadata.highlights.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Abilities & Moves:",
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )));
        for h in &card.metadata.highlights {
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

    if let Some(ref local) = card.local_image_path {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Cached File: {local}"),
            theme::muted_style(),
        )));
    }

    let title = format!(
        " Card Inspector ({} / {}) ",
        app.detail_idx + 1,
        app.detail_cards.len()
    );
    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(r_color))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(title, theme::title_style())),
        );
    frame.render_widget(panel, cols[1]);
}

