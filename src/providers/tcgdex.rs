use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Semaphore;

use crate::domain::{Card, CardMetadata, Collection, Rarity};
use crate::error::{AppError, AppResult};
use crate::providers::{BoxFuture, CardDataProvider};

const TCGDEX_BASE_URL: &str = "https://api.tcgdex.net/v2/en";

/// Curated starter set IDs to surface first in the Pokémon universe.
const CURATED_SET_IDS: &[&str] = &["base1", "base2", "swsh1", "sv01"];

/// Public, zero-key card data provider powered by the open-source TCGdex API.
pub struct TcgDexProvider {
    client: Client,
    base_url: String,
}

impl TcgDexProvider {
    pub fn new() -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(12))
            .user_agent("terminal-card-game/0.1")
            .build()?;
        Ok(Self {
            client,
            base_url: TCGDEX_BASE_URL.to_string(),
        })
    }

    /// Picks a balanced sample of card IDs across the set so the resulting demo subset contains
    /// rare/holo cards (usually near the front of a set), uncommons (middle), and commons (tail).
    fn select_balanced_sample(cards: &[TcgDexCardBrief], limit: usize) -> Vec<String> {
        if cards.is_empty() || limit == 0 {
            return Vec::new();
        }
        if cards.len() <= limit {
            return cards.iter().map(|c| c.id.clone()).collect();
        }

        let mut selected_indices = Vec::with_capacity(limit);
        let mut seen = std::collections::HashSet::new();

        // Always include the first few chase/holo cards if present (e.g., Charizard #4 in base1)
        let head_count = limit.min(6).min(cards.len());
        for idx in 0..head_count {
            if seen.insert(idx) {
                selected_indices.push(idx);
            }
        }

        // Distribute the remaining slots evenly across the rest of the set
        let remaining = limit.saturating_sub(selected_indices.len());
        if remaining > 0 && cards.len() > head_count {
            let span = cards.len() - head_count;
            for step in 0..remaining {
                let idx = head_count + (step * span) / remaining;
                let idx = idx.min(cards.len() - 1);
                if seen.insert(idx) {
                    selected_indices.push(idx);
                }
            }
        }

        // Fill any gaps sequentially if integer division collided
        for idx in 0..cards.len() {
            if selected_indices.len() >= limit {
                break;
            }
            if seen.insert(idx) {
                selected_indices.push(idx);
            }
        }

        selected_indices
            .into_iter()
            .map(|i| cards[i].id.clone())
            .collect()
    }

    fn normalize_rarity(raw: Option<&str>, variants: Option<&TcgDexVariants>) -> (Rarity, String) {
        let raw_str = raw.unwrap_or("Common").trim();
        let lower = raw_str.to_ascii_lowercase();
        let is_holo_only = variants
            .map(|v| v.holo && !v.normal)
            .unwrap_or(false);

        let normalized = if lower.contains("secret")
            || lower.contains("hyper")
            || lower.contains("special illustration")
            || lower.contains("crown")
            || lower.contains("gold")
            || lower.contains("rainbow")
        {
            Rarity::SecretRare
        } else if lower.contains("ultra")
            || lower.contains("illustration")
            || lower.contains("double rare")
            || lower.contains("full art")
            || lower.contains("vmax")
            || lower.contains("vstar")
            || lower.contains("shiny")
            || lower.contains("radiant")
            || lower.contains("amazing")
            || lower.contains("ace spec")
            || lower.contains("star")
        {
            Rarity::UltraRare
        } else if lower.contains("holo") || (lower == "rare" && is_holo_only) {
            Rarity::HoloRare
        } else if lower.contains("rare") || lower.contains("three diamond") {
            Rarity::Rare
        } else if lower.contains("uncommon") || lower.contains("two diamond") {
            Rarity::Uncommon
        } else {
            Rarity::Common
        };

        let display_raw = if raw_str.eq_ignore_ascii_case("rare") && is_holo_only {
            "Rare Holo".to_string()
        } else {
            raw_str.to_string()
        };

        (normalized, display_raw)
    }

    fn map_card(detail: TcgDexCardDetail, collection_id: &str) -> Card {
        let (rarity, raw_rarity) =
            Self::normalize_rarity(detail.rarity.as_deref(), detail.variants.as_ref());

        let (image_url, thumbnail_url) = match detail.image.as_deref() {
            Some(base) if !base.is_empty() => {
                let clean = base.trim_end_matches('/');
                (
                    Some(format!("{clean}/high.png")),
                    Some(format!("{clean}/low.png")),
                )
            }
            _ => (None, None),
        };

        let mut attributes = BTreeMap::new();
        if let Some(cat) = detail.category.as_deref() {
            attributes.insert("Category".to_string(), cat.to_string());
        }
        if let Some(hp) = detail.hp {
            attributes.insert("HP".to_string(), hp.to_string());
        }
        if let Some(stage) = detail.stage.as_deref() {
            attributes.insert("Stage".to_string(), stage.to_string());
        }
        if let Some(evolves_from) = detail.evolve_from.as_deref() {
            attributes.insert("Evolves From".to_string(), evolves_from.to_string());
        }
        if let Some(retreat) = detail.retreat {
            attributes.insert("Retreat Cost".to_string(), retreat.to_string());
        }
        if let Some(pricing) = detail.pricing.as_ref() {
            if let Some(cm) = pricing.cardmarket.as_ref() {
                if let Some(trend) = cm.trend.or(cm.avg) {
                    attributes.insert("Market Trend".to_string(), format!("€{trend:.2}"));
                }
            } else if let Some(tp) = pricing.tcgplayer.as_ref()
                && let Some(price) = tp
                    .holofoil
                    .as_ref()
                    .and_then(|p| p.market_price)
                    .or_else(|| tp.normal.as_ref().and_then(|p| p.market_price))
            {
                attributes.insert("Market Price".to_string(), format!("${price:.2}"));
            }
        }

        let tags = detail.types.unwrap_or_default();
        let subtitle = match (tags.first(), detail.stage.as_deref(), detail.category.as_deref()) {
            (Some(t), Some(s), _) => Some(format!("{t} • {s}")),
            (Some(t), None, Some(c)) => Some(format!("{t} • {c}")),
            (None, _, Some(c)) => Some(c.to_string()),
            _ => None,
        };

        let mut highlights = Vec::new();
        if let Some(abilities) = detail.abilities {
            for ab in abilities {
                let kind = ab.kind.unwrap_or_else(|| "Ability".to_string());
                let effect = ab.effect.unwrap_or_default();
                highlights.push(format!("[{kind}] {}: {effect}", ab.name));
            }
        }
        if let Some(attacks) = detail.attacks {
            for atk in attacks {
                let dmg = atk
                    .damage
                    .map(|v| match v {
                        serde_json::Value::Number(n) => format!(" ({n} dmg)"),
                        serde_json::Value::String(s) if !s.is_empty() => format!(" ({s} dmg)"),
                        _ => String::new(),
                    })
                    .unwrap_or_default();
                let cost = atk
                    .cost
                    .filter(|c| !c.is_empty())
                    .map(|c| format!("[{}] ", c.join("/")))
                    .unwrap_or_default();
                let effect = atk.effect.unwrap_or_default();
                if effect.is_empty() {
                    highlights.push(format!("{cost}{}{dmg}", atk.name));
                } else {
                    highlights.push(format!("{cost}{}{dmg} — {effect}", atk.name));
                }
            }
        }

        Card {
            id: detail.id,
            universe_id: "pokemon".to_string(),
            collection_id: collection_id.to_string(),
            card_number: detail.local_id,
            name: detail.name,
            rarity,
            raw_rarity,
            image_url,
            thumbnail_url,
            local_image_path: None,
            metadata: CardMetadata {
                subtitle,
                creator: detail.illustrator,
                flavor_text: detail.description,
                tags,
                attributes,
                highlights,
            },
        }
    }

    async fn fetch_single_set(&self, set_id: &str) -> AppResult<TcgDexSetDetail> {
        let url = format!("{}/sets/{}", self.base_url, set_id);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Provider(format!(
                "TCGdex returned HTTP {} for set '{set_id}'",
                resp.status()
            )));
        }
        Ok(resp.json::<TcgDexSetDetail>().await?)
    }
}

