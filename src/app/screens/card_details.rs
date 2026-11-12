use crossterm::event::{KeyCode, KeyEvent};

use crate::app::context::AppContext;
use crate::app::screens::{Screen, ScreenAction, ScreenController};
use crate::domain::Card;

#[derive(Debug, Clone)]
pub struct InspectableCard {
    pub card: Card,
    pub quantity: u32,
    pub first_acquired_at: Option<String>,
    pub slot_note: Option<String>,
}

pub struct CardDetailsScreen {
    pub cards: Vec<InspectableCard>,
    pub idx: usize,
    pub return_screen: Screen,
    pub scroll: u16,
}

impl Default for CardDetailsScreen {
    fn default() -> Self {
        Self {
            cards: Vec::new(),
            idx: 0,
            return_screen: Screen::MyCollection,
            scroll: 0,
        }
    }
}

impl CardDetailsScreen {
    pub fn open(
        &mut self,
        cards: Vec<InspectableCard>,
        idx: usize,
        return_screen: Screen,
        ctx: &mut AppContext,
    ) {
        self.cards = cards;
        self.idx = idx.min(self.cards.len().saturating_sub(1));
        self.return_screen = return_screen;
        self.scroll = 0;
        if let Some(item) = self.cards.get(self.idx) {
            let card = item.card.clone();
            ctx.request_card_into_slot("main", &card);
        }
    }
}

impl ScreenController for CardDetailsScreen {
    fn screen(&self) -> Screen {
        Screen::CardDetails
    }

    fn title(&self) -> &'static str {
        "Card Details"
    }

    fn footer_hints(&self, _compact: bool) -> &'static str {
        "[←/→] Prev / Next Card  [↑/↓] Scroll Info  [?] Help  [Esc] Back"
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        let len = self.cards.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                ScreenAction::Navigate(self.return_screen)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                ScreenAction::None
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(6);
                ScreenAction::None
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(6);
                ScreenAction::None
            }
            KeyCode::Left | KeyCode::Char('h') if len > 0 => {
                self.idx = self.idx.checked_sub(1).unwrap_or(len - 1);
                self.scroll = 0;
                let card = self.cards[self.idx].card.clone();
                ctx.request_card_into_slot("main", &card);
                ScreenAction::None
            }
            KeyCode::Right | KeyCode::Char('l') if len > 0 => {
                self.idx = (self.idx + 1) % len;
                self.scroll = 0;
                let card = self.cards[self.idx].card.clone();
                ctx.request_card_into_slot("main", &card);
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }
}

