use std::collections::HashSet;

use rand::Rng;

use crate::domain::{Card, PackConfig, PackSlot, Rarity};
use crate::error::{AppError, AppResult};

/// Configurable pack generator decoupled from any specific universe or API.
pub struct PackGenerator;

impl PackGenerator {
    /// Generates `(slot_label, Card)` picks from `pool` according to `config`.
    /// Prefers cards not yet selected in the same pack when multiple candidates exist,
    /// and falls back to the nearest available rarity tier if `pool` lacks a rolled rarity.
    pub fn generate_cards<R: Rng + ?Sized>(
        config: &PackConfig,
        pool: &[Card],
        rng: &mut R,
    ) -> AppResult<Vec<(String, Card)>> {
        if pool.is_empty() {
            return Err(AppError::PackGeneration(
                "Cannot open a pack from an empty collection pool".to_string(),
            ));
        }

        let mut picked_ids = HashSet::new();
        let mut results = Vec::with_capacity(config.slots.len());

        for slot in &config.slots {
            let rolled_rarity = Self::roll_slot_rarity(slot, rng);
            let card = Self::pick_card_for_rarity(pool, rolled_rarity, &picked_ids, rng);
            picked_ids.insert(card.id.clone());
            results.push((slot.label.clone(), card));
        }

        Ok(results)
    }

    fn roll_slot_rarity<R: Rng + ?Sized>(slot: &PackSlot, rng: &mut R) -> Rarity {
        let total_weight: u32 = slot.rarity_weights.iter().map(|(_, w)| *w).sum();
        if total_weight == 0 {
            return Rarity::Common;
        }
        let mut roll = rng.random_range(0..total_weight);
        for (rarity, weight) in &slot.rarity_weights {
            if roll < *weight {
                return *rarity;
            }
            roll -= *weight;
        }
        slot.rarity_weights
            .last()
            .map(|(r, _)| *r)
            .unwrap_or(Rarity::Common)
    }

    fn pick_card_for_rarity<R: Rng + ?Sized>(
        pool: &[Card],
        target: Rarity,
        already_picked: &HashSet<String>,
        rng: &mut R,
    ) -> Card {
        // Search order: exact target, then nearest lower/higher rarities by distance
        let mut rarities: Vec<Rarity> = Rarity::all().to_vec();
        rarities.sort_by_key(|r| {
            let dist = (*r as i32 - target as i32).abs();
            // Slight preference for falling downward if equidistant, unless target is Common
            let dir_bias = if *r <= target { 0 } else { 1 };
            (dist, dir_bias)
        });

        for rarity in rarities {
            let matching_fresh: Vec<&Card> = pool
                .iter()
                .filter(|c| c.rarity == rarity && !already_picked.contains(&c.id))
                .collect();
            if !matching_fresh.is_empty() {
                let idx = rng.random_range(0..matching_fresh.len());
                return matching_fresh[idx].clone();
            }

            let matching_any: Vec<&Card> = pool.iter().filter(|c| c.rarity == rarity).collect();
            if !matching_any.is_empty() {
                let idx = rng.random_range(0..matching_any.len());
                return matching_any[idx].clone();
            }
        }

        let idx = rng.random_range(0..pool.len());
        pool[idx].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardMetadata;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn card(id: &str, rarity: Rarity) -> Card {
        Card {
            id: id.to_string(),
            universe_id: "pokemon".to_string(),
            collection_id: "base1".to_string(),
            card_number: id.to_string(),
            name: format!("Test {id}"),
            rarity,
            raw_rarity: rarity.label().to_string(),
            image_url: None,
            thumbnail_url: None,
            local_image_path: None,
            metadata: CardMetadata::default(),
        }
    }

    #[test]
    fn generates_configured_count_and_respects_guaranteed_hit_slot() {
        let pool = vec![
            card("c1", Rarity::Common),
            card("c2", Rarity::Common),
            card("c3", Rarity::Common),
            card("u1", Rarity::Uncommon),
            card("u2", Rarity::Uncommon),
            card("r1", Rarity::Rare),
            card("h1", Rarity::HoloRare),
        ];
        let cfg = PackConfig::default_configs()
            .into_iter()
            .find(|p| p.id == "demo-5")
            .unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let generated = PackGenerator::generate_cards(&cfg, &pool, &mut rng).unwrap();
        assert_eq!(generated.len(), 5);

        let last_rarity = generated.last().unwrap().1.rarity;
        assert!(
            matches!(
                last_rarity,
                Rarity::Rare | Rarity::HoloRare | Rarity::UltraRare | Rarity::SecretRare
            ),
            "expected Rare+ in hit slot, got {last_rarity:?}"
        );
    }

    #[test]
    fn gracefully_falls_back_when_target_rarity_missing() {
        let pool = vec![card("only-common", Rarity::Common)];
        let cfg = PackConfig::default_configs()
            .into_iter()
            .find(|p| p.id == "collector-5")
            .unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        let generated = PackGenerator::generate_cards(&cfg, &pool, &mut rng).unwrap();
        assert_eq!(generated.len(), 5);
        assert!(generated.iter().all(|(_, c)| c.id == "only-common"));
    }

    #[test]
    fn errors_on_empty_pool() {
        let cfg = PackConfig::default_configs().remove(0);
        let mut rng = StdRng::seed_from_u64(1);
        assert!(PackGenerator::generate_cards(&cfg, &[], &mut rng).is_err());
    }
}

