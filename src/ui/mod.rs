pub mod card_details;
pub mod collection_select;
pub mod collection_view;
pub mod main_menu;
pub mod pack_opening;
pub mod pack_select;
pub mod settings;
pub mod theme;
pub mod universe_select;
pub mod widgets;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};

use crate::app::{ActiveModal, App, Screen};
use crate::images::renderer::ImageRenderer;

pub fn render(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();

    // Fill dark background across the terminal viewport
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::BG_DARK)),
        area,
    );

    // Fixed 3-band layout (no banner layout shift when status toasts appear/disappear)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);

    match app.screen {
        Screen::MainMenu => main_menu::render(frame, &app.ctx, &app.main_menu, chunks[1]),
        Screen::UniverseSelect => {
            universe_select::render(frame, &app.ctx, &app.universe_select, chunks[1])
        }
        Screen::CollectionSelect => {
            collection_select::render(frame, &app.ctx, &app.collection_select, chunks[1])
        }
        Screen::PackSelect => pack_select::render(frame, &app.ctx, &app.pack_select, chunks[1]),
        Screen::PackOpening => {
            pack_opening::render(frame, &mut app.ctx, &app.pack_opening_screen, chunks[1])
        }
        Screen::MyCollection => {
            collection_view::render(frame, &mut app.ctx, &mut app.my_collection, chunks[1])
        }
        Screen::CardDetails => {
            card_details::render(frame, &mut app.ctx, &mut app.card_details, chunks[1])
        }
        Screen::Settings => settings::render(frame, &app.ctx, &app.settings_screen, chunks[1]),
    }

    render_footer(frame, app, chunks[2]);

    if let Some(ref banner) = app.banner {
        render_floating_toast(frame, banner.message.as_str(), banner.is_error, chunks[1]);
    }

    if let Some(ref modal) = app.modal {
        render_modal_overlay(frame, modal, app.active_controller().title(), area);
    }
}

fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let screen_name = app.active_controller().title();

    let universe_name = app
        .active_universe()
        .map(|u| format!("{} {}", u.badge, u.name))
        .unwrap_or_else(|| "Pokémon".to_string());

    let proto = ImageRenderer::protocol_display_name(app.renderer.active_protocol());
    let unique_owned = app.owned_cards.len();
    let total_copies: u32 = app.owned_cards.iter().map(|o| o.quantity).sum();

    let mut spans = vec![
        Span::styled(
            if area.width >= 90 {
                " ✦ TERMINAL TCG VAULT "
            } else {
                " ✦ TCG VAULT "
            },
            Style::default()
                .fg(theme::ACCENT_GOLD)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", theme::muted_style()),
        Span::styled(
            screen_name,
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if area.width >= 74 {
        spans.push(Span::styled(" │ ", theme::muted_style()));
        spans.push(Span::styled(universe_name, theme::primary_style()));
    }

    spans.push(Span::styled(" │ ", theme::muted_style()));
    spans.push(Span::styled(
        format!("Owned: {unique_owned} ({total_copies}x)"),
        Style::default().fg(theme::ACCENT_GREEN),
    ));

    if area.width >= 106 {
        spans.push(Span::styled(" │ ", theme::muted_style()));
        spans.push(Span::styled(format!("GFX: {proto}"), theme::muted_style()));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER_DIM))
        .style(Style::default().bg(theme::BG_PANEL));
    frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let compact = area.width < 105;
    let hints = app.active_controller().footer_hints(compact);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::BORDER_DIM))
        .style(Style::default().bg(theme::BG_PANEL));
    let p = Paragraph::new(Line::from(Span::styled(
        format!(" {hints}"),
        theme::muted_style(),
    )))
    .block(block);
    frame.render_widget(p, area);
}

