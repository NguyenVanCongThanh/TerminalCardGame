use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{AppContext, UniverseSelectScreen};
use crate::ui::theme;

pub fn render(
    frame: &mut Frame<'_>,
    ctx: &AppContext,
    screen: &UniverseSelectScreen,
    area: Rect,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .margin(1)
        .split(area);

    let items: Vec<ListItem<'_>> = ctx
        .universes
        .iter()
        .enumerate()
        .map(|(i, u)| {
            let status = if u.is_implemented {
                theme::pill_badge("ACTIVE", theme::BG_DARK, theme::ACCENT_GREEN)
            } else {
                theme::pill_badge("COMING SOON", theme::TEXT_MUTED, theme::BG_SELECTED)
            };
            let prefix = if i == screen.universe_idx { "▶ " } else { "  " };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{prefix}{} {} ", u.badge, u.name),
                        if u.is_implemented {
                            Style::default()
                                .fg(theme::TEXT_PRIMARY)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            theme::muted_style()
                        },
                    ),
                    status,
                ]),
                Line::from(Span::styled(
                    format!("  {}", u.subtitle),
                    theme::muted_style(),
                )),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(screen.universe_idx));

    let list = List::new(items)
        .block(theme::panel_block("Select Card Universe", true))
        .highlight_style(Style::default().bg(theme::BG_SELECTED));
    frame.render_stateful_widget(list, cols[0], &mut state);

    let detail_lines = if let Some(u) = ctx.universes.get(screen.universe_idx) {
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{}  {}", u.badge, u.name),
                Style::default()
                    .fg(theme::ACCENT_GOLD)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                u.subtitle.clone(),
                Style::default().fg(theme::ACCENT_CYAN),
            )),
            Line::from(""),
            Line::from(Span::styled(u.description.clone(), theme::primary_style())),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status : ", theme::muted_style()),
                if u.is_implemented {
                    theme::pill_badge("PLAYABLE NOW", theme::BG_DARK, theme::ACCENT_GREEN)
                } else {
                    theme::pill_badge("PLANNED EXPANSION", theme::TEXT_MUTED, theme::BG_SELECTED)
                },
            ]),
        ];

        if u.is_implemented {
            let owned_in_u = ctx
                .owned_cards
                .iter()
                .filter(|o| o.card.universe_id == u.id)
                .count();
            let packs_in_u = ctx
                .pack_history
                .iter()
                .filter(|p| p.universe_id == u.id)
                .count();
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Your Vault Stats:",
                theme::title_style(),
            )));
            lines.push(Line::from(vec![
                Span::styled("  Available Sets : ", theme::muted_style()),
                Span::styled(format!("{}", ctx.collections.len()), theme::primary_style()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Unique Owned   : ", theme::muted_style()),
                Span::styled(
                    format!("{owned_in_u} cards"),
                    Style::default().fg(theme::ACCENT_GOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Packs Opened   : ", theme::muted_style()),
                Span::styled(format!("{packs_in_u}"), theme::primary_style()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press [Enter] to browse card sets in this universe!",
                Style::default().fg(theme::ACCENT_GREEN),
            )));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Upcoming universe expansion — stay tuned for future card sets and custom packs!",
                theme::muted_style(),
            )));
        }
        lines
    } else {
        vec![Line::from("No universes available.")]
    };

    let detail = Paragraph::new(detail_lines)
        .wrap(Wrap { trim: true })
        .block(theme::panel_block("Universe Showcase", false));
    frame.render_widget(detail, cols[1]);
}
