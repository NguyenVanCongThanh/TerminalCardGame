use crossterm::event::{KeyCode, KeyEvent};

use crate::app::context::AppContext;
use crate::app::screens::card_details::InspectableCard;
use crate::app::screens::{Screen, ScreenAction, ScreenController};
use crate::storage::CardRepository;

#[derive(Default)]
pub struct MainMenuScreen {
    pub menu_idx: usize,
    pub focus_history: bool,
    pub history_idx: usize,
}

impl MainMenuScreen {
    fn activate_item(&mut self, idx: usize) -> ScreenAction {
        self.menu_idx = idx;
        match idx {
            0 => ScreenAction::Navigate(Screen::CollectionSelect),
            1 => ScreenAction::Navigate(Screen::UniverseSelect),
            2 => ScreenAction::Navigate(Screen::MyCollection),
            3 => ScreenAction::Navigate(Screen::Settings),
            4 => ScreenAction::Quit,
            _ => ScreenAction::None,
        }
    }

    fn inspect_history_entry(&self, ctx: &AppContext) -> ScreenAction {
        let Some(entry) = ctx.pack_history.get(self.history_idx) else {
            return ScreenAction::None;
        };
        let Ok(cards) = ctx.store.list_cards_by_ids(&entry.card_ids) else {
            return ScreenAction::None;
        };
        if cards.is_empty() {
            return ScreenAction::None;
        }
        let inspectables: Vec<InspectableCard> = cards
            .into_iter()
            .enumerate()
            .map(|(i, card)| {
                let qty = ctx
                    .owned_cards
                    .iter()
                    .find(|o| o.card.id == card.id)
                    .map(|o| o.quantity)
                    .unwrap_or(1);
                InspectableCard {
                    card,
                    quantity: qty,
                    first_acquired_at: Some(entry.opened_at.clone()),
                    slot_note: Some(format!("{} (Slot {})", entry.pack_name, i + 1)),
                }
            })
            .collect();
        ScreenAction::OpenCardDetails {
            cards: inspectables,
            idx: 0,
            return_screen: Screen::MainMenu,
        }
    }
}

impl ScreenController for MainMenuScreen {
    fn screen(&self) -> Screen {
        Screen::MainMenu
    }

    fn title(&self) -> &'static str {
        "Main Menu"
    }

    fn footer_hints(&self, compact: bool) -> &'static str {
        if compact {
            "[↑/↓] Move  [Tab] History  [Enter] Select  [?] Help  [q] Quit"
        } else {
            "[↑/↓] Navigate  [Tab] History  [Enter] Select  [1-5] Quick Jump  [?] Help  [q] Quit"
        }
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        const MENU_LEN: usize = 5;
        if key.code == KeyCode::Tab && !ctx.pack_history.is_empty() {
            self.focus_history = !self.focus_history;
            if self.history_idx >= ctx.pack_history.len() {
                self.history_idx = 0;
            }
            return ScreenAction::None;
        }

        if self.focus_history && !ctx.pack_history.is_empty() {
            let hlen = ctx.pack_history.len();
            return match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.history_idx = self.history_idx.checked_sub(1).unwrap_or(hlen - 1);
                    ScreenAction::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.history_idx = (self.history_idx + 1) % hlen;
                    ScreenAction::None
                }
                KeyCode::Enter | KeyCode::Char(' ') => self.inspect_history_entry(ctx),
                KeyCode::Esc => {
                    self.focus_history = false;
                    ScreenAction::None
                }
                KeyCode::Char('1') => self.activate_item(0),
                KeyCode::Char('2') => self.activate_item(1),
                KeyCode::Char('3') => self.activate_item(2),
                KeyCode::Char('4') => self.activate_item(3),
                KeyCode::Char('5') | KeyCode::Char('q') => ScreenAction::Quit,
                _ => ScreenAction::None,
            };
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.menu_idx = self.menu_idx.checked_sub(1).unwrap_or(MENU_LEN - 1);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.menu_idx = (self.menu_idx + 1) % MENU_LEN;
                ScreenAction::None
            }
            KeyCode::Char('1') => self.activate_item(0),
            KeyCode::Char('2') => self.activate_item(1),
            KeyCode::Char('3') => self.activate_item(2),
            KeyCode::Char('4') => self.activate_item(3),
            KeyCode::Char('5') | KeyCode::Char('q') => ScreenAction::Quit,
            KeyCode::Enter | KeyCode::Char(' ') => self.activate_item(self.menu_idx),
            _ => ScreenAction::None,
        }
    }
}

