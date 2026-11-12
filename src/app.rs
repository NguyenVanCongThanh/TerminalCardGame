use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::DynamicImage;
use rand::rng;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::domain::{
    AppSettings, Card, Collection, CollectionProgress, ImageQuality, OwnedCard, PackConfig,
    PackHistoryEntry, PackOpeningResult, Rarity, Universe,
};
use crate::error::AppResult;
use crate::images::renderer::{CardImageSlot, ImageRenderer, ResizedSlotEvent};
use crate::images::{CacheStats, ImageCache};
use crate::pack_engine::PackGenerator;
use crate::providers::ProviderRegistry;
use crate::storage::sqlite::SqliteStore;
use crate::storage::{
    CardRepository, CollectionRepository, InventoryRepository, SettingsRepository,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    UniverseSelect,
    CollectionSelect,
    PackSelect,
    PackOpening,
    MyCollection,
    CardDetails,
    Settings,
}

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

#[derive(Debug, Clone)]
pub struct InspectableCard {
    pub card: Card,
    pub quantity: u32,
    pub first_acquired_at: Option<String>,
    pub slot_note: Option<String>,
}

pub enum AsyncMessage {
    CollectionsSynced {
        result: AppResult<Vec<Collection>>,
    },
    CardsSynced {
        collection_id: String,
        open_pack_after: bool,
        result: AppResult<Vec<Card>>,
    },
    ImageLoaded {
        card_id: String,
        quality: ImageQuality,
        result: AppResult<(DynamicImage, PathBuf)>,
    },
    ImageResized(ResizedSlotEvent),
}

#[derive(Debug, Clone)]
pub struct StatusBanner {
    pub message: String,
    pub is_error: bool,
    pub ttl_ticks: u64,
}

pub struct App {
    pub running: bool,
    pub screen: Screen,
    pub tick: u64,

    pub store: Arc<SqliteStore>,
    pub providers: Arc<ProviderRegistry>,
    pub image_cache: Arc<ImageCache>,
    pub renderer: ImageRenderer,
    pub settings: AppSettings,

    pub async_tx: UnboundedSender<AsyncMessage>,
    pub async_rx: UnboundedReceiver<AsyncMessage>,
    pub resize_tx: UnboundedSender<ResizedSlotEvent>,
    pub image_slots: HashMap<String, CardImageSlot>,
    in_flight_images: HashSet<String>,

    // Domain data cached in memory for responsive UI
    pub universes: Vec<Universe>,
    pub collections: Vec<Collection>,
    pub collection_progress: Vec<CollectionProgress>,
    pub cached_card_counts: HashMap<String, usize>,
    pub cached_rarity_counts: HashMap<Rarity, usize>,
    pub pack_configs: Vec<PackConfig>,
    pub owned_cards: Vec<OwnedCard>,
    pub pack_history: Vec<PackHistoryEntry>,
    pub cache_stats: CacheStats,

    // Screen-specific cursors & states
    pub menu_idx: usize,
    pub universe_idx: usize,
    pub selected_universe_id: String,
    pub collection_idx: usize,
    pub selected_collection_id: String,
    pub syncing_collection_id: Option<String>,
    pub syncing_collections_list: bool,
    pub pack_idx: usize,
    pub pack_opening: Option<PackOpeningState>,

    // My Collection filters & view
    pub collection_filter: Option<String>,
    pub rarity_filter: Option<Rarity>,
    pub owned_cursor: usize,
    pub grid_mode: bool,

    // Card details view
    pub detail_cards: Vec<InspectableCard>,
    pub detail_idx: usize,
    pub detail_return_screen: Screen,

    // Settings cursor
    pub settings_idx: usize,

    pub banner: Option<StatusBanner>,
}

impl App {
    pub fn new(
        store: Arc<SqliteStore>,
        providers: Arc<ProviderRegistry>,
        image_cache: Arc<ImageCache>,
        renderer: ImageRenderer,
    ) -> AppResult<Self> {
        let settings = store.load_settings()?;
        let universes = store.list_universes()?;
        let selected_universe_id = universes
            .iter()
            .find(|u| u.is_implemented)
            .map(|u| u.id.clone())
            .unwrap_or_else(|| "pokemon".to_string());

        let collections = store.list_collections(&selected_universe_id)?;
        let selected_collection_id = collections
            .first()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| "base1".to_string());

