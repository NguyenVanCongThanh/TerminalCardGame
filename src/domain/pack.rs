use serde::{Deserialize, Serialize};

use super::card::{Card, Rarity};

/// A single slot inside a pack definition with weighted rarity probabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackSlot {
    pub label: String,
    /// `(Rarity, weight)` pairs determining the draw probability for this slot.
    pub rarity_weights: Vec<(Rarity, u32)>,
}

/// Configurable pack template reusable across collections and universes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub slots: Vec<PackSlot>,
}

impl PackConfig {
    pub fn card_count(&self) -> usize {
        self.slots.len()
    }

    /// Standard configurable pack templates.
    pub fn default_configs() -> Vec<Self> {
        vec![
            Self {
                id: "demo-5".to_string(),
                name: "Demo Pack (5 Cards)".to_string(),
                description: "Fast 5-card pack: 2 Commons, 1 Common/Uncommon, 1 Uncommon/Rare, and 1 Guaranteed Rare+ Hit Slot.".to_string(),
                slots: vec![
                    PackSlot {
                        label: "Base Slot 1".to_string(),
                        rarity_weights: vec![(Rarity::Common, 90), (Rarity::Uncommon, 10)],
                    },
                    PackSlot {
                        label: "Base Slot 2".to_string(),
                        rarity_weights: vec![(Rarity::Common, 85), (Rarity::Uncommon, 15)],
                    },
                    PackSlot {
                        label: "Flex Slot".to_string(),
                        rarity_weights: vec![(Rarity::Common, 35), (Rarity::Uncommon, 60), (Rarity::Rare, 5)],
                    },
                    PackSlot {
                        label: "Spotlight Slot".to_string(),
                        rarity_weights: vec![(Rarity::Uncommon, 65), (Rarity::Rare, 25), (Rarity::HoloRare, 10)],
                    },
                    PackSlot {
                        label: "Hit Slot".to_string(),
                        rarity_weights: vec![
                            (Rarity::Rare, 50),
                            (Rarity::HoloRare, 30),
                            (Rarity::UltraRare, 15),
                            (Rarity::SecretRare, 5),
                        ],
                    },
                ],
            },
            Self {
                id: "booster-10".to_string(),
                name: "Standard Booster (10 Cards)".to_string(),
                description: "Classic TCG booster: 5 Commons, 3 Uncommons, 1 Reverse/Wildcard, and 1 Guaranteed Rare+ Slot.".to_string(),
                slots: vec![
                    PackSlot { label: "Common 1".into(), rarity_weights: vec![(Rarity::Common, 100)] },
                    PackSlot { label: "Common 2".into(), rarity_weights: vec![(Rarity::Common, 100)] },
                    PackSlot { label: "Common 3".into(), rarity_weights: vec![(Rarity::Common, 100)] },
                    PackSlot { label: "Common 4".into(), rarity_weights: vec![(Rarity::Common, 100)] },
                    PackSlot { label: "Common 5".into(), rarity_weights: vec![(Rarity::Common, 100)] },
                    PackSlot { label: "Uncommon 1".into(), rarity_weights: vec![(Rarity::Uncommon, 100)] },
                    PackSlot { label: "Uncommon 2".into(), rarity_weights: vec![(Rarity::Uncommon, 100)] },
                    PackSlot { label: "Uncommon 3".into(), rarity_weights: vec![(Rarity::Uncommon, 100)] },
                    PackSlot {
                        label: "Wildcard".into(),
                        rarity_weights: vec![
                            (Rarity::Common, 30),
                            (Rarity::Uncommon, 45),
                            (Rarity::Rare, 18),
                            (Rarity::HoloRare, 7),
                        ],
                    },
                    PackSlot {
                        label: "Rare Hit".into(),
                        rarity_weights: vec![
                            (Rarity::Rare, 52),
                            (Rarity::HoloRare, 28),
                            (Rarity::UltraRare, 15),
                            (Rarity::SecretRare, 5),
                        ],
                    },
                ],
            },
            Self {
                id: "collector-5".to_string(),
                name: "Collector's Showcase (5 Cards)".to_string(),
                description: "High-rarity collector pack with boosted Holo, Ultra, and Secret Rare odds.".to_string(),
                slots: vec![
                    PackSlot { label: "Uncommon+".into(), rarity_weights: vec![(Rarity::Uncommon, 75), (Rarity::Rare, 25)] },
                    PackSlot { label: "Rare Slot".into(), rarity_weights: vec![(Rarity::Rare, 70), (Rarity::HoloRare, 30)] },
                    PackSlot { label: "Holo Chance".into(), rarity_weights: vec![(Rarity::Rare, 40), (Rarity::HoloRare, 45), (Rarity::UltraRare, 15)] },
                    PackSlot { label: "Ultra Chance".into(), rarity_weights: vec![(Rarity::HoloRare, 50), (Rarity::UltraRare, 38), (Rarity::SecretRare, 12)] },
                    PackSlot { label: "Chase Finale".into(), rarity_weights: vec![(Rarity::HoloRare, 30), (Rarity::UltraRare, 45), (Rarity::SecretRare, 25)] },
                ],
            },
        ]
    }
}

/// A single card pulled from an opened pack, annotated with whether it was newly discovered.
#[derive(Debug, Clone)]
pub struct PulledCard {
    pub slot_label: String,
    pub card: Card,
    pub is_new: bool,
    pub new_quantity: u32,
}

/// Complete output of opening a pack.
#[derive(Debug, Clone)]
pub struct PackOpeningResult {
    pub pack_config: PackConfig,
    pub universe_id: String,
    pub collection_id: String,
    pub collection_name: String,
    pub cards: Vec<PulledCard>,
}

