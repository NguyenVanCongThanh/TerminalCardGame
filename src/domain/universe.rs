use serde::{Deserialize, Serialize};

/// Represents an overarching trading card universe / theme (e.g., Pokémon, Yu-Gi-Oh!, Anime).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Universe {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub description: String,
    /// Identifier of the `CardDataProvider` responsible for syncing this universe.
    pub provider_id: String,
    /// Whether this universe is currently playable in the MVP.
    pub is_implemented: bool,
    /// Display badge/symbol shown in the UI.
    pub badge: String,
}

impl Universe {
    /// Returns the default MVP universe definitions.
    /// Future universes can be registered dynamically or loaded from storage/plugins.
    pub fn default_universes() -> Vec<Self> {
        vec![
            Self {
                id: "pokemon".to_string(),
                name: "Pokémon TCG".to_string(),
                subtitle: "Pocket Monsters Trading Card Game".to_string(),
                description: "Classic and modern Pokémon card sets powered by the open TCGdex API."
                    .to_string(),
                provider_id: "tcgdex-pokemon".to_string(),
                is_implemented: true,
                badge: "⚡".to_string(),
            },
            Self {
                id: "yugioh".to_string(),
                name: "Yu-Gi-Oh! (Planned)".to_string(),
                subtitle: "Duel Monsters Universe".to_string(),
                description: "Reserved universe slot demonstrating multi-provider extensibility."
                    .to_string(),
                provider_id: "unimplemented".to_string(),
                is_implemented: false,
                badge: "◈".to_string(),
            },
            Self {
                id: "anime".to_string(),
                name: "Anime Chronicles (Planned)".to_string(),
                subtitle: "Multi-Series Character Cards".to_string(),
                description: "Reserved universe slot for anime character & custom JSON collections."
                    .to_string(),
                provider_id: "unimplemented".to_string(),
                is_implemented: false,
                badge: "★".to_string(),
            },
        ]
    }
}

