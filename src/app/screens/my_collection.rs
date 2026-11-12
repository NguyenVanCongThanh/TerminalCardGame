use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::context::AppContext;
use crate::app::screens::card_details::InspectableCard;
use crate::app::screens::{Screen, ScreenAction, ScreenController};
use crate::domain::{Card, OwnedCard, Rarity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionSort {
    Newest,
    RarityDesc,
    QuantityDesc,
    CardNumber,
    NameAsc,
}

impl CollectionSort {
    pub fn label(self) -> &'static str {
        match self {
            CollectionSort::Newest => "Newest",
            CollectionSort::RarityDesc => "Rarity ↓",
            CollectionSort::QuantityDesc => "Qty ↓",
            CollectionSort::CardNumber => "Set #",
            CollectionSort::NameAsc => "Name A-Z",
        }
    }

    pub fn next(self) -> Self {
        match self {
            CollectionSort::Newest => CollectionSort::RarityDesc,
            CollectionSort::RarityDesc => CollectionSort::QuantityDesc,
            CollectionSort::QuantityDesc => CollectionSort::CardNumber,
            CollectionSort::CardNumber => CollectionSort::NameAsc,
            CollectionSort::NameAsc => CollectionSort::Newest,
        }
    }
}

pub struct MyCollectionScreen {
    pub collection_filter: Option<String>,
    pub rarity_filter: Option<Rarity>,
    pub search_query: String,
    pub search_active: bool,
    pub sort_mode: CollectionSort,
    pub owned_cursor: usize,
    pub grid_mode: bool,
    pub grid_page_size: usize,
}

impl Default for MyCollectionScreen {
    fn default() -> Self {
        Self {
            collection_filter: None,
            rarity_filter: None,
            search_query: String::new(),
            search_active: false,
            sort_mode: CollectionSort::Newest,
            owned_cursor: 0,
            grid_mode: false,
            grid_page_size: 4,
        }
    }
}

impl MyCollectionScreen {
    pub fn filtered_owned_cards<'a>(&self, owned_cards: &'a [OwnedCard]) -> Vec<&'a OwnedCard> {
        let query = self.search_query.trim().to_ascii_lowercase();
        let mut items: Vec<&OwnedCard> = owned_cards
            .iter()
            .filter(|o| match self.rarity_filter {
                Some(r) => o.card.rarity == r,
                None => true,
            })
            .filter(|o| {
                if query.is_empty() {
                    return true;
                }
                o.card.name.to_ascii_lowercase().contains(&query)
                    || o.card.card_number.to_ascii_lowercase().contains(&query)
                    || o.card.raw_rarity.to_ascii_lowercase().contains(&query)
                    || o.card
                        .metadata
                        .tags
                        .iter()
                        .any(|t| t.to_ascii_lowercase().contains(&query))
            })
            .collect();

        match self.sort_mode {
            CollectionSort::Newest => {}
            CollectionSort::RarityDesc => {
                items.sort_by(|a, b| {
                    b.card
                        .rarity
                        .cmp(&a.card.rarity)
                        .then_with(|| a.card.name.cmp(&b.card.name))
                });
            }
            CollectionSort::QuantityDesc => {
                items.sort_by(|a, b| {
                    b.quantity
                        .cmp(&a.quantity)
                        .then_with(|| b.card.rarity.cmp(&a.card.rarity))
                        .then_with(|| a.card.name.cmp(&b.card.name))
                });
            }
            CollectionSort::CardNumber => {
                items.sort_by(|a, b| {
                    let na = a.card.card_number.parse::<u32>().unwrap_or(u32::MAX);
                    let nb = b.card.card_number.parse::<u32>().unwrap_or(u32::MAX);
                    a.card
                        .collection_id
                        .cmp(&b.card.collection_id)
                        .then_with(|| na.cmp(&nb))
                        .then_with(|| a.card.card_number.cmp(&b.card.card_number))
                });
            }
            CollectionSort::NameAsc => {
                items.sort_by(|a, b| a.card.name.cmp(&b.card.name));
            }
        }
        items
    }

    pub fn grid_cols(&self) -> usize {
        match self.grid_page_size {
            6 => 3,
            _ => 2,
        }
    }

    pub fn clamp_cursor(&mut self, owned_cards: &[OwnedCard]) {
        let filtered_len = self.filtered_owned_cards(owned_cards).len();
        if filtered_len == 0 {
            self.owned_cursor = 0;
        } else if self.owned_cursor >= filtered_len {
            self.owned_cursor = filtered_len - 1;
        }
    }

    pub fn sync_grid_page_size(&mut self, new_size: usize, ctx: &mut AppContext) {
        let clamped = new_size.clamp(2, 6);
        if self.grid_page_size != clamped {
            self.grid_page_size = clamped;
            if self.grid_mode {
                self.sync_preview_slots(ctx);
            }
        }
    }

    pub fn sync_preview_slots(&self, ctx: &mut AppContext) {
        let filtered: Vec<Card> = self
            .filtered_owned_cards(&ctx.owned_cards)
            .into_iter()
            .map(|o| o.card.clone())
            .collect();

        if filtered.is_empty() {
            if let Some(s) = ctx.image_slots.get_mut("preview") {
                s.clear();
            }
            for i in 0..6 {
                if let Some(s) = ctx.image_slots.get_mut(&format!("grid-{i}")) {
                    s.clear();
                }
            }
            return;
        }

        if self.grid_mode {
            let page_size = self.grid_page_size.clamp(2, 6);
            let page_start = (self.owned_cursor / page_size) * page_size;
            for i in 0..6 {
                let slot_key = format!("grid-{i}");
                if i < page_size
                    && let Some(card) = filtered.get(page_start + i).cloned()
                {
                    ctx.request_card_into_slot(&slot_key, &card);
                } else if let Some(s) = ctx.image_slots.get_mut(&slot_key) {
                    s.clear();
                }
            }
        } else if let Some(card) = filtered.get(self.owned_cursor).cloned() {
            ctx.request_card_into_slot("preview", &card);
        }
    }
}

