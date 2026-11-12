pub mod sqlite;

use crate::domain::{
    AppSettings, Card, Collection, CollectionProgress, OwnedCard, PackConfig, PackHistoryEntry,
    PulledCard, Universe,
};
use crate::error::AppResult;

/// Persistence contract for Universes and Collections.
pub trait CollectionRepository: Send + Sync {
    fn list_universes(&self) -> AppResult<Vec<Universe>>;
    fn upsert_universes(&self, universes: &[Universe]) -> AppResult<()>;
    fn list_collections(&self, universe_id: &str) -> AppResult<Vec<Collection>>;
    fn upsert_collections(&self, collections: &[Collection]) -> AppResult<()>;
    fn mark_collection_synced(&self, collection_id: &str, synced_at: &str) -> AppResult<()>;
}

/// Persistence contract for cached collectible cards.
pub trait CardRepository: Send + Sync {
    fn list_cards_in_collection(&self, collection_id: &str) -> AppResult<Vec<Card>>;
    fn list_cards_by_ids(&self, ids: &[String]) -> AppResult<Vec<Card>>;
    fn count_cards_in_collection(&self, collection_id: &str) -> AppResult<usize>;
    fn upsert_cards(&self, cards: &[Card]) -> AppResult<()>;
    fn update_local_image_path(&self, card_id: &str, path: &str) -> AppResult<()>;
    fn clear_local_image_paths(&self) -> AppResult<()>;
}

/// Persistence contract for player inventory, completion stats, and pack opening history.
pub trait InventoryRepository: Send + Sync {
    fn list_owned_cards(&self, collection_filter: Option<&str>) -> AppResult<Vec<OwnedCard>>;
    fn list_collection_progress(&self, universe_id: &str) -> AppResult<Vec<CollectionProgress>>;
    fn record_pack_opening(
        &self,
        pack_config: &PackConfig,
        universe_id: &str,
        collection_id: &str,
        cards: &[(&str, Card)],
    ) -> AppResult<Vec<PulledCard>>;
    fn list_pack_history(&self, limit: usize) -> AppResult<Vec<PackHistoryEntry>>;
}

/// Persistence contract for player preferences.
pub trait SettingsRepository: Send + Sync {
    fn load_settings(&self) -> AppResult<AppSettings>;
    fn save_settings(&self, settings: &AppSettings) -> AppResult<()>;
}

