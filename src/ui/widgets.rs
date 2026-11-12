use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::domain::Card;
use crate::images::renderer::CardImageSlot;
use crate::ui::theme;

/// Renders a stylized trading card back (used before flipping a card during pack opening).
pub fn render_card_back(frame: &mut Frame<'_>, area: Rect, tick: u64, slot_label: &str) {
    let sparkle = if (tick / 5).is_multiple_of(2) { "✦" } else { "✧" };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme::ACCENT_CYAN))
        .style(Style::default().bg(theme::BG_PANEL))
        .title(format!(" {sparkle} MYSTERY CARD {sparkle} "));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let pad_top = inner.height.saturating_sub(11) / 2;
    let mut lines = vec![Line::from(""); pad_top as usize];
    lines.push(Line::from(Span::styled(
        "╭─────────────────╮",
        Style::default().fg(theme::ACCENT_CYAN),
    )));
    lines.push(Line::from(Span::styled(
        "│  ◈  TCG VAULT ◈ │",
        Style::default()
            .fg(theme::ACCENT_GOLD)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "│    . * ✦ * .    │",
        Style::default().fg(theme::ACCENT_CYAN),
    )));
    lines.push(Line::from(Span::styled(
        "│   ╭─────────╮   │",
        Style::default().fg(theme::BORDER_FOCUS),
    )));
    lines.push(Line::from(Span::styled(
        "│   │  [ ? ]  │   │",
        Style::default()
            .fg(theme::ACCENT_GOLD)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "│   ╰─────────╯   │",
        Style::default().fg(theme::BORDER_FOCUS),
    )));
    lines.push(Line::from(Span::styled(
        "│    ` * ✧ * `    │",
        Style::default().fg(theme::ACCENT_CYAN),
    )));
    lines.push(Line::from(Span::styled(
        "╰─────────────────╯",
        Style::default().fg(theme::ACCENT_CYAN),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Slot: {slot_label}"),
        theme::muted_style(),
    )));
    lines.push(Line::from(Span::styled(
        "Press [Space] or [Enter] to Reveal",
        Style::default()
            .fg(theme::ACCENT_GOLD)
            .add_modifier(Modifier::BOLD),
    )));

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(p, inner);
}

/// Renders a card's artwork slot inside a rarity-styled frame, or a clean loading/offline fallback card face.
pub fn render_card_artwork_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    card: &Card,
    slot: &mut CardImageSlot,
    tick: u64,
    focused: bool,
) {
    let r_color = theme::rarity_color(card.rarity, tick);
    let border_type = if card.rarity.is_hit() || focused {
        BorderType::Thick
    } else {
        BorderType::Rounded
    };

    let title = format!(
        " {} #{} [{}] ",
        card.name,
        card.card_number,
        card.rarity.short_badge()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(r_color))
        .style(Style::default().bg(theme::BG_DARK))
        .title(Span::styled(
            title,
            Style::default().fg(r_color).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 4 {
        return;
    }

    if slot.has_ready_protocol() {
        slot.render_into(frame, inner);
        return;
    }

    // Fallback ASCII card face shown while loading or when offline without cached artwork
    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spin_ch = spinner[(tick as usize) % spinner.len()];

    let status_line = if slot.loading {
        Line::from(Span::styled(
            format!("{spin_ch} Loading artwork..."),
            Style::default().fg(theme::ACCENT_CYAN),
        ))
    } else if let Some(ref err) = slot.error {
        Line::from(Span::styled(
            format!("Artwork unavailable: {err}"),
            Style::default().fg(theme::ACCENT_RED),
        ))
    } else {
        Line::from(Span::styled(
            "No local artwork cached",
            theme::muted_style(),
        ))
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            card.name.clone(),
            Style::default()
                .fg(r_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            card.metadata
                .subtitle
                .clone()
                .unwrap_or_else(|| card.raw_rarity.clone()),
            theme::muted_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}  •  #{}", card.rarity.label(), card.card_number),
            Style::default().fg(r_color),
        )),
        Line::from(""),
        status_line,
    ];

    if let Some(hp) = card.metadata.attributes.get("HP") {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("HP: {hp}"),
            theme::primary_style(),
        )));
    }

    let fallback = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(fallback, inner);
}
