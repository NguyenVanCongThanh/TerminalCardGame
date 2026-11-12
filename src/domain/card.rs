use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Normalized cross-universe rarity tiers used by pack generation and UI highlighting.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    HoloRare,
    UltraRare,
    SecretRare,
}

impl Rarity {
    pub fn label(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::HoloRare => "Holo Rare",
            Rarity::UltraRare => "Ultra Rare",
            Rarity::SecretRare => "Secret Rare",
        }
    }

    pub fn short_badge(self) -> &'static str {
        match self {
            Rarity::Common => "● C",
            Rarity::Uncommon => "◆ U",
            Rarity::Rare => "★ R",
            Rarity::HoloRare => "✦ HR",
            Rarity::UltraRare => "❖ UR",
            Rarity::SecretRare => "♛ SR",
        }
    }

    pub fn is_hit(self) -> bool {
        matches!(
            self,
            Rarity::HoloRare | Rarity::UltraRare | Rarity::SecretRare
        )
    }

    pub fn all() -> &'static [Rarity] {
        &[
            Rarity::Common,
            Rarity::Uncommon,
            Rarity::Rare,
            Rarity::HoloRare,
            Rarity::UltraRare,
            Rarity::SecretRare,
        ]
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "Common" => Rarity::Common,
            "Uncommon" => Rarity::Uncommon,
            "Rare" => Rarity::Rare,
            "HoloRare" => Rarity::HoloRare,
            "UltraRare" => Rarity::UltraRare,
            "SecretRare" => Rarity::SecretRare,
            _ => Rarity::Common,
        }
    }

    pub fn as_db_str(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::HoloRare => "HoloRare",
            Rarity::UltraRare => "UltraRare",
            Rarity::SecretRare => "SecretRare",
        }
    }
}

/// Theme-agnostic metadata attached to any collectible card.
/// Universe-specific fields (HP, attacks, mana, position, anime series, etc.) live in `attributes`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CardMetadata {
    /// Short classification subtitle (e.g., "Fire • Stage 2", "Spell Card", "Forward").
    pub subtitle: Option<String>,
    /// Artist, illustrator, studio, or photographer.
    pub creator: Option<String>,
    /// Lore or flavor description.
    pub flavor_text: Option<String>,
    /// Categorization tags (e.g., types, factions, traits).
    pub tags: Vec<String>,
    /// Flexible key-value attributes displayed dynamically in the Card Details panel.
    pub attributes: BTreeMap<String, String>,
    /// Structured abilities / moves / special effects rendered as bullet lines.
    pub highlights: Vec<String>,
}

/// Core collectible card entity decoupled from any specific API or universe schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Card {
    pub id: String,
    pub universe_id: String,
    pub collection_id: String,
    pub card_number: String,
    pub name: String,
    pub rarity: Rarity,
    /// Original provider rarity string (e.g., "Special illustration rare").
    pub raw_rarity: String,
    pub image_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub local_image_path: Option<String>,
    pub metadata: CardMetadata,
}

