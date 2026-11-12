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
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Screen};
use crate::images::renderer::ImageRenderer;

pub fn render(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();

    // Fill dark background across the terminal viewport
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::BG_DARK)),
        area,
    );

    let has_banner = app.banner.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(if has_banner { 1 } else { 0 }),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);

    match app.screen {
        Screen::MainMenu => main_menu::render(frame, app, chunks[1]),
        Screen::UniverseSelect => universe_select::render(frame, app, chunks[1]),
        Screen::CollectionSelect => collection_select::render(frame, app, chunks[1]),
        Screen::PackSelect => pack_select::render(frame, app, chunks[1]),
        Screen::PackOpening => pack_opening::render(frame, app, chunks[1]),
        Screen::MyCollection => collection_view::render(frame, app, chunks[1]),
        Screen::CardDetails => card_details::render(frame, app, chunks[1]),
        Screen::Settings => settings::render(frame, app, chunks[1]),
    }

    if let Some(ref banner) = app.banner {
        let style = if banner.is_error {
            Style::default()
                .bg(theme::ACCENT_RED)
                .fg(theme::BG_DARK)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(theme::ACCENT_CYAN)
                .fg(theme::BG_DARK)
                .add_modifier(Modifier::BOLD)
        };
        let p = Paragraph::new(Line::from(format!(" {} ", banner.message))).style(style);
        frame.render_widget(p, chunks[2]);
    }

    render_footer(frame, app, chunks[3]);
}

fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let screen_name = match app.screen {
        Screen::MainMenu => "Main Menu",
        Screen::UniverseSelect => "Universes",
        Screen::CollectionSelect => "Collections",
        Screen::PackSelect => "Pack Selection",
        Screen::PackOpening => "Pack Opening",
        Screen::MyCollection => "My Collection",
        Screen::CardDetails => "Card Details",
        Screen::Settings => "Settings",
    };

    let universe_name = app
        .active_universe()
        .map(|u| format!("{} {}", u.badge, u.name))
        .unwrap_or_else(|| "Pokémon".to_string());

    let proto = ImageRenderer::protocol_display_name(app.renderer.active_protocol());
    let unique_owned = app.owned_cards.len();
    let total_copies: u32 = app.owned_cards.iter().map(|o| o.quantity).sum();

    let header_line = Line::from(vec![
        Span::styled(
            " ✦ TERMINAL TCG VAULT ",
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
        Span::styled(" │ ", theme::muted_style()),
        Span::styled(universe_name, theme::primary_style()),
        Span::styled(" │ ", theme::muted_style()),
        Span::styled(
            format!("Owned: {unique_owned} ({total_copies}x)"),
            Style::default().fg(theme::ACCENT_GREEN),
        ),
        Span::styled(" │ ", theme::muted_style()),
        Span::styled(format!("GFX: {proto}"), theme::muted_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER_DIM))
        .style(Style::default().bg(theme::BG_PANEL));
    frame.render_widget(Paragraph::new(header_line).block(block), area);
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let hints = match app.screen {
        Screen::MainMenu => "[↑/↓] Navigate   [Enter] Select   [1-5] Quick Jump   [q] Quit",
        Screen::UniverseSelect => "[↑/↓] Select Universe   [Enter] Open   [Esc] Back",
        Screen::CollectionSelect => "[↑/↓] Select Set   [Enter] Open Packs   [s] Sync Cards   [r] Refresh Sets   [Esc] Back",
        Screen::PackSelect => "[↑/↓] Choose Pack   [Enter/Space] Tear Open Pack!   [s] Re-sync Set   [Esc] Back",
        Screen::PackOpening => "[Space/Enter] Flip / Next   [←/→] Browse   [a] Reveal All   [d] Details   [o] Open Another   [c] Collection   [Esc] Back",
        Screen::MyCollection => "[↑/↓/←/→] Browse   [Enter] Card Details   [v] List/Grid   [Tab] Set Filter   [f] Rarity Filter   [Esc] Back",
        Screen::CardDetails => "[←/→] Prev / Next Card   [Esc/Backspace] Back",
        Screen::Settings => "[↑/↓] Select Option   [←/→/Enter] Change / Execute   [Esc] Back",
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::BORDER_DIM))
        .style(Style::default().bg(theme::BG_PANEL));
    let p = Paragraph::new(Line::from(Span::styled(format!(" {hints}"), theme::muted_style())))
        .block(block);
    frame.render_widget(p, area);
}

