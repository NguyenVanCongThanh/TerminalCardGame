use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    ClearImageCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveModal {
    Confirm {
        title: String,
        message: String,
        action: ConfirmAction,
    },
    Help,
}

#[derive(Debug, Clone)]
pub struct StatusBanner {
    pub message: String,
    pub is_error: bool,
    pub ttl_ticks: u64,
}

/// Shared application services, cached domain data, image pipeline, and global overlays.
pub struct AppContext {
    pub running: bool,
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

    // Shared active selection & sync state
    pub selected_universe_id: String,
    pub selected_collection_id: String,
    pub syncing_collection_id: Option<String>,
    pub syncing_collections_list: bool,
    pub active_pack_idx: usize,

    pub modal: Option<ActiveModal>,
    pub banner: Option<StatusBanner>,
}

impl AppContext {
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
        for key in [
            "main", "preview", "grid-0", "grid-1", "grid-2", "grid-3", "grid-4", "grid-5",
        ] {
            image_slots.insert(key.to_string(), CardImageSlot::new(key, resize_tx.clone()));
        }

        let mut ctx = Self {
            running: true,
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
            selected_universe_id,
            selected_collection_id,
            syncing_collection_id: None,
            syncing_collections_list: false,
            active_pack_idx: 0,
            modal: None,
            banner: None,
        };

        ctx.refresh_cached_stats(None);
        Ok(ctx)
    }

    pub fn set_banner(&mut self, message: impl Into<String>, is_error: bool) {
        self.banner = Some(StatusBanner {
            message: message.into(),
            is_error,
            ttl_ticks: if is_error { 90 } else { 55 },
        });
    }

    pub fn tick_banner(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if let Some(ref mut b) = self.banner {
            if b.ttl_ticks > 0 {
                b.ttl_ticks -= 1;
            }
            if b.ttl_ticks == 0 {
                self.banner = None;
            }
        }
    }

    pub fn refresh_cached_stats(&mut self, collection_filter: Option<&str>) {
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
        self.refresh_owned_cards(collection_filter);
        if let Ok(hist) = self.store.list_pack_history(8) {
            self.pack_history = hist;
        }
        self.cache_stats = self.image_cache.stats();
    }

    pub fn refresh_selected_collection_rarities(&mut self) {
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

    pub fn refresh_owned_cards(&mut self, collection_filter: Option<&str>) {
        if let Ok(owned) = self.store.list_owned_cards(collection_filter) {
            self.owned_cards = owned;
        }
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

    pub fn execute_confirm_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::ClearImageCache => {
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
        }
    }

    /// Generate and persist a pack opening for `pack_idx`, returning the resulting `PackOpeningResult`.
    pub fn open_pack(&mut self, pack_idx: usize) -> Option<PackOpeningResult> {
        let pack_cfg = self.pack_configs.get(pack_idx).cloned()?;
        self.active_pack_idx = pack_idx;
        let collection_id = self.selected_collection_id.clone();
        let collection_name = self
            .active_collection()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| collection_id.clone());

        let pool = match self.store.list_cards_in_collection(&collection_id) {
            Ok(p) if !p.is_empty() => p,
            Ok(_) => {
                self.set_banner("Collection has no cached cards yet — sync first!", true);
                return None;
            }
            Err(e) => {
                self.set_banner(format!("Failed reading card pool: {e}"), true);
                return None;
            }
        };

        let mut rng = rng();
        let picks = match PackGenerator::generate_cards(&pack_cfg, &pool, &mut rng) {
            Ok(p) => p,
            Err(e) => {
                self.set_banner(format!("Pack generation error: {e}"), true);
                return None;
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
                return None;
            }
        };

        // Pre-fetch artwork for every card in the pack immediately in the background
        for item in &pulled {
            self.prefetch_card_image(&item.card);
        }
        if let Some(first) = pulled.first() {
            self.request_card_into_slot("main", &first.card);
        }

        self.refresh_cached_stats(None);
        Some(PackOpeningResult {
            pack_config: pack_cfg,
            universe_id: self.selected_universe_id.clone(),
            collection_id,
            collection_name,
            cards: pulled,
        })
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

    pub fn spawn_sync_collections_list(&mut self) {
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

    pub fn spawn_sync_collection_cards(&mut self, collection_id: String, open_pack_after: bool) {
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

    pub fn handle_async_message(
        &mut self,
        msg: AsyncMessage,
        current_screen: super::Screen,
        collection_filter: Option<&str>,
    ) -> Option<super::Screen> {
        match msg {
            AsyncMessage::CollectionsSynced { result } => {
                self.syncing_collections_list = false;
                match result {
                    Ok(cols) => {
                        if let Err(e) = self.store.upsert_collections(&cols) {
                            self.set_banner(format!("Failed saving sets: {e}"), true);
                        } else {
                            self.refresh_cached_stats(collection_filter);
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
                None
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
                            return None;
                        }
                        let _ = self
                            .store
                            .mark_collection_synced(&collection_id, &Utc::now().to_rfc3339());
                        self.refresh_cached_stats(collection_filter);
                        self.set_banner(
                            format!("Synced {count} cards for '{collection_id}'"),
                            false,
                        );
                        if open_pack_after && current_screen == super::Screen::CollectionSelect {
                            return Some(super::Screen::PackSelect);
                        }
                    }
                    Err(e) => {
                        let cached = self.store.count_cards_in_collection(&collection_id).unwrap_or(0);
                        if cached > 0 {
                            self.set_banner(
                                format!("Network error ({e}); using {cached} cached cards"),
                                true,
                            );
                            if open_pack_after && current_screen == super::Screen::CollectionSelect {
                                return Some(super::Screen::PackSelect);
                            }
                        } else {
                            self.set_banner(
                                format!("Sync failed for '{collection_id}': {e}"),
                                true,
                            );
                        }
                    }
                }
                None
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
                None
            }
            AsyncMessage::ImageResized(ev) => {
                if let Some(slot) = self.image_slots.get_mut(&ev.slot_key) {
                    slot.apply_resize(ev.response);
                }
                None
            }
        }
    }
}

