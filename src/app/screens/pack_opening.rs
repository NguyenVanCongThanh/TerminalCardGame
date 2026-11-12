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
}

impl PackOpeningScreen {
    pub fn start(&mut self, result: PackOpeningResult, fast_animations: bool) {
        let max_ticks = if fast_animations { 4 } else { 10 };
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
        if compact {
            "[Space] Flip/Next  [←/→] Slot  [a] All  [d] Info  [o] Again  [Esc] Back"
        } else {
            "[Space/Enter] Flip/Next  [←/→] Browse  [a] Reveal All  [d] Details  [o] Open Another  [c] Collection  [Esc] Back"
        }
    }

    fn on_enter(&mut self, ctx: &mut AppContext) {
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
                    }
                    KeyCode::Esc => pending = Some(Pending::BackToPacks),
                    _ => {}
                },
                PackPhase::Revealing {
                    cursor,
                    revealed,
                    flash_ticks,
                } => match key.code {
                    KeyCode::Esc => pending = Some(Pending::BackToPacks),
                    KeyCode::Left | KeyCode::Char('h') => {
                        *cursor = cursor.checked_sub(1).unwrap_or(total - 1);
                        if revealed[*cursor] {
                            pending = Some(Pending::BindMainCard(Box::new(
                                state.result.cards[*cursor].card.clone(),
                            )));
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        *cursor = (*cursor + 1) % total;
                        if revealed[*cursor] {
                            pending = Some(Pending::BindMainCard(Box::new(
                                state.result.cards[*cursor].card.clone(),
                            )));
                        }
                    }
                    KeyCode::Char('a') => {
                        revealed.fill(true);
                        let last = total.saturating_sub(1);
                        *cursor = last;
                        state.phase = PackPhase::Summary { cursor: last };
                        pending = Some(Pending::BindMainCard(Box::new(
                            state.result.cards[last].card.clone(),
                        )));
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if !revealed[*cursor] {
                            revealed[*cursor] = true;
                            let pulled = &state.result.cards[*cursor];
                            *flash_ticks = if pulled.card.rarity.is_hit() { 16 } else { 5 };
                            pending = Some(Pending::BindMainCard(Box::new(pulled.card.clone())));
                        } else if *cursor + 1 < total {
                            *cursor += 1;
                            if revealed[*cursor] {
                                pending = Some(Pending::BindMainCard(Box::new(
                                    state.result.cards[*cursor].card.clone(),
                                )));
                            }
                        } else if revealed.iter().all(|&r| r) {
                            state.phase = PackPhase::Summary { cursor: *cursor };
                        } else if let Some(next_unrevealed) = revealed.iter().position(|&r| !r) {
                            *cursor = next_unrevealed;
                        }
                    }
                    _ => {}
                },
                PackPhase::Summary { cursor } => match key.code {
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                        pending = Some(Pending::BackToPacks);
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Up | KeyCode::Char('k') => {
                        *cursor = cursor.checked_sub(1).unwrap_or(total - 1);
                        pending = Some(Pending::BindMainCard(Box::new(
                            state.result.cards[*cursor].card.clone(),
                        )));
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') => {
                        *cursor = (*cursor + 1) % total;
                        pending = Some(Pending::BindMainCard(Box::new(
                            state.result.cards[*cursor].card.clone(),
                        )));
                    }
                    KeyCode::Enter | KeyCode::Char('d') => {
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

