use crossterm::event::{KeyCode, KeyEvent};

use crate::app::context::AppContext;
use crate::app::screens::card_details::InspectableCard;
use crate::app::screens::{Screen, ScreenAction, ScreenController};
use crate::domain::{Card, PackOpeningResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackPhase {
    Ripping {
        ticks: u8,
        max_ticks: u8,
    },
    Revealing {
        cursor: usize,
        revealed: Vec<bool>,
        flash_ticks: u8,
    },
    Summary {
        cursor: usize,
    },
}

pub struct PackOpeningState {
    pub result: PackOpeningResult,
    pub phase: PackPhase,
}

#[derive(Default)]
pub struct PackOpeningScreen {
    pub state: Option<PackOpeningState>,
    pub detail_scroll: u16,
}

impl PackOpeningScreen {
    pub fn start(&mut self, result: PackOpeningResult, fast_animations: bool) {
        let max_ticks = if fast_animations { 4 } else { 10 };
        self.detail_scroll = 0;
        self.state = Some(PackOpeningState {
            result,
            phase: PackPhase::Ripping {
                ticks: 0,
                max_ticks,
            },
        });
    }

    pub fn bind_active_card(&self, ctx: &mut AppContext) {
        if let Some(ref state) = self.state
            && let PackPhase::Summary { cursor } | PackPhase::Revealing { cursor, .. } =
                state.phase
            && let Some(pulled) = state.result.cards.get(cursor)
        {
            ctx.request_card_into_slot("main", &pulled.card);
        }
    }
}

impl ScreenController for PackOpeningScreen {
    fn screen(&self) -> Screen {
        Screen::PackOpening
    }

    fn title(&self) -> &'static str {
        "Pack Opening"
    }

    fn footer_hints(&self, compact: bool) -> &'static str {
        match self.state.as_ref().map(|s| &s.phase) {
            Some(PackPhase::Ripping { .. }) => {
                if compact {
                    "[Space/Enter] Skip  [?] Help  [Esc/q] Back"
                } else {
                    "[Space/Enter] Skip Animation  [?] Help  [Esc/q] Back"
                }
            }
            Some(PackPhase::Revealing { .. }) => {
                if compact {
                    "[Space] Flip/Next  [←/→] Slot  [a] All  [Esc/q] Back"
                } else {
                    "[Space/Enter] Flip/Next  [←/→] Browse  [↑/↓] Scroll Info  [a] Reveal All  [Esc/q] Back"
                }
            }
            Some(PackPhase::Summary { .. }) | None => {
                if compact {
                    "[↑/↓] Slot  [d] Info  [o] Again  [c] Vault  [Esc/q] Back"
                } else {
                    "[↑/↓] Select  [Enter/d] Details  [o] Open Another  [c] Collection  [Esc/q] Back"
                }
            }
        }
    }

    fn on_enter(&mut self, ctx: &mut AppContext) {
        self.detail_scroll = 0;
        self.bind_active_card(ctx);
    }

    fn on_tick(&mut self, _ctx: &mut AppContext) {
        let Some(ref mut state) = self.state else {
            return;
        };
        match &mut state.phase {
            PackPhase::Ripping { ticks, max_ticks } => {
                *ticks = ticks.saturating_add(1);
                if *ticks >= *max_ticks {
                    let len = state.result.cards.len();
                    state.phase = PackPhase::Revealing {
                        cursor: 0,
                        revealed: vec![false; len],
                        flash_ticks: 0,
                    };
                    self.detail_scroll = 0;
                }
            }
            PackPhase::Revealing { flash_ticks, .. } => {
                if *flash_ticks > 0 {
                    *flash_ticks -= 1;
                }
            }
            PackPhase::Summary { .. } => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        enum Pending {
            BackToPacks,
            GoToCollection,
            OpenAnother,
            InspectAll(usize),
            BindMainCard(Box<Card>),
        }

        let mut pending: Option<Pending> = None;

        if let Some(ref mut state) = self.state {
            let total = state.result.cards.len();
            match &mut state.phase {
                PackPhase::Ripping { .. } => match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        state.phase = PackPhase::Revealing {
                            cursor: 0,
                            revealed: vec![false; total],
                            flash_ticks: 0,
                        };
                        self.detail_scroll = 0;
                    }
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                        pending = Some(Pending::BackToPacks);
                    }
                    _ => {}
                },
                PackPhase::Revealing {
                    cursor,
                    revealed,
                    flash_ticks,
                } => match key.code {
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                        pending = Some(Pending::BackToPacks);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.detail_scroll = self.detail_scroll.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        self.detail_scroll = self.detail_scroll.saturating_sub(5);
                    }
                    KeyCode::PageDown => {
                        self.detail_scroll = self.detail_scroll.saturating_add(5);
                    }
                    KeyCode::Left | KeyCode::Char('h') if total > 0 => {
                        *cursor = cursor.checked_sub(1).unwrap_or(total - 1);
                        self.detail_scroll = 0;
                        if revealed[*cursor] {
                            pending = Some(Pending::BindMainCard(Box::new(
                                state.result.cards[*cursor].card.clone(),
                            )));
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') if total > 0 => {
                        *cursor = (*cursor + 1) % total;
                        self.detail_scroll = 0;
                        if revealed[*cursor] {
                            pending = Some(Pending::BindMainCard(Box::new(
                                state.result.cards[*cursor].card.clone(),
                            )));
                        }
                    }
                    KeyCode::Char('a') if total > 0 => {
                        revealed.fill(true);
                        let last = total - 1;
                        *cursor = last;
                        self.detail_scroll = 0;
                        state.phase = PackPhase::Summary { cursor: last };
                        pending = Some(Pending::BindMainCard(Box::new(
                            state.result.cards[last].card.clone(),
                        )));
                    }
                    KeyCode::Enter | KeyCode::Char(' ') if total > 0 => {
                        if !revealed[*cursor] {
                            revealed[*cursor] = true;
                            self.detail_scroll = 0;
                            let pulled = &state.result.cards[*cursor];
                            *flash_ticks = if pulled.card.rarity.is_hit() { 16 } else { 5 };
                            pending = Some(Pending::BindMainCard(Box::new(pulled.card.clone())));
                        } else if revealed.iter().all(|&r| r) {
                            state.phase = PackPhase::Summary { cursor: *cursor };
                        } else if *cursor + 1 < total {
                            *cursor += 1;
                            self.detail_scroll = 0;
                            if revealed[*cursor] {
                                pending = Some(Pending::BindMainCard(Box::new(
                                    state.result.cards[*cursor].card.clone(),
                                )));
                            }
                        } else if let Some(next_unrevealed) = revealed.iter().position(|&r| !r) {
                            *cursor = next_unrevealed;
                            self.detail_scroll = 0;
                        }
                    }
                    _ => {}
                },
                PackPhase::Summary { cursor } => match key.code {
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                        pending = Some(Pending::BackToPacks);
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Up | KeyCode::Char('k')
                        if total > 0 =>
                    {
                        *cursor = cursor.checked_sub(1).unwrap_or(total - 1);
                        pending = Some(Pending::BindMainCard(Box::new(
                            state.result.cards[*cursor].card.clone(),
                        )));
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j')
                        if total > 0 =>
                    {
                        *cursor = (*cursor + 1) % total;
                        pending = Some(Pending::BindMainCard(Box::new(
                            state.result.cards[*cursor].card.clone(),
                        )));
                    }
                    KeyCode::Enter | KeyCode::Char('d') if total > 0 => {
                        pending = Some(Pending::InspectAll(*cursor));
                    }
                    KeyCode::Char('o') => {
                        pending = Some(Pending::OpenAnother);
                    }
                    KeyCode::Char('c') => {
                        pending = Some(Pending::GoToCollection);
                    }
                    _ => {}
                },
            }
        }

        match pending {
            Some(Pending::BackToPacks) => {
                self.state = None;
                ScreenAction::Navigate(Screen::PackSelect)
            }
            Some(Pending::GoToCollection) => {
                self.state = None;
                ScreenAction::Navigate(Screen::MyCollection)
            }
            Some(Pending::OpenAnother) => ScreenAction::OpenPack(ctx.active_pack_idx),
            Some(Pending::InspectAll(idx)) => {
                if let Some(ref state) = self.state {
                    let cards: Vec<InspectableCard> = state
                        .result
                        .cards
                        .iter()
                        .map(|p| InspectableCard {
                            card: p.card.clone(),
                            quantity: p.new_quantity,
                            first_acquired_at: None,
                            slot_note: Some(format!(
                                "{} • {}",
                                p.slot_label,
                                if p.is_new { "NEW DISCOVERY" } else { "DUPLICATE" }
                            )),
                        })
                        .collect();
                    return ScreenAction::OpenCardDetails {
                        cards,
                        idx,
                        return_screen: Screen::PackOpening,
                    };
                }
                ScreenAction::None
            }
            Some(Pending::BindMainCard(card)) => {
                ctx.request_card_into_slot("main", &card);
                ScreenAction::None
            }
            None => ScreenAction::None,
        }
    }
}