        let (async_tx, async_rx) = unbounded_channel();
        let (resize_tx, mut resize_rx) = unbounded_channel::<ResizedSlotEvent>();

        // Forward resize completions into unified async_tx
        let forward_tx = async_tx.clone();
        tokio::spawn(async move {
            while let Some(ev) = resize_rx.recv().await {
                let _ = forward_tx.send(AsyncMessage::ImageResized(ev));
            }
        });

        let mut image_slots = HashMap::new();
        for key in ["main", "preview", "grid-0", "grid-1", "grid-2", "grid-3"] {
            image_slots.insert(key.to_string(), CardImageSlot::new(key, resize_tx.clone()));
        }

        let mut app = Self {
            running: true,
            screen: Screen::MainMenu,
            tick: 0,
            store,
            providers,
            image_cache,
            renderer,
            settings,
            async_tx,
            async_rx,
            resize_tx,
            image_slots,
            in_flight_images: HashSet::new(),
            universes,
            collections,
            collection_progress: Vec::new(),
            cached_card_counts: HashMap::new(),
            cached_rarity_counts: HashMap::new(),
            pack_configs: PackConfig::default_configs(),
            owned_cards: Vec::new(),
            pack_history: Vec::new(),
            cache_stats: CacheStats::default(),
            menu_idx: 0,
            universe_idx: 0,
            selected_universe_id,
            collection_idx: 0,
            selected_collection_id,
            syncing_collection_id: None,
            syncing_collections_list: false,
            pack_idx: 0,
            pack_opening: None,
            collection_filter: None,
            rarity_filter: None,
            owned_cursor: 0,
            grid_mode: false,
            detail_cards: Vec::new(),
            detail_idx: 0,
            detail_return_screen: Screen::MyCollection,
            settings_idx: 0,
            banner: None,
        };

