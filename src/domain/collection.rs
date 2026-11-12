use serde::{Deserialize, Serialize};

/// Represents a card set / expansion belonging to a `Universe`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Collection {
    pub id: String,
    pub universe_id: String,
    pub name: String,
    pub series: String,
    pub release_date: Option<String>,
    pub total_cards_in_set: u32,
    pub logo_url: Option<String>,
    pub description: String,
    /// Timestamp (RFC3339) when card metadata was last synced locally.
    pub synced_at: Option<String>,
}

impl Collection {
    /// Curated starter collections for the Pokémon universe so users can immediately browse
    /// iconic eras even before fetching full card metadata.
    pub fn starter_pokemon_collections() -> Vec<Self> {
        vec![
            Self {
                id: "base1".to_string(),
                universe_id: "pokemon".to_string(),
                name: "Base Set".to_string(),
                series: "Base".to_string(),
                release_date: Some("1999-01-09".to_string()),
                total_cards_in_set: 102,
                logo_url: Some("https://assets.tcgdex.net/en/base/base1/logo.png".to_string()),
                description: "The iconic 1999 original Pokémon TCG expansion featuring Charizard, Blastoise, and Venusaur.".to_string(),
                synced_at: None,
            },
            Self {
                id: "base2".to_string(),
                universe_id: "pokemon".to_string(),
                name: "Jungle".to_string(),
                series: "Base".to_string(),
                release_date: Some("1999-06-16".to_string()),
                total_cards_in_set: 64,
                logo_url: Some("https://assets.tcgdex.net/en/base/base2/logo.png".to_string()),
                description: "Second classic Wizards of the Coast expansion introducing Snorlax, Scyther, and Eevee evolutions.".to_string(),
                synced_at: None,
            },
            Self {
                id: "swsh1".to_string(),
                universe_id: "pokemon".to_string(),
                name: "Sword & Shield".to_string(),
                series: "Sword & Shield".to_string(),
                release_date: Some("2020-02-07".to_string()),
                total_cards_in_set: 216,
                logo_url: Some("https://assets.tcgdex.net/en/swsh/swsh1/logo.png".to_string()),
                description: "Galar era debut introducing Pokémon V and VMAX mechanics.".to_string(),
                synced_at: None,
            },
            Self {
                id: "sv01".to_string(),
                universe_id: "pokemon".to_string(),
                name: "Scarlet & Violet".to_string(),
                series: "Scarlet & Violet".to_string(),
                release_date: Some("2023-03-31".to_string()),
                total_cards_in_set: 258,
                logo_url: Some("https://assets.tcgdex.net/en/sv/sv01/logo.png".to_string()),
                description: "Paldea era debut featuring Illustration Rares, Special Illustration Rares, and Pokémon ex.".to_string(),
                synced_at: None,
            },
        ]
    }
}

