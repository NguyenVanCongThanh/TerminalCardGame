use serde::{Deserialize, Serialize};

use super::card::Card;

/// An entry in the player's inventory combining card data with ownership counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedCard {
    pub card: Card,
    pub quantity: u32,
    pub first_acquired_at: String,
    pub last_acquired_at: String,
}

/// Summary of player completion within a specific collection.
#[derive(Debug, Clone, Default)]
pub struct CollectionProgress {
    pub collection_id: String,
    pub collection_name: String,
    pub unique_owned: usize,
    pub total_cached_cards: usize,
    pub total_copies_owned: u32,
    pub total_cards_in_set: u32,
}

impl CollectionProgress {
    pub fn cached_completion_pct(&self) -> f64 {
        if self.total_cached_cards == 0 {
            0.0
        } else {
            (self.unique_owned as f64 / self.total_cached_cards as f64) * 100.0
        }
    }
}

/// Persistent log of opened packs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackHistoryEntry {
    pub id: i64,
    pub pack_config_id: String,
    pub pack_name: String,
    pub universe_id: String,
    pub collection_id: String,
    pub opened_at: String,
    pub card_ids: Vec<String>,
}

/// User-configurable terminal image rendering mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtocolMode {
    Auto,
    Halfblocks,
    Kitty,
    Sixel,
    Iterm2,
}

impl ProtocolMode {
    pub fn label(self) -> &'static str {
        match self {
            ProtocolMode::Auto => "Auto-Detect (Recommended)",
            ProtocolMode::Halfblocks => "Unicode Half-Blocks (Universal Fallback)",
            ProtocolMode::Kitty => "Kitty Graphics Protocol",
            ProtocolMode::Sixel => "Sixel Bitmap Graphics",
            ProtocolMode::Iterm2 => "iTerm2 Inline Images",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ProtocolMode::Auto => ProtocolMode::Halfblocks,
            ProtocolMode::Halfblocks => ProtocolMode::Kitty,
            ProtocolMode::Kitty => ProtocolMode::Sixel,
            ProtocolMode::Sixel => ProtocolMode::Iterm2,
            ProtocolMode::Iterm2 => ProtocolMode::Auto,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ProtocolMode::Auto => ProtocolMode::Iterm2,
            ProtocolMode::Halfblocks => ProtocolMode::Auto,
            ProtocolMode::Kitty => ProtocolMode::Halfblocks,
            ProtocolMode::Sixel => ProtocolMode::Kitty,
            ProtocolMode::Iterm2 => ProtocolMode::Sixel,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ProtocolMode::Auto => "auto",
            ProtocolMode::Halfblocks => "halfblocks",
            ProtocolMode::Kitty => "kitty",
            ProtocolMode::Sixel => "sixel",
            ProtocolMode::Iterm2 => "iterm2",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "halfblocks" => ProtocolMode::Halfblocks,
            "kitty" => ProtocolMode::Kitty,
            "sixel" => ProtocolMode::Sixel,
            "iterm2" => ProtocolMode::Iterm2,
            _ => ProtocolMode::Auto,
        }
    }
}

/// Image resolution preference for downloaded card artwork.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageQuality {
    High,
    Low,
}

impl ImageQuality {
    pub fn label(self) -> &'static str {
        match self {
            ImageQuality::High => "High (600x825 PNG)",
            ImageQuality::Low => "Low / Fast (245x337 PNG)",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            ImageQuality::High => ImageQuality::Low,
            ImageQuality::Low => ImageQuality::High,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ImageQuality::High => "high",
            ImageQuality::Low => "low",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "low" => ImageQuality::Low,
            _ => ImageQuality::High,
        }
    }
}

/// Persisted player & UI settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub protocol_mode: ProtocolMode,
    pub image_quality: ImageQuality,
    pub fast_animations: bool,
    pub cards_per_sync: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            protocol_mode: ProtocolMode::Auto,
            image_quality: ImageQuality::High,
            fast_animations: false,
            cards_per_sync: 28,
        }
    }
}