        app.refresh_cached_stats();
        Ok(app)
    }

    pub fn set_banner(&mut self, message: impl Into<String>, is_error: bool) {
        self.banner = Some(StatusBanner {
            message: message.into(),
            is_error,
            ttl_ticks: if is_error { 90 } else { 55 },
        });
    }

    pub fn refresh_cached_stats(&mut self) {
        if let Ok(cols) = self.store.list_collections(&self.selected_universe_id) {
            self.collections = cols;
        }
        if let Ok(prog) = self.store.list_collection_progress(&self.selected_universe_id) {
            self.collection_progress = prog;
        }
        self.cached_card_counts.clear();
        for col in &self.collections {
            let count = self.store.count_cards_in_collection(&col.id).unwrap_or(0);
            self.cached_card_counts.insert(col.id.clone(), count);
        }
        self.refresh_selected_collection_rarities();
        self.refresh_owned_cards();
        if let Ok(hist) = self.store.list_pack_history(8) {
            self.pack_history = hist;
        }
        self.cache_stats = self.image_cache.stats();
    }

    fn refresh_selected_collection_rarities(&mut self) {
        self.cached_rarity_counts.clear();
        if let Ok(cards) = self
            .store
            .list_cards_in_collection(&self.selected_collection_id)
        {
            for card in cards {
                *self.cached_rarity_counts.entry(card.rarity).or_insert(0) += 1;
            }
        }
    }

    pub fn refresh_owned_cards(&mut self) {
        if let Ok(owned) = self
            .store
            .list_owned_cards(self.collection_filter.as_deref())
        {
            self.owned_cards = owned;
            let filtered_len = self.filtered_owned_cards().len();
            if filtered_len == 0 {
                self.owned_cursor = 0;
            } else if self.owned_cursor >= filtered_len {
                self.owned_cursor = filtered_len - 1;
            }
        }
    }

    pub fn filtered_owned_cards(&self) -> Vec<&OwnedCard> {
        self.owned_cards
            .iter()
            .filter(|o| match self.rarity_filter {
                Some(r) => o.card.rarity == r,
                None => true,
            })
            .collect()
    }

    pub fn active_universe(&self) -> Option<&Universe> {
        self.universes
            .iter()
            .find(|u| u.id == self.selected_universe_id)
    }

    pub fn active_collection(&self) -> Option<&Collection> {
        self.collections
            .iter()
            .find(|c| c.id == self.selected_collection_id)
    }

    pub fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);

        if let Some(ref mut b) = self.banner {
            if b.ttl_ticks > 0 {
                b.ttl_ticks -= 1;
            }
            if b.ttl_ticks == 0 {
                self.banner = None;
            }
        }

        if self.screen == Screen::PackOpening
            && let Some(ref mut state) = self.pack_opening
        {
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
    }

    pub fn handle_async_message(&mut self, msg: AsyncMessage) {
        match msg {
            AsyncMessage::CollectionsSynced { result } => {
                self.syncing_collections_list = false;
                match result {
                    Ok(cols) => {
                        if let Err(e) = self.store.upsert_collections(&cols) {
                            self.set_banner(format!("Failed saving sets: {e}"), true);
                        } else {
                            self.refresh_cached_stats();
                            self.set_banner(
                                format!("Synced {} Pokémon sets from TCGdex", cols.len()),
                                false,
                            );
                        }
                    }
                    Err(e) => {
                        self.set_banner(
                            format!("Online set sync failed (using cached sets): {e}"),
                            true,
                        );
                    }
                }
            }
            AsyncMessage::CardsSynced {
                collection_id,
                open_pack_after,
                result,
            } => {
                self.syncing_collection_id = None;
                match result {
                    Ok(cards) => {
                        let count = cards.len();
                        if let Err(e) = self.store.upsert_cards(&cards) {
                            self.set_banner(format!("Failed caching cards: {e}"), true);
                            return;
                        }
                        let _ = self
                            .store
                            .mark_collection_synced(&collection_id, &Utc::now().to_rfc3339());
                        self.refresh_cached_stats();
                        self.set_banner(
                            format!("Synced {count} cards for '{collection_id}'"),
                            false,
                        );
                        if open_pack_after && self.screen == Screen::CollectionSelect {
                            self.screen = Screen::PackSelect;
                            self.pack_idx = 0;
                        }
                    }
                    Err(e) => {
                        let cached = self.store.count_cards_in_collection(&collection_id).unwrap_or(0);
                        if cached > 0 {
                            self.set_banner(
                                format!("Network error ({e}); using {cached} cached cards"),
                                true,
                            );
                            if open_pack_after && self.screen == Screen::CollectionSelect {
                                self.screen = Screen::PackSelect;
                            }
                        } else {
                            self.set_banner(
                                format!("Sync failed for '{collection_id}': {e}"),
                                true,
                            );
                        }
                    }
                }
            }
            AsyncMessage::ImageLoaded {
                card_id,
                quality,
                result,
            } => {
                let key = ImageCache::cache_key(&card_id, quality);
                self.in_flight_images.remove(&key);
                match result {
                    Ok((img, path)) => {
                        let path_str = path.to_string_lossy().to_string();
                        let _ = self.store.update_local_image_path(&card_id, &path_str);
                        self.cache_stats = self.image_cache.stats();

                        for slot in self.image_slots.values_mut() {
                            if slot.assigned_card_id.as_deref() == Some(&card_id) {
                                slot.set_image(&card_id, &self.renderer, img.clone());
                            }
                        }
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        for slot in self.image_slots.values_mut() {
                            if slot.assigned_card_id.as_deref() == Some(&card_id) {
                                slot.set_error(&card_id, msg.clone());
                            }
                        }
                    }
                }
            }
            AsyncMessage::ImageResized(ev) => {
                if let Some(slot) = self.image_slots.get_mut(&ev.slot_key) {
                    slot.apply_resize(ev.response);
                }
            }
        }
    }

    pub fn handle_terminal_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                self.running = false;
                return;
            }
            self.handle_key(key);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.screen {
            Screen::MainMenu => self.handle_main_menu_key(key),
            Screen::UniverseSelect => self.handle_universe_key(key),
            Screen::CollectionSelect => self.handle_collection_select_key(key),
            Screen::PackSelect => self.handle_pack_select_key(key),
            Screen::PackOpening => self.handle_pack_opening_key(key),
            Screen::MyCollection => self.handle_my_collection_key(key),
            Screen::CardDetails => self.handle_card_details_key(key),
            Screen::Settings => self.handle_settings_key(key),
        }
    }

    fn handle_main_menu_key(&mut self, key: KeyEvent) {
        const MENU_LEN: usize = 5;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.menu_idx = self.menu_idx.checked_sub(1).unwrap_or(MENU_LEN - 1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.menu_idx = (self.menu_idx + 1) % MENU_LEN;
            }
            KeyCode::Char('1') => self.activate_main_menu(0),
            KeyCode::Char('2') => self.activate_main_menu(1),
            KeyCode::Char('3') => self.activate_main_menu(2),
            KeyCode::Char('4') => self.activate_main_menu(3),
            KeyCode::Char('5') | KeyCode::Char('q') => self.running = false,
            KeyCode::Enter | KeyCode::Char(' ') => self.activate_main_menu(self.menu_idx),
            _ => {}
        }
    }

    fn activate_main_menu(&mut self, idx: usize) {
        self.menu_idx = idx;
        match idx {
            0 => {
                // Open Packs -> straight into Collection Select for active universe
                self.refresh_cached_stats();
                self.screen = Screen::CollectionSelect;
            }
            1 => {
                self.screen = Screen::UniverseSelect;
            }
            2 => {
                self.enter_my_collection();
            }
            3 => {
                self.cache_stats = self.image_cache.stats();
                self.screen = Screen::Settings;
            }
            4 => self.running = false,
            _ => {}
        }
    }

    fn handle_universe_key(&mut self, key: KeyEvent) {
        let len = self.universes.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                self.screen = Screen::MainMenu;
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.universe_idx = self.universe_idx.checked_sub(1).unwrap_or(len - 1);
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.universe_idx = (self.universe_idx + 1) % len;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(u) = self.universes.get(self.universe_idx).cloned() {
                    if u.is_implemented {
                        self.selected_universe_id = u.id;
                        self.collection_idx = 0;
                        self.refresh_cached_stats();
                        self.screen = Screen::CollectionSelect;
                    } else {
                        self.set_banner(
                            format!("'{}' is a planned future universe slot!", u.name),
                            false,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_collection_select_key(&mut self, key: KeyEvent) {
        let len = self.collections.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                self.screen = Screen::MainMenu;
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.collection_idx = self.collection_idx.checked_sub(1).unwrap_or(len - 1);
                if let Some(col) = self.collections.get(self.collection_idx) {
                    self.selected_collection_id = col.id.clone();
                    self.refresh_selected_collection_rarities();
                }
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.collection_idx = (self.collection_idx + 1) % len;
                if let Some(col) = self.collections.get(self.collection_idx) {
                    self.selected_collection_id = col.id.clone();
                    self.refresh_selected_collection_rarities();
                }
            }
            KeyCode::Char('s') => {
                // Sync currently highlighted collection without leaving screen
                if let Some(col) = self.collections.get(self.collection_idx).cloned() {
                    self.spawn_sync_collection_cards(col.id, false);
                }
            }
            KeyCode::Char('r') => {
                // Refresh collection metadata list from API
                self.spawn_sync_collections_list();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(col) = self.collections.get(self.collection_idx).cloned() {
                    self.selected_collection_id = col.id.clone();
                    self.refresh_selected_collection_rarities();
                    let cached = self
                        .cached_card_counts
                        .get(&col.id)
                        .copied()
                        .unwrap_or(0);
                    if cached > 0 {
                        self.screen = Screen::PackSelect;
                        self.pack_idx = 0;
                    } else {
                        self.spawn_sync_collection_cards(col.id, true);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_pack_select_key(&mut self, key: KeyEvent) {
        let len = self.pack_configs.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                self.screen = Screen::CollectionSelect;
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.pack_idx = self.pack_idx.checked_sub(1).unwrap_or(len - 1);
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.pack_idx = (self.pack_idx + 1) % len;
            }
            KeyCode::Char('s') => {
                self.spawn_sync_collection_cards(self.selected_collection_id.clone(), false);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.open_selected_pack();
            }
            _ => {}
        }
    }

    fn handle_pack_opening_key(&mut self, key: KeyEvent) {
        enum Action {
            BackToPacks,
            GoToCollection,
            OpenAnother,
            InspectAll(usize),
            BindMainCard(Box<Card>),
        }

        let mut action: Option<Action> = None;

        if let Some(ref mut state) = self.pack_opening {
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
                    KeyCode::Esc => action = Some(Action::BackToPacks),
                    _ => {}
                },
                PackPhase::Revealing {
                    cursor,
                    revealed,
                    flash_ticks,
                } => match key.code {
                    KeyCode::Esc => action = Some(Action::BackToPacks),
                    KeyCode::Left | KeyCode::Char('h') => {
                        *cursor = cursor.checked_sub(1).unwrap_or(total - 1);
                        if revealed[*cursor] {
                            action = Some(Action::BindMainCard(Box::new(
                                state.result.cards[*cursor].card.clone(),
                            )));
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        *cursor = (*cursor + 1) % total;
                        if revealed[*cursor] {
                            action = Some(Action::BindMainCard(Box::new(
                                state.result.cards[*cursor].card.clone(),
                            )));
                        }
                    }
                    KeyCode::Char('a') => {
                        revealed.fill(true);
                        let last = total.saturating_sub(1);
                        *cursor = last;
                        state.phase = PackPhase::Summary { cursor: last };
                        action = Some(Action::BindMainCard(Box::new(
                            state.result.cards[last].card.clone(),
                        )));
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if !revealed[*cursor] {
                            revealed[*cursor] = true;
                            let pulled = &state.result.cards[*cursor];
                            if pulled.card.rarity.is_hit() {
                                *flash_ticks = 14;
                            }
                            action = Some(Action::BindMainCard(Box::new(pulled.card.clone())));
                        } else if *cursor + 1 < total {
                            *cursor += 1;
                            if revealed[*cursor] {
                                action = Some(Action::BindMainCard(Box::new(
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
                        action = Some(Action::BackToPacks);
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Up | KeyCode::Char('k') => {
                        *cursor = cursor.checked_sub(1).unwrap_or(total - 1);
                        action = Some(Action::BindMainCard(Box::new(
                            state.result.cards[*cursor].card.clone(),
                        )));
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') => {
                        *cursor = (*cursor + 1) % total;
                        action = Some(Action::BindMainCard(Box::new(
                            state.result.cards[*cursor].card.clone(),
                        )));
                    }
                    KeyCode::Enter | KeyCode::Char('d') => {
                        action = Some(Action::InspectAll(*cursor));
                    }
                    KeyCode::Char('o') => {
                        action = Some(Action::OpenAnother);
                    }
                    KeyCode::Char('c') => {
                        action = Some(Action::GoToCollection);
                    }
                    _ => {}
                },
            }
        }

        if let Some(act) = action {
            match act {
                Action::BackToPacks => {
                    self.pack_opening = None;
                    self.screen = Screen::PackSelect;
                }
                Action::GoToCollection => {
                    self.pack_opening = None;
                    self.enter_my_collection();
                }
                Action::OpenAnother => {
                    self.open_selected_pack();
                }
                Action::InspectAll(idx) => {
                    if let Some(ref state) = self.pack_opening {
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
                        self.open_card_details(cards, idx, Screen::PackOpening);
                    }
                }
                Action::BindMainCard(card) => {
                    self.request_card_into_slot("main", &card);
                }
            }
        }
    }

    fn handle_my_collection_key(&mut self, key: KeyEvent) {
        let len = self.filtered_owned_cards().len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                self.screen = Screen::MainMenu;
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                let step = if self.grid_mode && len > 2 { 2 } else { 1 };
                self.owned_cursor = if self.owned_cursor >= step {
                    self.owned_cursor - step
                } else {
                    len - 1
                };
                self.sync_collection_preview_slots();
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                let step = if self.grid_mode && len > 2 { 2 } else { 1 };
                self.owned_cursor = (self.owned_cursor + step) % len;
                self.sync_collection_preview_slots();
            }
            KeyCode::Left | KeyCode::Char('h') if len > 0 && self.grid_mode => {
                self.owned_cursor = self.owned_cursor.checked_sub(1).unwrap_or(len - 1);
                self.sync_collection_preview_slots();
            }
            KeyCode::Right | KeyCode::Char('l') if len > 0 && self.grid_mode => {
                self.owned_cursor = (self.owned_cursor + 1) % len;
                self.sync_collection_preview_slots();
            }
            KeyCode::Char('v') => {
                self.grid_mode = !self.grid_mode;
                self.sync_collection_preview_slots();
            }
            KeyCode::Tab => {
                // Cycle collection filter: None -> each collection -> None
                let ids: Vec<String> = self.collections.iter().map(|c| c.id.clone()).collect();
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
                self.refresh_owned_cards();
                self.sync_collection_preview_slots();
            }
            KeyCode::Char('f') => {
                // Cycle rarity filter
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
                self.sync_collection_preview_slots();
            }
            KeyCode::Enter | KeyCode::Char(' ') if len > 0 => {
                let cards: Vec<InspectableCard> = self
                    .filtered_owned_cards()
                    .into_iter()
                    .map(|o| InspectableCard {
                        card: o.card.clone(),
                        quantity: o.quantity,
                        first_acquired_at: Some(o.first_acquired_at.clone()),
                        slot_note: None,
                    })
                    .collect();
                self.open_card_details(cards, self.owned_cursor, Screen::MyCollection);
            }
            _ => {}
        }
    }

    fn handle_card_details_key(&mut self, key: KeyEvent) {
        let len = self.detail_cards.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                let ret = self.detail_return_screen;
                self.screen = ret;
                if ret == Screen::PackOpening {
                    if let Some(ref state) = self.pack_opening
                        && let PackPhase::Summary { cursor } | PackPhase::Revealing { cursor, .. } =
                            state.phase
                    {
                        let card = state.result.cards[cursor].card.clone();
                        self.request_card_into_slot("main", &card);
                    }
                } else if ret == Screen::MyCollection {
                    self.sync_collection_preview_slots();
                }
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.detail_idx = self.detail_idx.checked_sub(1).unwrap_or(len - 1);
                let card = self.detail_cards[self.detail_idx].card.clone();
                self.request_card_into_slot("main", &card);
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.detail_idx = (self.detail_idx + 1) % len;
                let card = self.detail_cards[self.detail_idx].card.clone();
                self.request_card_into_slot("main", &card);
            }
            _ => {}
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        const SETTINGS_ROWS: usize = 5;
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                self.screen = Screen::MainMenu;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.settings_idx = self.settings_idx.checked_sub(1).unwrap_or(SETTINGS_ROWS - 1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.settings_idx = (self.settings_idx + 1) % SETTINGS_ROWS;
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter | KeyCode::Char(' ') => {
                self.adjust_setting(true);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.adjust_setting(false);
            }
            _ => {}
        }
    }

    fn adjust_setting(&mut self, forward: bool) {
        match self.settings_idx {
            0 => {
                self.settings.protocol_mode = if forward {
                    self.settings.protocol_mode.next()
                } else {
                    self.settings.protocol_mode.prev()
                };
                self.renderer.set_mode(self.settings.protocol_mode);
                // Re-encode any currently loaded images cleanly under new protocol
                for slot in self.image_slots.values_mut() {
                    slot.clear();
                }
                let _ = self.store.save_settings(&self.settings);
                self.set_banner(
                    format!(
                        "Image protocol set to: {}",
                        self.settings.protocol_mode.label()
                    ),
                    false,
                );
            }
            1 => {
                self.settings.image_quality = self.settings.image_quality.toggle();
                let _ = self.store.save_settings(&self.settings);
                self.set_banner(
                    format!(
                        "Artwork quality: {}",
                        self.settings.image_quality.label()
                    ),
                    false,
                );
            }
            2 => {
                self.settings.fast_animations = !self.settings.fast_animations;
                let _ = self.store.save_settings(&self.settings);
            }
            3 => {
                const SIZES: [usize; 4] = [16, 28, 40, 60];
                let idx = SIZES
                    .iter()
                    .position(|&s| s == self.settings.cards_per_sync)
                    .unwrap_or(1);
                let next = if forward {
                    (idx + 1) % SIZES.len()
                } else {
                    idx.checked_sub(1).unwrap_or(SIZES.len() - 1)
                };
                self.settings.cards_per_sync = SIZES[next];
                let _ = self.store.save_settings(&self.settings);
            }
            4 => {
                if let Err(e) = self.image_cache.clear_cache() {
                    self.set_banner(format!("Failed clearing cache: {e}"), true);
                } else {
                    let _ = self.store.clear_local_image_paths();
                    for slot in self.image_slots.values_mut() {
                        slot.clear();
                    }
                    self.cache_stats = self.image_cache.stats();
                    self.set_banner("Cleared local image cache", false);
                }
            }
            _ => {}
        }
    }

    fn enter_my_collection(&mut self) {
        self.refresh_cached_stats();
        self.screen = Screen::MyCollection;
        self.sync_collection_preview_slots();
    }

    fn open_card_details(
        &mut self,
        cards: Vec<InspectableCard>,
        idx: usize,
        return_screen: Screen,
    ) {
        self.detail_cards = cards;
        self.detail_idx = idx.min(self.detail_cards.len().saturating_sub(1));
        self.detail_return_screen = return_screen;
        self.screen = Screen::CardDetails;
        if let Some(item) = self.detail_cards.get(self.detail_idx) {
            let card = item.card.clone();
            self.request_card_into_slot("main", &card);
        }
    }

    fn open_selected_pack(&mut self) {
        let Some(pack_cfg) = self.pack_configs.get(self.pack_idx).cloned() else {
            return;
        };
        let collection_id = self.selected_collection_id.clone();
        let collection_name = self
            .active_collection()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| collection_id.clone());

        let pool = match self.store.list_cards_in_collection(&collection_id) {
            Ok(p) if !p.is_empty() => p,
            Ok(_) => {
                self.set_banner("Collection has no cached cards yet — sync first!", true);
                return;
            }
            Err(e) => {
                self.set_banner(format!("Failed reading card pool: {e}"), true);
                return;
            }
        };

        let mut rng = rng();
        let picks = match PackGenerator::generate_cards(&pack_cfg, &pool, &mut rng) {
            Ok(p) => p,
            Err(e) => {
                self.set_banner(format!("Pack generation error: {e}"), true);
                return;
            }
        };

        let pick_refs: Vec<(&str, Card)> = picks
            .iter()
            .map(|(lbl, c)| (lbl.as_str(), c.clone()))
            .collect();
        let pulled = match self.store.record_pack_opening(
            &pack_cfg,
            &self.selected_universe_id,
            &collection_id,
            &pick_refs,
        ) {
            Ok(p) => p,
            Err(e) => {
                self.set_banner(format!("Failed saving pack opening: {e}"), true);
                return;
            }
        };

        // Pre-fetch artwork for every card in the pack immediately in the background
        for item in &pulled {
            self.prefetch_card_image(&item.card);
        }
        if let Some(first) = pulled.first() {
            self.request_card_into_slot("main", &first.card);
        }

        let max_ticks = if self.settings.fast_animations { 4 } else { 10 };
        self.pack_opening = Some(PackOpeningState {
            result: PackOpeningResult {
                pack_config: pack_cfg,
                universe_id: self.selected_universe_id.clone(),
                collection_id,
                collection_name,
                cards: pulled,
            },
            phase: PackPhase::Ripping {
                ticks: 0,
                max_ticks,
            },
        });
        self.refresh_cached_stats();
        self.screen = Screen::PackOpening;
    }

    fn sync_collection_preview_slots(&mut self) {
        let filtered: Vec<Card> = self
            .filtered_owned_cards()
            .into_iter()
            .map(|o| o.card.clone())
            .collect();

        if filtered.is_empty() {
            if let Some(s) = self.image_slots.get_mut("preview") {
                s.clear();
            }
            for i in 0..4 {
                if let Some(s) = self.image_slots.get_mut(&format!("grid-{i}")) {
                    s.clear();
                }
            }
            return;
        }

        if self.grid_mode {
            let page_start = (self.owned_cursor / 4) * 4;
            for i in 0..4 {
                let slot_key = format!("grid-{i}");
                if let Some(card) = filtered.get(page_start + i).cloned() {
                    self.request_card_into_slot(&slot_key, &card);
                } else if let Some(s) = self.image_slots.get_mut(&slot_key) {
                    s.clear();
                }
            }
        } else if let Some(card) = filtered.get(self.owned_cursor).cloned() {
            self.request_card_into_slot("preview", &card);
        }
    }

    pub fn request_card_into_slot(&mut self, slot_key: &str, card: &Card) {
        let quality = self.settings.image_quality;
        if let Some(slot) = self.image_slots.get(slot_key)
            && slot.assigned_card_id.as_deref() == Some(&card.id)
            && (slot.has_ready_protocol() || slot.loading)
        {
            return;
        }

        if let Some(img) = self.image_cache.get_in_memory(&card.id, quality) {
            if let Some(slot) = self.image_slots.get_mut(slot_key) {
                slot.set_image(&card.id, &self.renderer, img);
            }
            return;
        }

        if let Some(slot) = self.image_slots.get_mut(slot_key) {
            slot.begin_loading(&card.id);
        }
        self.prefetch_card_image(card);
    }

    pub fn prefetch_card_image(&mut self, card: &Card) {
        let quality = self.settings.image_quality;
        let cache_key = ImageCache::cache_key(&card.id, quality);
        if self.image_cache.get_in_memory(&card.id, quality).is_some()
            || self.in_flight_images.contains(&cache_key)
        {
            return;
        }
        self.in_flight_images.insert(cache_key);

        let cache = self.image_cache.clone();
        let card_clone = card.clone();
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            let res = cache.load_card_image(&card_clone, quality).await;
            let _ = tx.send(AsyncMessage::ImageLoaded {
                card_id: card_clone.id,
                quality,
                result: res,
            });
        });
    }

    fn spawn_sync_collections_list(&mut self) {
        if self.syncing_collections_list {
            return;
        }
        let Some(u) = self.active_universe().cloned() else {
            return;
        };
        let Ok(provider) = self.providers.get(&u.provider_id) else {
            self.set_banner(format!("No provider registered for {}", u.name), true);
            return;
        };
        self.syncing_collections_list = true;
        self.set_banner("Refreshing Pokémon collections from TCGdex...", false);
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            let result = provider.fetch_collections().await;
            let _ = tx.send(AsyncMessage::CollectionsSynced { result });
        });
    }

    fn spawn_sync_collection_cards(&mut self, collection_id: String, open_pack_after: bool) {
        if self.syncing_collection_id.is_some() {
            return;
        }
        let Some(u) = self.active_universe().cloned() else {
            return;
        };
        let Ok(provider) = self.providers.get(&u.provider_id) else {
            self.set_banner(format!("No provider registered for {}", u.name), true);
            return;
        };

        self.syncing_collection_id = Some(collection_id.clone());
        let limit = self.settings.cards_per_sync;
        let tx = self.async_tx.clone();
        self.set_banner(
            format!("Syncing up to {limit} real cards for '{collection_id}'..."),
            false,
        );

        tokio::spawn(async move {
            let result = provider.fetch_collection_cards(&collection_id, limit).await;
            let _ = tx.send(AsyncMessage::CardsSynced {
                collection_id,
                open_pack_after,
                result,
            });
        });
    }
}
