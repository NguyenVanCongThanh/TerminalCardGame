use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap};

use crate::domain::{Card, PulledCard};
use crate::images::renderer::CardImageSlot;
use crate::ui::theme;

/// Custom Widget rendering a foil booster pack with serrated edges, tier-specific styling,
/// and an optional ripping progress ratio (`0.0..=1.0`) for the pack-opening tear animation.
pub struct BoosterPackWidget<'a> {
    pub pack_id: &'a str,
    pub pack_name: &'a str,
    pub set_name: &'a str,
    pub card_count: usize,
    pub tick: u64,
    pub rip_ratio: Option<f64>,
}

impl<'a> Widget for BoosterPackWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 9 {
            return;
        }

        let (accent_primary, accent_secondary, tier_label) = match self.pack_id {
            "collector-5" => (
                theme::ACCENT_PURPLE,
                theme::rarity_color(crate::domain::Rarity::SecretRare, self.tick),
                "★ COLLECTOR EDITION ★",
            ),
            "booster-10" => (
                theme::ACCENT_GOLD,
                theme::ACCENT_GREEN,
                "◆ STANDARD BOOSTER ◆",
            ),
            _ => (theme::ACCENT_CYAN, theme::ACCENT_GOLD, "● STARTER DEMO ●"),
        };

        let pack_w: u16 = 28.min(area.width.saturating_sub(2));
        let pack_h: u16 = 9.min(area.height);
        let start_x = area.x + area.width.saturating_sub(pack_w) / 2;
        let start_y = area.y + area.height.saturating_sub(pack_h) / 2;

        let ratio = self.rip_ratio.unwrap_or(0.0).clamp(0.0, 1.0);
        let tear_cols = if self.rip_ratio.is_some() {
            ((ratio * (pack_w as f64 - 4.0)).round() as u16).min(pack_w.saturating_sub(4))
        } else {
            0
        };
        let top_shift = if self.rip_ratio.is_some() {
            ((ratio * 5.0).round() as u16).min(4)
        } else {
            0
        };

        let sparkle_a = if (self.tick / 3).is_multiple_of(2) {
            "✦"
        } else {
            "✧"
        };
        let sparkle_b = if (self.tick / 3).is_multiple_of(2) {
            "✧"
        } else {
            "★"
        };

        let crimp = "▲▼".repeat((pack_w as usize / 2).max(4));
        let foil_bar = "▓".repeat(pack_w.saturating_sub(4) as usize);

        let rows: Vec<(u16, String, Style)> = vec![
            (
                top_shift,
                crimp.chars().take(pack_w as usize).collect(),
                Style::default().fg(accent_primary),
            ),
            (
                top_shift,
                format!("│ {foil_bar} │"),
                Style::default().fg(accent_secondary),
            ),
            (
                0,
                if tear_cols > 0 {
                    let torn = "─".repeat(tear_cols as usize);
                    let rem = "═".repeat(pack_w.saturating_sub(4 + tear_cols) as usize);
                    format!("✂ {torn}✦{rem}")
                } else {
                    format!("├─{}─┤", "─".repeat(pack_w.saturating_sub(4) as usize))
                },
                Style::default()
                    .fg(if tear_cols > 0 {
                        theme::ACCENT_GOLD
                    } else {
                        accent_primary
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            (
                0,
                format!(
                    "│ {:^width$} │",
                    self.set_name
                        .chars()
                        .take(pack_w.saturating_sub(4) as usize)
                        .collect::<String>(),
                    width = pack_w.saturating_sub(4) as usize
                ),
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            (
                0,
                format!(
                    "│ {:^width$} │",
                    format!("{sparkle_a} {} CARDS {sparkle_b}", self.card_count),
                    width = pack_w.saturating_sub(4) as usize
                ),
                Style::default()
                    .fg(accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            (
                0,
                format!(
                    "│ {:^width$} │",
                    tier_label,
                    width = pack_w.saturating_sub(4) as usize
                ),
                Style::default().fg(accent_secondary),
            ),
            (
                0,
                format!(
                    "│ {:^width$} │",
                    self.pack_name
                        .chars()
                        .take(pack_w.saturating_sub(4) as usize)
                        .collect::<String>(),
                    width = pack_w.saturating_sub(4) as usize
                ),
                theme::muted_style(),
            ),
            (
                0,
                format!("│ {foil_bar} │"),
                Style::default().fg(accent_secondary),
            ),
            (
                0,
                crimp.chars().take(pack_w as usize).collect(),
                Style::default().fg(accent_primary),
            ),
        ];

        for (row_idx, (shift, text, style)) in rows.into_iter().enumerate() {
            let y = start_y + row_idx as u16;
            if y >= area.bottom() {
                break;
            }
            let x = (start_x + shift).min(area.right().saturating_sub(1));
            let max_w = area.right().saturating_sub(x) as usize;
            let clipped: String = text.chars().take(max_w).collect();
            buf.set_string(x, y, clipped, style);
        }

        // Emit foil burst sparks around the seam while ripping
        if tear_cols > 0 {
            let seam_y = start_y + 2;
            let spark_x_left = start_x.saturating_sub(2);
            let spark_x_right = (start_x + pack_w + 1).min(area.right().saturating_sub(1));
            if seam_y > area.top() {
                buf.set_string(
                    spark_x_left,
                    seam_y.saturating_sub(1),
                    sparkle_a,
                    Style::default().fg(theme::ACCENT_GOLD),
                );
                buf.set_string(
                    spark_x_right,
                    seam_y.saturating_sub(1),
                    sparkle_b,
                    Style::default().fg(accent_secondary),
                );
            }
        }
    }
}

/// Custom 1-line horizontal scrollable strip of pack slots (`◀ [1 ●] ▶[2 ?] ... ▶`)
/// so 10-card boosters never wrap or clip unpredictably.
pub struct PackStripWidget<'a> {
    pub cards: &'a [PulledCard],
    pub revealed: &'a [bool],
    pub cursor: usize,
    pub tick: u64,
}

impl<'a> Widget for PackStripWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height == 0 || self.cards.is_empty() {
            return;
        }

        let total = self.cards.len();
        let pill_w: usize = 10; // e.g., " 10● SR* "
        let usable_w = area.width.saturating_sub(4) as usize;
        let max_visible = (usable_w / (pill_w + 1)).max(1).min(total);

        // Keep cursor centered in sliding window
        let half = max_visible / 2;
        let mut start = self.cursor.saturating_sub(half);
        if start + max_visible > total {
            start = total.saturating_sub(max_visible);
        }
        let end = (start + max_visible).min(total);

        let y = area.y;
        let mut x = area.x;

        if start > 0 {
            buf.set_string(x, y, "◀ ", Style::default().fg(theme::ACCENT_GOLD));
            x += 2;
        }

        for idx in start..end {
            if x + 6 >= area.right() {
                break;
            }
            let pulled = &self.cards[idx];
            let is_cur = idx == self.cursor;
            let is_rev = self.revealed.get(idx).copied().unwrap_or(false);

            let label = if is_rev {
                let new_mark = if pulled.is_new { "*" } else { "" };
                format!(
                    " {}{}{} ",
                    idx + 1,
                    pulled.card.rarity.short_badge(),
                    new_mark
                )
            } else {
                format!(" {}.? ", idx + 1)
            };

            let style = if is_cur {
                let bg = if is_rev {
                    theme::rarity_color(pulled.card.rarity, self.tick)
                } else {
                    theme::ACCENT_GOLD
                };
                Style::default()
                    .bg(bg)
                    .fg(theme::BG_DARK)
                    .add_modifier(Modifier::BOLD)
            } else if is_rev {
                theme::rarity_style(pulled.card.rarity, self.tick)
            } else {
                theme::muted_style()
            };

            let clipped: String = label
                .chars()
                .take(area.right().saturating_sub(x) as usize)
                .collect();
            let char_len = clipped.chars().count() as u16;
            buf.set_string(x, y, clipped, style);
            x += char_len + 1;
        }

        if end < total && area.right() >= 2 {
            buf.set_string(
                area.right().saturating_sub(2),
                y,
                " ▶",
                Style::default().fg(theme::ACCENT_GOLD),
            );
        }
    }
}

