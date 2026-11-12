use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::images::renderer::ImageRenderer;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .margin(1)
        .split(area);

    let rows = [(
            "Image Rendering Protocol",
            app.settings.protocol_mode.label().to_string(),
            "Choose Auto-Detect or force Half-Blocks, Kitty, Sixel, or iTerm2.",
        ),
        (
            "Card Artwork Quality",
            app.settings.image_quality.label().to_string(),
            "High (600x825 PNG) gives crisp details; Low (245x337) downloads faster.",
        ),
        (
            "Pack Opening Animations",
            if app.settings.fast_animations {
                "Fast".to_string()
            } else {
                "Standard".to_string()
            },
            "Controls the foil-rip opening animation duration.",
        ),
        (
            "Cards Synced Per Set",
            format!("{} cards", app.settings.cards_per_sync),
            "Number of representative cards fetched when syncing a new set.",
        ),
        (
            "Clear Local Image Cache",
            format!(
                "{} files ({:.1} KB)",
                app.cache_stats.file_count,
                app.cache_stats.total_bytes as f64 / 1024.0
            ),
            "Press [Enter] to delete cached card PNGs (collection progress stays safe).",
        )];

    let items: Vec<ListItem<'_>> = rows
        .iter()
        .enumerate()
        .map(|(idx, (name, value, desc))| {
            let prefix = if idx == app.settings_idx { "▶ " } else { "  " };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{prefix}{name}: "),
                        Style::default()
                            .fg(theme::TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("< {value} >"),
                        Style::default().fg(theme::ACCENT_GOLD),
                    ),
                ]),
                Line::from(Span::styled(format!("    {desc}"), theme::muted_style())),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.settings_idx));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_FOCUS))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Settings ", theme::title_style())),
        )
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    let detected = ImageRenderer::protocol_display_name(app.renderer.detected_protocol());
    let active = ImageRenderer::protocol_display_name(app.renderer.active_protocol());
    let (fw, fh) = app.renderer.font_size();

    let info_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Terminal Graphics Diagnostics",
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Auto-Detected : ", theme::muted_style()),
            Span::styled(detected, Style::default().fg(theme::ACCENT_GREEN)),
        ]),
        Line::from(vec![
            Span::styled("  Active Mode   : ", theme::muted_style()),
            Span::styled(active, Style::default().fg(theme::ACCENT_GOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Cell FontSize : ", theme::muted_style()),
            Span::styled(format!("{fw}x{fh} px"), theme::primary_style()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  If your terminal shows garbled symbols with a native protocol, switch Image Rendering Protocol to 'Unicode Half-Blocks' — it works in every true-color terminal!",
            theme::muted_style(),
        )),
    ];

    let info_panel = Paragraph::new(info_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BORDER_DIM))
                .style(Style::default().bg(theme::BG_PANEL))
                .title(Span::styled(" Graphics Diagnostics ", theme::title_style())),
        );
    frame.render_widget(info_panel, cols[1]);
}