impl CardDataProvider for TcgDexProvider {
    fn id(&self) -> &str {
        "tcgdex-pokemon"
    }

    fn universe_id(&self) -> &str {
        "pokemon"
    }

    fn fetch_collections(&self) -> BoxFuture<'_, AppResult<Vec<Collection>>> {
        Box::pin(async move {
            let mut results = Vec::new();
            for set_id in CURATED_SET_IDS {
                match self.fetch_single_set(set_id).await {
                    Ok(set) => {
                        let series = set
                            .serie
                            .as_ref()
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| "Pokémon".to_string());
                        let total = set
                            .card_count
                            .as_ref()
                            .map(|c| c.total.max(c.official))
                            .unwrap_or(set.cards.len() as u32);
                        let logo_url = set
                            .logo
                            .as_deref()
                            .filter(|l| !l.is_empty())
                            .map(|l| format!("{}.png", l.trim_end_matches('/')));

                        results.push(Collection {
                            id: set.id.clone(),
                            universe_id: "pokemon".to_string(),
                            name: set.name,
                            series: series.clone(),
                            release_date: set.release_date,
                            total_cards_in_set: total,
                            logo_url,
                            description: format!(
                                "{series} series expansion synced live from TCGdex ({total} cards in full set)."
                            ),
                            synced_at: None,
                        });
                    }
                    Err(err) => {
                        // Fail only if all sets fail; otherwise return partial successes
                        if results.is_empty() && *set_id == *CURATED_SET_IDS.last().unwrap() {
                            return Err(err);
                        }
                    }
                }
            }
            if results.is_empty() {
                return Err(AppError::Provider(
                    "Unable to fetch any Pokémon collections from TCGdex".to_string(),
                ));
            }
            Ok(results)
        })
    }

    fn fetch_collection_cards(
        &self,
        collection_id: &str,
        limit: usize,
    ) -> BoxFuture<'_, AppResult<Vec<Card>>> {
        let collection_id = collection_id.to_string();
        Box::pin(async move {
            let set = self.fetch_single_set(&collection_id).await?;
            let sample_ids = Self::select_balanced_sample(&set.cards, limit);
            if sample_ids.is_empty() {
                return Err(AppError::Provider(format!(
                    "Set '{collection_id}' returned zero cards"
                )));
            }

            let semaphore = Arc::new(Semaphore::new(8));
            let mut join_set = tokio::task::JoinSet::new();

            for card_id in sample_ids {
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|e| AppError::Provider(e.to_string()))?;
                let client = self.client.clone();
                let base_url = self.base_url.clone();
                let cid = collection_id.clone();

                join_set.spawn(async move {
                    let _permit = permit;
                    let url = format!("{base_url}/cards/{card_id}");
                    let resp = client.get(&url).send().await?;
                    if !resp.status().is_success() {
                        return Err(AppError::Provider(format!(
                            "TCGdex HTTP {} for card '{card_id}'",
                            resp.status()
                        )));
                    }
                    let detail = resp.json::<TcgDexCardDetail>().await?;
                    Ok::<Card, AppError>(Self::map_card(detail, &cid))
                });
            }

            let mut cards = Vec::new();
            let mut last_err = None;
            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(Ok(card)) => cards.push(card),
                    Ok(Err(e)) => last_err = Some(e),
                    Err(e) => last_err = Some(AppError::Provider(e.to_string())),
                }
            }

            if cards.is_empty() {
                return Err(last_err.unwrap_or_else(|| {
                    AppError::Provider("Failed to fetch any cards from TCGdex".to_string())
                }));
            }

            cards.sort_by(|a, b| {
                let na = a.card_number.parse::<u32>().unwrap_or(u32::MAX);
                let nb = b.card_number.parse::<u32>().unwrap_or(u32::MAX);
                na.cmp(&nb).then_with(|| a.card_number.cmp(&b.card_number))
            });

            Ok(cards)
        })
    }
}

