use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::images::renderer::ImageRenderer;
use crate::ui::theme;

enum SettingValueKind {
    Selector(String),
    Toggle(bool),
    Action(String),
}

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
        .margin(1)
        .split(area);

    let rows = [
        (
            "Image Rendering Protocol",
            SettingValueKind::Selector(app.settings.protocol_mode.label().to_string()),
            "Use [←/→] to choose Auto-Detect or force Half-Blocks, Kitty, Sixel, or iTerm2.",
        ),
        (
            "Card Artwork Quality",
            SettingValueKind::Selector(app.settings.image_quality.label().to_string()),
            "Use [←/→] to toggle High (600x825 PNG) or Low / Fast (245x337 PNG).",
        ),
        (
            "Fast Pack Animations",
            SettingValueKind::Toggle(app.settings.fast_animations),
            "Use [←/→] or [Enter] to shorten the foil-rip opening animation.",
        ),
        (
            "Cards Synced Per Set",
            SettingValueKind::Selector(format!("{} cards", app.settings.cards_per_sync)),
            "Use [←/→] to choose sample size (16 / 28 / 40 / 60) when syncing a set.",
        ),
        (
            "Clear Local Image Cache",
            SettingValueKind::Action(format!(
                "{} files ({:.1} KB)",
                app.cache_stats.file_count,
                app.cache_stats.total_bytes as f64 / 1024.0
            )),
            "Press [Enter] to open confirmation dialog before deleting cached PNGs.",
        ),
    ];

    let items: Vec<ListItem<'_>> = rows
        .iter()
        .enumerate()
        .map(|(idx, (name, kind, desc))| {
            let prefix = if idx == app.settings_idx { "▶ " } else { "  " };
            let value_span = match kind {
                SettingValueKind::Selector(v) => Span::styled(
                    format!("< {v} >"),
                    Style::default().fg(theme::ACCENT_GOLD),
                ),
                SettingValueKind::Toggle(on) => {
                    if *on {
                        theme::pill_badge("ON", theme::BG_DARK, theme::ACCENT_GREEN)
                    } else {
                        theme::pill_badge("OFF", theme::TEXT_MUTED, theme::BG_SELECTED)
                    }
                }
                SettingValueKind::Action(info) => theme::pill_badge(
                    format!("Enter to Clear • {info}"),
                    theme::BG_DARK,
                    theme::ACCENT_RED,
                ),
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{prefix}{name}: "),
                        Style::default()
                            .fg(theme::TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    value_span,
                ]),
                Line::from(Span::styled(format!("  {desc}"), theme::muted_style())),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.settings_idx));

    let list = List::new(items)
        .block(theme::panel_block("Settings", true))
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    let detected = ImageRenderer::protocol_display_name(app.renderer.detected_protocol());
    let active = ImageRenderer::protocol_display_name(app.renderer.active_protocol());
    let (fw, fh) = app.renderer.font_size();

    let info_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Terminal Graphics Diagnostics",
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Auto-Detected : ", theme::muted_style()),
            Span::styled(detected, Style::default().fg(theme::ACCENT_GREEN)),
        ]),
        Line::from(vec![
            Span::styled("Active Mode   : ", theme::muted_style()),
            Span::styled(active, Style::default().fg(theme::ACCENT_GOLD)),
        ]),
        Line::from(vec![
            Span::styled("Cell FontSize : ", theme::muted_style()),
            Span::styled(format!("{fw}x{fh} px"), theme::primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("24-bit Color  : ", theme::muted_style()),
            Span::styled("██", Style::default().fg(theme::ACCENT_RED)),
            Span::styled("██", Style::default().fg(theme::ACCENT_GOLD)),
            Span::styled("██", Style::default().fg(theme::ACCENT_GREEN)),
            Span::styled("██", Style::default().fg(theme::ACCENT_CYAN)),
            Span::styled("██", Style::default().fg(theme::ACCENT_PURPLE)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "If your terminal shows garbled symbols with a native protocol, switch Image Rendering Protocol to 'Unicode Half-Blocks' — it works in every true-color terminal!",
            theme::muted_style(),
        )),
    ];

    let info_panel = Paragraph::new(info_lines)
        .wrap(Wrap { trim: true })
        .block(theme::panel_block("Graphics Diagnostics", false));
    frame.render_widget(info_panel, cols[1]);
}
