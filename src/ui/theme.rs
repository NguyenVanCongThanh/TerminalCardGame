use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding};

use crate::domain::Rarity;

pub const BG_DARK: Color = Color::Rgb(16, 19, 28);
pub const BG_PANEL: Color = Color::Rgb(24, 29, 43);
pub const BG_SELECTED: Color = Color::Rgb(40, 50, 76);
pub const BORDER_DIM: Color = Color::Rgb(68, 78, 108);
pub const BORDER_FOCUS: Color = Color::Rgb(110, 175, 255);
pub const TEXT_PRIMARY: Color = Color::Rgb(236, 240, 248);
pub const TEXT_MUTED: Color = Color::Rgb(145, 156, 182);
pub const ACCENT_GOLD: Color = Color::Rgb(255, 206, 75);
pub const ACCENT_CYAN: Color = Color::Rgb(86, 204, 242);
pub const ACCENT_GREEN: Color = Color::Rgb(92, 224, 148);
pub const ACCENT_RED: Color = Color::Rgb(245, 101, 101);
pub const ACCENT_PURPLE: Color = Color::Rgb(186, 128, 255);

pub fn rarity_color(rarity: Rarity, tick: u64) -> Color {
    match rarity {
        Rarity::Common => Color::Rgb(176, 184, 200),
        Rarity::Uncommon => Color::Rgb(92, 224, 148),
        Rarity::Rare => Color::Rgb(96, 178, 255),
        Rarity::HoloRare => Color::Rgb(204, 128, 255),
        Rarity::UltraRare => Color::Rgb(255, 204, 64),
        Rarity::SecretRare => {
            // Subtle shimmer cycle for Secret Rare frames
            match (tick / 4) % 3 {
                0 => Color::Rgb(255, 120, 180),
                1 => Color::Rgb(255, 215, 90),
                _ => Color::Rgb(110, 240, 255),
            }
        }
    }
}

pub fn rarity_style(rarity: Rarity, tick: u64) -> Style {
    let mut style = Style::default().fg(rarity_color(rarity, tick));
    if rarity.is_hit() {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

/// Maps TCG element/category tags to consistent semantic colors.
pub fn tag_color(tag: &str) -> Color {
    match tag.trim().to_ascii_lowercase().as_str() {
        "fire" => Color::Rgb(255, 118, 84),
        "water" => Color::Rgb(86, 182, 255),
        "grass" => Color::Rgb(98, 224, 134),
        "lightning" | "electric" => Color::Rgb(255, 214, 70),
        "psychic" => Color::Rgb(204, 128, 255),
        "fighting" => Color::Rgb(224, 140, 88),
        "darkness" | "dark" => Color::Rgb(145, 132, 188),
        "metal" | "steel" => Color::Rgb(176, 194, 212),
        "dragon" => Color::Rgb(218, 174, 72),
        "fairy" => Color::Rgb(255, 148, 204),
        "colorless" | "normal" => Color::Rgb(200, 208, 224),
        _ => ACCENT_CYAN,
    }
}

pub fn title_style() -> Style {
    Style::default()
        .fg(ACCENT_GOLD)
        .add_modifier(Modifier::BOLD)
}

pub fn muted_style() -> Style {
    Style::default().fg(TEXT_MUTED)
}

pub fn primary_style() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

pub fn selected_row_style() -> Style {
    Style::default()
        .bg(BG_SELECTED)
        .fg(TEXT_PRIMARY)
        .add_modifier(Modifier::BOLD)
}

/// Standardized rounded panel block with consistent horizontal padding and focus border.
pub fn panel_block<'a>(title: impl Into<String>, focused: bool) -> Block<'a> {
    let border_color = if focused { BORDER_FOCUS } else { BORDER_DIM };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG_PANEL))
        .padding(Padding::horizontal(1))
        .title(Span::styled(format!(" {} ", title.into()), title_style()))
}

/// Standardized rounded panel block with custom border color and horizontal padding.
pub fn colored_panel_block<'a>(title: impl Into<String>, border_color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG_PANEL))
        .padding(Padding::horizontal(1))
        .title(Span::styled(format!(" {} ", title.into()), title_style()))
}

/// Renders a padded pill badge span (` LABEL `) for statuses, counts, and tags.
pub fn pill_badge<'a>(label: impl Into<String>, fg: Color, bg: Color) -> Span<'a> {
    Span::styled(
        format!(" {} ", label.into()),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

/// Estimates the rendered height (in terminal rows) of `lines` when word-wrapped to `inner_width`.
pub fn wrapped_line_count(lines: &[Line<'_>], inner_width: u16) -> u16 {
    let max_w = inner_width as usize;
    if max_w == 0 {
        return 0;
    }

    let mut total_rows: usize = 0;
    for line in lines {
        let plain: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let full_w = Line::raw(&plain).width();
        if full_w <= max_w {
            total_rows += 1;
            continue;
        }

        let leading_ws = plain.chars().take_while(|c| c.is_whitespace()).count();
        let mut rows: usize = 1;
        let mut col = leading_ws.min(max_w);
        let mut first_word = true;

        for word in plain.split_whitespace() {
            let word_w = Line::raw(word).width();
            if word_w > max_w {
                if !first_word && col > 0 {
                    rows += 1;
                }
                let chunks = word_w.div_ceil(max_w);
                rows += chunks.saturating_sub(1);
                let rem = word_w % max_w;
                col = if rem == 0 { max_w } else { rem };
            } else if first_word {
                if col + word_w <= max_w {
                    col += word_w;
                } else {
                    rows += 1;
                    col = word_w;
                }
            } else if col + 1 + word_w <= max_w {
                col += 1 + word_w;
            } else {
                rows += 1;
                col = word_w;
            }
            first_word = false;
        }
        total_rows += rows;
    }

    total_rows.min(u16::MAX as usize) as u16
}

/// Wraps a plain text string into lines of at most `max_width` terminal columns (for `ListItem` descriptions).
pub fn wrap_plain_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;

    for word in text.split_whitespace() {
        let word_w = Line::raw(word).width();
        if current.is_empty() {
            if word_w <= max_width {
                current.push_str(word);
                current_w = word_w;
            } else {
                let clipped: String = word.chars().take(max_width).collect();
                lines.push(clipped);
            }
        } else if current_w + 1 + word_w <= max_width {
            current.push(' ');
            current.push_str(word);
            current_w += 1 + word_w;
        } else {
            lines.push(current);
            current = word.to_string();
            current_w = word_w;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