/// Renders animated border sparkles on the frame's outer perimeter only (never touching interior artwork pixels).
pub fn render_sparkle_border_overlay(
    buf: &mut Buffer,
    area: Rect,
    color: Color,
    tick: u64,
) {
    if area.width < 6 || area.height < 4 {
        return;
    }
    let ch = if tick.is_multiple_of(2) { "✦" } else { "✧" };
    let style = Style::default()
        .fg(color)
        .add_modifier(Modifier::BOLD);

    let corners = [
        (area.x, area.y),
        (area.right() - 1, area.y),
        (area.x, area.bottom() - 1),
        (area.right() - 1, area.bottom() - 1),
    ];
    for (cx, cy) in corners {
        buf.set_string(cx, cy, ch, style);
    }
}

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

/// Renders a card's artwork slot inside a rarity-styled frame, or a structured fallback card face.
pub fn render_card_artwork_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    card: &Card,
    slot: &mut CardImageSlot,
    tick: u64,
    focused: bool,
) {
    render_card_artwork_frame_with_badge(frame, area, card, slot, tick, focused, None);
}

/// Renders a card's artwork frame with an optional bottom-right border badge (e.g., `x3`, `NEW!`).
pub fn render_card_artwork_frame_with_badge(
    frame: &mut Frame<'_>,
    area: Rect,
    card: &Card,
    slot: &mut CardImageSlot,
    tick: u64,
    focused: bool,
    bottom_badge: Option<&str>,
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
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(r_color))
        .style(Style::default().bg(theme::BG_DARK))
        .title(Span::styled(
            title,
            Style::default().fg(r_color).add_modifier(Modifier::BOLD),
        ));

    if let Some(badge) = bottom_badge {
        block = block.title_bottom(
            Line::from(theme::pill_badge(
                badge,
                if focused {
                    theme::BG_DARK
                } else {
                    theme::TEXT_PRIMARY
                },
                if focused {
                    theme::ACCENT_GOLD
                } else {
                    theme::BG_SELECTED
                },
            ))
            .alignment(Alignment::Right),
        );
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if focused && card.rarity.is_hit() {
        render_sparkle_border_overlay(frame.buffer_mut(), area, r_color, tick);
    }

    if inner.width < 4 || inner.height < 4 {
        return;
    }

    if slot.has_ready_protocol() {
        slot.render_into(frame, inner);
        return;
    }

    // Structured TCG fallback face shown while loading or when offline without cached artwork
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
            "Offline card face (no cached PNG)",
            theme::muted_style(),
        ))
    };

    let hp_str = card
        .metadata
        .attributes
        .get("HP")
        .map(|hp| format!("  HP {hp}"))
        .unwrap_or_default();

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                card.name.clone(),
                Style::default()
                    .fg(r_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(hp_str, Style::default().fg(theme::ACCENT_GOLD)),
        ]),
        Line::from(Span::styled(
            card.metadata
                .subtitle
                .clone()
                .unwrap_or_else(|| card.raw_rarity.clone()),
            theme::muted_style(),
        )),
        Line::from(""),
        Line::from(theme::pill_badge(
            format!("{} • #{}", card.rarity.label(), card.card_number),
            theme::BG_DARK,
            r_color,
        )),
        Line::from(""),
        status_line,
    ];

    if let Some(first_highlight) = card.metadata.highlights.first() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            first_highlight.chars().take(inner.width.saturating_sub(4) as usize).collect::<String>(),
            theme::primary_style(),
        )));
    }

    let fallback = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(fallback, inner);
}