fn render_floating_toast(frame: &mut Frame<'_>, message: &str, is_error: bool, body_area: Rect) {
    if body_area.width < 24 || body_area.height < 4 {
        return;
    }
    let max_w = (body_area.width.saturating_sub(4)).min(66);
    let text_len = message.chars().count() as u16 + 4;
    let width = text_len.clamp(22, max_w);
    let height = 3;
    let x = body_area.right().saturating_sub(width + 1);
    let y = body_area.top();
    let toast_area = Rect::new(x, y, width, height);

    let (fg, bg) = if is_error {
        (theme::ACCENT_RED, theme::BG_DARK)
    } else {
        (theme::ACCENT_CYAN, theme::BG_DARK)
    };

    frame.render_widget(Clear, toast_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(fg))
        .style(Style::default().bg(bg))
        .padding(Padding::horizontal(1));
    let p = Paragraph::new(Line::from(Span::styled(
        message,
        Style::default().fg(fg).add_modifier(Modifier::BOLD),
    )))
    .wrap(Wrap { trim: true })
    .block(block);
    frame.render_widget(p, toast_area);
}

fn render_modal_overlay(
    frame: &mut Frame<'_>,
    modal: &ActiveModal,
    screen_title: &str,
    area: Rect,
) {
    match modal {
        ActiveModal::Confirm { title, message, .. } => {
            let popup = centered_rect(62, 38, area).max_clamp(44, 10, area);
            frame.render_widget(Clear, popup);
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(message.as_str(), theme::primary_style())),
                Line::from(""),
                Line::from(vec![
                    theme::pill_badge("Enter / y : Confirm", theme::BG_DARK, theme::ACCENT_RED),
                    Span::raw("   "),
                    theme::pill_badge("Esc / n : Cancel", theme::TEXT_PRIMARY, theme::BG_SELECTED),
                ]),
            ];
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme::ACCENT_GOLD))
                .style(Style::default().bg(theme::BG_PANEL))
                .padding(Padding::horizontal(2))
                .title(Span::styled(
                    format!(" {title} "),
                    Style::default()
                        .fg(theme::ACCENT_GOLD)
                        .add_modifier(Modifier::BOLD),
                ));
            let p = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(block);
            frame.render_widget(p, popup);
        }
        ActiveModal::Help => {
            let popup = centered_rect(72, 68, area).max_clamp(52, 16, area);
            frame.render_widget(Clear, popup);

            let lines = vec![
                Line::from(Span::styled(
                    format!("Active Screen: {screen_title}"),
                    Style::default()
                        .fg(theme::ACCENT_CYAN)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled("Global Shortcuts", theme::title_style())),
                Line::from("  [?]             Toggle this Help modal"),
                Line::from("  [Esc / q]       Go back / Close modal"),
                Line::from("  [Ctrl+C]        Quit immediately"),
                Line::from(""),
                Line::from(Span::styled("Pack Opening", theme::title_style())),
                Line::from("  [Space / Enter] Flip current card / Advance slot"),
                Line::from("  [a]             Reveal all remaining cards & view summary"),
                Line::from("  [← / →]         Browse card slots"),
                Line::from("  [d] / [o] / [c] Details / Open another / My Collection"),
                Line::from(""),
                Line::from(Span::styled("My Collection", theme::title_style())),
                Line::from("  [/]             Live search by name, #, rarity, or tag"),
                Line::from("  [s]             Cycle sort (Newest, Rarity, Qty, Set #, A-Z)"),
                Line::from("  [v] / [Tab] / [f] Toggle List/Grid, Filter Set, Filter Rarity"),
                Line::from(""),
                Line::from(Span::styled(
                    "Press [Esc], [Enter], or [?] to close",
                    theme::muted_style(),
                )),
            ];

            let block = theme::colored_panel_block(" Keyboard Shortcuts & Guide ", theme::ACCENT_CYAN);
            let p = Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .block(block);
            frame.render_widget(p, popup);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

trait RectClampExt {
    fn max_clamp(self, min_w: u16, min_h: u16, bounds: Rect) -> Rect;
}

impl RectClampExt for Rect {
    fn max_clamp(self, min_w: u16, min_h: u16, bounds: Rect) -> Rect {
        let w = self.width.max(min_w).min(bounds.width);
        let h = self.height.max(min_h).min(bounds.height);
        let x = bounds.x + bounds.width.saturating_sub(w) / 2;
        let y = bounds.y + bounds.height.saturating_sub(h) / 2;
        Rect::new(x, y, w, h)
    }
}