impl ScreenController for MyCollectionScreen {
    fn screen(&self) -> Screen {
        Screen::MyCollection
    }

    fn title(&self) -> &'static str {
        "My Collection"
    }

    fn footer_hints(&self, compact: bool) -> &'static str {
        if self.search_active {
            "[Type] Filter by name/tag  [Backspace] Delete  [Enter/Esc] Done Searching"
        } else if compact {
            "[↑/↓] Browse  [/] Search  [s] Sort  [v] View  [Tab] Set  [f] Rarity  [Esc] Back"
        } else {
            "[↑/↓/←/→] Browse  [Enter] Details  [/] Search  [s] Sort  [v] List/Grid  [Tab] Set  [f] Rarity  [Esc] Back"
        }
    }

    fn blocks_global_help(&self) -> bool {
        self.search_active
    }

    fn on_enter(&mut self, ctx: &mut AppContext) {
        ctx.refresh_cached_stats(self.collection_filter.as_deref());
        self.clamp_cursor(&ctx.owned_cards);
        self.sync_preview_slots(ctx);
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        if self.search_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.search_active = false;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.owned_cursor = 0;
                    self.sync_preview_slots(ctx);
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search_query.push(c);
                    self.owned_cursor = 0;
                    self.sync_preview_slots(ctx);
                }
                _ => {}
            }
            return ScreenAction::None;
        }

        let len = self.filtered_owned_cards(&ctx.owned_cards).len();
        let grid_cols = self.grid_cols();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                ScreenAction::Navigate(Screen::MainMenu)
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                ScreenAction::None
            }
            KeyCode::Char('s') => {
                self.sort_mode = self.sort_mode.next();
                self.owned_cursor = 0;
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Char('x') if !self.search_query.is_empty() => {
                self.search_query.clear();
                self.owned_cursor = 0;
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                let step = if self.grid_mode && len > grid_cols {
                    grid_cols
                } else {
                    1
                };
                self.owned_cursor = if self.owned_cursor >= step {
                    self.owned_cursor - step
                } else {
                    len - 1
                };
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                let step = if self.grid_mode && len > grid_cols {
                    grid_cols
                } else {
                    1
                };
                self.owned_cursor = (self.owned_cursor + step) % len;
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Left | KeyCode::Char('h') if len > 0 && self.grid_mode => {
                self.owned_cursor = self.owned_cursor.checked_sub(1).unwrap_or(len - 1);
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Right | KeyCode::Char('l') if len > 0 && self.grid_mode => {
                self.owned_cursor = (self.owned_cursor + 1) % len;
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Char('v') => {
                self.grid_mode = !self.grid_mode;
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Tab => {
                let ids: Vec<String> = ctx.collections.iter().map(|c| c.id.clone()).collect();
                self.collection_filter = match &self.collection_filter {
                    None => ids.first().cloned(),
                    Some(curr) => {
                        let pos = ids.iter().position(|id| id == curr);
                        match pos {
                            Some(i) if i + 1 < ids.len() => Some(ids[i + 1].clone()),
                            _ => None,
                        }
                    }
                };
                self.owned_cursor = 0;
                ctx.refresh_owned_cards(self.collection_filter.as_deref());
                self.clamp_cursor(&ctx.owned_cards);
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Char('f') => {
                let rarities = Rarity::all();
                self.rarity_filter = match self.rarity_filter {
                    None => Some(rarities[0]),
                    Some(curr) => {
                        let pos = rarities.iter().position(|&r| r == curr);
                        match pos {
                            Some(i) if i + 1 < rarities.len() => Some(rarities[i + 1]),
                            _ => None,
                        }
                    }
                };
                self.owned_cursor = 0;
                self.sync_preview_slots(ctx);
                ScreenAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') if len > 0 => {
                let cards: Vec<InspectableCard> = self
                    .filtered_owned_cards(&ctx.owned_cards)
                    .into_iter()
                    .map(|o| InspectableCard {
                        card: o.card.clone(),
                        quantity: o.quantity,
                        first_acquired_at: Some(o.first_acquired_at.clone()),
                        slot_note: None,
                    })
                    .collect();
                ScreenAction::OpenCardDetails {
                    cards,
                    idx: self.owned_cursor,
                    return_screen: Screen::MyCollection,
                }
            }
            _ => ScreenAction::None,
        }
    }
}

