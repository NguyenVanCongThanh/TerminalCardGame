pub mod tcgdex;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::domain::{Card, Collection};
use crate::error::{AppError, AppResult};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Abstraction over external APIs or local datasets that supply card collections and cards.
/// Keeping this trait free of provider-specific types allows introducing new universes
/// (Yu-Gi-Oh!, Anime, Football, custom JSON packs) without modifying core game logic.
pub trait CardDataProvider: Send + Sync {
    /// Unique identifier for this provider (e.g. `"tcgdex-pokemon"`).
    fn id(&self) -> &str;

    /// Universe identifier served by this provider (e.g. `"pokemon"`).
    fn universe_id(&self) -> &str;

    /// Fetch available collections/sets for the universe.
    fn fetch_collections(&self) -> BoxFuture<'_, AppResult<Vec<Collection>>>;

    /// Fetch a bounded subset of cards belonging to `collection_id`.
    fn fetch_collection_cards(
        &self,
        collection_id: &str,
        limit: usize,
    ) -> BoxFuture<'_, AppResult<Vec<Card>>>;
}

/// Registry mapping provider IDs to concrete `CardDataProvider` implementations.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn CardDataProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn CardDataProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    pub fn get(&self, provider_id: &str) -> AppResult<Arc<dyn CardDataProvider>> {
        self.providers.get(provider_id).cloned().ok_or_else(|| {
            AppError::Provider(format!(
                "No card data provider registered for '{provider_id}'"
            ))
        })
    }
}