#[derive(Debug, Deserialize)]
struct TcgDexCardBrief {
    id: String,
    #[serde(rename = "localId")]
    _local_id: String,
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Debug, Deserialize)]
struct TcgDexSerie {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TcgDexCardCount {
    #[serde(default)]
    official: u32,
    #[serde(default)]
    total: u32,
}

#[derive(Debug, Deserialize)]
struct TcgDexSetDetail {
    id: String,
    name: String,
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
    logo: Option<String>,
    serie: Option<TcgDexSerie>,
    #[serde(rename = "cardCount")]
    card_count: Option<TcgDexCardCount>,
    #[serde(default)]
    cards: Vec<TcgDexCardBrief>,
}

#[derive(Debug, Deserialize)]
struct TcgDexVariants {
    #[serde(default)]
    holo: bool,
    #[serde(default)]
    normal: bool,
}

#[derive(Debug, Deserialize)]
struct TcgDexAbility {
    #[serde(rename = "type")]
    kind: Option<String>,
    name: String,
    effect: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TcgDexAttack {
    cost: Option<Vec<String>>,
    name: String,
    effect: Option<String>,
    damage: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TcgDexCardmarket {
    avg: Option<f64>,
    trend: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TcgDexTcgplayerVariant {
    #[serde(rename = "marketPrice")]
    market_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TcgDexTcgplayer {
    holofoil: Option<TcgDexTcgplayerVariant>,
    normal: Option<TcgDexTcgplayerVariant>,
}

#[derive(Debug, Deserialize)]
struct TcgDexPricing {
    cardmarket: Option<TcgDexCardmarket>,
    tcgplayer: Option<TcgDexTcgplayer>,
}

#[derive(Debug, Deserialize)]
struct TcgDexCardDetail {
    id: String,
    #[serde(rename = "localId")]
    local_id: String,
    name: String,
    category: Option<String>,
    illustrator: Option<String>,
    image: Option<String>,
    rarity: Option<String>,
    variants: Option<TcgDexVariants>,
    hp: Option<u32>,
    types: Option<Vec<String>>,
    #[serde(rename = "evolveFrom")]
    evolve_from: Option<String>,
    description: Option<String>,
    stage: Option<String>,
    abilities: Option<Vec<TcgDexAbility>>,
    attacks: Option<Vec<TcgDexAttack>>,
    retreat: Option<u32>,
    pricing: Option<TcgDexPricing>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_set_holo_and_non_holo_rares() {
        let holo = TcgDexVariants {
            holo: true,
            normal: false,
        };
        let non_holo = TcgDexVariants {
            holo: false,
            normal: true,
        };

        let (r1, raw1) = TcgDexProvider::normalize_rarity(Some("Rare"), Some(&holo));
        assert_eq!(r1, Rarity::HoloRare);
        assert_eq!(raw1, "Rare Holo");

        let (r2, raw2) = TcgDexProvider::normalize_rarity(Some("Rare"), Some(&non_holo));
        assert_eq!(r2, Rarity::Rare);
        assert_eq!(raw2, "Rare");

        let (r3, _) =
            TcgDexProvider::normalize_rarity(Some("Special illustration rare"), Some(&holo));
        assert_eq!(r3, Rarity::SecretRare);
    }

    #[test]
    fn balanced_sample_includes_head_and_spread() {
        let cards: Vec<TcgDexCardBrief> = (1..=100)
            .map(|i| TcgDexCardBrief {
                id: format!("base1-{i}"),
                _local_id: i.to_string(),
                _name: format!("Card {i}"),
            })
            .collect();
        let sample = TcgDexProvider::select_balanced_sample(&cards, 20);
        assert_eq!(sample.len(), 20);
        assert_eq!(sample[0], "base1-1");
        assert_eq!(sample[3], "base1-4");
        assert!(sample.iter().any(|id| id == "base1-54" || id == "base1-53"));
        assert!(sample.iter().any(|id| id == "base1-94"));
    }
}
