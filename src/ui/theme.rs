use ratatui::style::{Color, Modifier, Style};

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

