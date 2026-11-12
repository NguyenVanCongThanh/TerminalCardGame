use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{
    AppSettings, Card, CardMetadata, Collection, CollectionProgress, ImageQuality, OwnedCard,
    PackConfig, PackHistoryEntry, ProtocolMode, PulledCard, Rarity, Universe,
};
use crate::error::AppResult;
use crate::storage::{
    CardRepository, CollectionRepository, InventoryRepository, SettingsRepository,
};

/// SQLite-backed persistence implementing all repository traits cleanly behind a single store.
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open(db_path: &Path) -> AppResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        store.seed_defaults()?;
        Ok(store)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        store.seed_defaults()?;
        Ok(store)
    }

    fn init_schema(&self) -> AppResult<()> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS universes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                subtitle TEXT NOT NULL,
                description TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                is_implemented INTEGER NOT NULL DEFAULT 0,
                badge TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                universe_id TEXT NOT NULL,
                name TEXT NOT NULL,
                series TEXT NOT NULL,
                release_date TEXT,
                total_cards_in_set INTEGER NOT NULL DEFAULT 0,
                logo_url TEXT,
                description TEXT NOT NULL DEFAULT '',
                synced_at TEXT,
                FOREIGN KEY (universe_id) REFERENCES universes(id)
            );

            CREATE TABLE IF NOT EXISTS cards (
                id TEXT PRIMARY KEY,
                universe_id TEXT NOT NULL,
                collection_id TEXT NOT NULL,
                card_number TEXT NOT NULL,
                name TEXT NOT NULL,
                rarity TEXT NOT NULL,
                raw_rarity TEXT NOT NULL,
                image_url TEXT,
                thumbnail_url TEXT,
                local_image_path TEXT,
                metadata_json TEXT NOT NULL,
                FOREIGN KEY (universe_id) REFERENCES universes(id),
                FOREIGN KEY (collection_id) REFERENCES collections(id)
            );

            CREATE INDEX IF NOT EXISTS idx_cards_collection ON cards(collection_id);

            CREATE TABLE IF NOT EXISTS inventory (
                card_id TEXT PRIMARY KEY,
                quantity INTEGER NOT NULL DEFAULT 1,
                first_acquired_at TEXT NOT NULL,
                last_acquired_at TEXT NOT NULL,
                FOREIGN KEY (card_id) REFERENCES cards(id)
            );

            CREATE TABLE IF NOT EXISTS pack_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pack_config_id TEXT NOT NULL,
                pack_name TEXT NOT NULL,
                universe_id TEXT NOT NULL,
                collection_id TEXT NOT NULL,
                opened_at TEXT NOT NULL,
                card_ids_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn seed_defaults(&self) -> AppResult<()> {
        self.upsert_universes(&Universe::default_universes())?;

        // Seed starter Pokémon collections only if empty so user-synced metadata is preserved.
        let existing = self.list_collections("pokemon")?;
        if existing.is_empty() {
            self.upsert_collections(&Collection::starter_pokemon_collections())?;
        }
        Ok(())
    }

    fn map_card_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Card> {
        let rarity_str: String = row.get(5)?;
        let meta_json: String = row.get(10)?;
        let metadata: CardMetadata = serde_json::from_str(&meta_json).unwrap_or_default();

        Ok(Card {
            id: row.get(0)?,
            universe_id: row.get(1)?,
            collection_id: row.get(2)?,
            card_number: row.get(3)?,
            name: row.get(4)?,
            rarity: Rarity::from_db_str(&rarity_str),
            raw_rarity: row.get(6)?,
            image_url: row.get(7)?,
            thumbnail_url: row.get(8)?,
            local_image_path: row.get(9)?,
            metadata,
        })
    }
}

impl CollectionRepository for SqliteStore {
    fn list_universes(&self) -> AppResult<Vec<Universe>> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, name, subtitle, description, provider_id, is_implemented, badge
             FROM universes ORDER BY sort_order ASC, name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let implemented: i64 = row.get(5)?;
            Ok(Universe {
                id: row.get(0)?,
                name: row.get(1)?,
                subtitle: row.get(2)?,
                description: row.get(3)?,
                provider_id: row.get(4)?,
                is_implemented: implemented != 0,
                badge: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn upsert_universes(&self, universes: &[Universe]) -> AppResult<()> {
        let mut conn = self.conn.lock().expect("sqlite mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO universes (id, name, subtitle, description, provider_id, is_implemented, badge, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   subtitle = excluded.subtitle,
                   description = excluded.description,
                   provider_id = excluded.provider_id,
                   is_implemented = excluded.is_implemented,
                   badge = excluded.badge,
                   sort_order = excluded.sort_order",
            )?;
            for (idx, u) in universes.iter().enumerate() {
                stmt.execute(params![
                    u.id,
                    u.name,
                    u.subtitle,
                    u.description,
                    u.provider_id,
                    if u.is_implemented { 1 } else { 0 },
                    u.badge,
                    idx as i64,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn list_collections(&self, universe_id: &str) -> AppResult<Vec<Collection>> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, universe_id, name, series, release_date, total_cards_in_set, logo_url, description, synced_at
             FROM collections
             WHERE universe_id = ?1
             ORDER BY COALESCE(release_date, '9999-99-99') ASC, name ASC",
        )?;
        let rows = stmt.query_map([universe_id], |row| {
            Ok(Collection {
                id: row.get(0)?,
                universe_id: row.get(1)?,
                name: row.get(2)?,
                series: row.get(3)?,
                release_date: row.get(4)?,
                total_cards_in_set: row.get(5)?,
                logo_url: row.get(6)?,
                description: row.get(7)?,
                synced_at: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn upsert_collections(&self, collections: &[Collection]) -> AppResult<()> {
        let mut conn = self.conn.lock().expect("sqlite mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO collections (id, universe_id, name, series, release_date, total_cards_in_set, logo_url, description, synced_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   series = excluded.series,
                   release_date = COALESCE(excluded.release_date, collections.release_date),
                   total_cards_in_set = CASE WHEN excluded.total_cards_in_set > 0 THEN excluded.total_cards_in_set ELSE collections.total_cards_in_set END,
                   logo_url = COALESCE(excluded.logo_url, collections.logo_url),
                   description = CASE WHEN excluded.description != '' THEN excluded.description ELSE collections.description END,
                   synced_at = COALESCE(excluded.synced_at, collections.synced_at)",
            )?;
            for c in collections {
                stmt.execute(params![
                    c.id,
                    c.universe_id,
                    c.name,
                    c.series,
                    c.release_date,
                    c.total_cards_in_set,
                    c.logo_url,
                    c.description,
                    c.synced_at,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn mark_collection_synced(&self, collection_id: &str, synced_at: &str) -> AppResult<()> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        conn.execute(
            "UPDATE collections SET synced_at = ?1 WHERE id = ?2",
            params![synced_at, collection_id],
        )?;
        Ok(())
    }
}

impl CardRepository for SqliteStore {
    fn list_cards_in_collection(&self, collection_id: &str) -> AppResult<Vec<Card>> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, universe_id, collection_id, card_number, name, rarity, raw_rarity,
                    image_url, thumbnail_url, local_image_path, metadata_json
             FROM cards
             WHERE collection_id = ?1",
        )?;
        let rows = stmt.query_map([collection_id], Self::map_card_row)?;
        let mut cards = rows.collect::<Result<Vec<_>, _>>()?;
        cards.sort_by(|a, b| {
            let na = a.card_number.parse::<u32>().unwrap_or(u32::MAX);
            let nb = b.card_number.parse::<u32>().unwrap_or(u32::MAX);
            na.cmp(&nb).then_with(|| a.card_number.cmp(&b.card_number))
        });
        Ok(cards)
    }

    fn count_cards_in_collection(&self, collection_id: &str) -> AppResult<usize> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM cards WHERE collection_id = ?1",
            [collection_id],
            |r| r.get(0),
        )?;
        Ok(count.max(0) as usize)
    }

    fn upsert_cards(&self, cards: &[Card]) -> AppResult<()> {
        let mut conn = self.conn.lock().expect("sqlite mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO cards (id, universe_id, collection_id, card_number, name, rarity, raw_rarity,
                                    image_url, thumbnail_url, local_image_path, metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                   card_number = excluded.card_number,
                   name = excluded.name,
                   rarity = excluded.rarity,
                   raw_rarity = excluded.raw_rarity,
                   image_url = COALESCE(excluded.image_url, cards.image_url),
                   thumbnail_url = COALESCE(excluded.thumbnail_url, cards.thumbnail_url),
                   local_image_path = COALESCE(excluded.local_image_path, cards.local_image_path),
                   metadata_json = excluded.metadata_json",
            )?;
            for card in cards {
                let meta_json = serde_json::to_string(&card.metadata)?;
                stmt.execute(params![
                    card.id,
                    card.universe_id,
                    card.collection_id,
                    card.card_number,
                    card.name,
                    card.rarity.as_db_str(),
                    card.raw_rarity,
                    card.image_url,
                    card.thumbnail_url,
                    card.local_image_path,
                    meta_json,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn update_local_image_path(&self, card_id: &str, path: &str) -> AppResult<()> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        conn.execute(
            "UPDATE cards SET local_image_path = ?1 WHERE id = ?2",
            params![path, card_id],
        )?;
        Ok(())
    }

    fn clear_local_image_paths(&self) -> AppResult<()> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        conn.execute("UPDATE cards SET local_image_path = NULL", [])?;
        Ok(())
    }
}

impl InventoryRepository for SqliteStore {
    fn list_owned_cards(&self, collection_filter: Option<&str>) -> AppResult<Vec<OwnedCard>> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let sql = match collection_filter {
            Some(_) => {
                "SELECT c.id, c.universe_id, c.collection_id, c.card_number, c.name, c.rarity, c.raw_rarity,
                        c.image_url, c.thumbnail_url, c.local_image_path, c.metadata_json,
                        i.quantity, i.first_acquired_at, i.last_acquired_at
                 FROM inventory i
                 JOIN cards c ON c.id = i.card_id
                 WHERE c.collection_id = ?1
                 ORDER BY i.last_acquired_at DESC, c.name ASC"
            }
            None => {
                "SELECT c.id, c.universe_id, c.collection_id, c.card_number, c.name, c.rarity, c.raw_rarity,
                        c.image_url, c.thumbnail_url, c.local_image_path, c.metadata_json,
                        i.quantity, i.first_acquired_at, i.last_acquired_at
                 FROM inventory i
                 JOIN cards c ON c.id = i.card_id
                 ORDER BY i.last_acquired_at DESC, c.name ASC"
            }
        };

        let mut stmt = conn.prepare(sql)?;
        let mapper = |row: &rusqlite::Row<'_>| -> rusqlite::Result<OwnedCard> {
            let card = Self::map_card_row(row)?;
            Ok(OwnedCard {
                card,
                quantity: row.get(11)?,
                first_acquired_at: row.get(12)?,
                last_acquired_at: row.get(13)?,
            })
        };

        let owned = match collection_filter {
            Some(cid) => stmt.query_map([cid], mapper)?.collect::<Result<Vec<_>, _>>()?,
            None => stmt.query_map([], mapper)?.collect::<Result<Vec<_>, _>>()?,
        };
        Ok(owned)
    }

    fn list_collection_progress(&self, universe_id: &str) -> AppResult<Vec<CollectionProgress>> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT col.id, col.name, col.total_cards_in_set,
                    COUNT(DISTINCT c.id) AS total_cached,
                    COUNT(DISTINCT CASE WHEN i.quantity > 0 THEN c.id END) AS unique_owned,
                    COALESCE(SUM(i.quantity), 0) AS total_copies
             FROM collections col
             LEFT JOIN cards c ON c.collection_id = col.id
             LEFT JOIN inventory i ON i.card_id = c.id
             WHERE col.universe_id = ?1
             GROUP BY col.id, col.name, col.total_cards_in_set
             ORDER BY COALESCE(col.release_date, '9999-99-99') ASC, col.name ASC",
        )?;
        let rows = stmt.query_map([universe_id], |row| {
            let total_cached: i64 = row.get(3)?;
            let unique_owned: i64 = row.get(4)?;
            let total_copies: i64 = row.get(5)?;
            Ok(CollectionProgress {
                collection_id: row.get(0)?,
                collection_name: row.get(1)?,
                total_cards_in_set: row.get(2)?,
                total_cached_cards: total_cached.max(0) as usize,
                unique_owned: unique_owned.max(0) as usize,
                total_copies_owned: total_copies.max(0) as u32,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn record_pack_opening(
        &self,
        pack_config: &PackConfig,
        universe_id: &str,
        collection_id: &str,
        cards: &[(&str, Card)],
    ) -> AppResult<Vec<PulledCard>> {
        let mut conn = self.conn.lock().expect("sqlite mutex poisoned");
        let tx = conn.transaction()?;
        let now = Utc::now().to_rfc3339();
        let mut pulled = Vec::with_capacity(cards.len());
        let mut card_ids = Vec::with_capacity(cards.len());

        for (slot_label, card) in cards {
            card_ids.push(card.id.clone());

            let existing_qty: Option<u32> = tx
                .query_row(
                    "SELECT quantity FROM inventory WHERE card_id = ?1",
                    [&card.id],
                    |r| r.get(0),
                )
                .optional()?;

            let (is_new, new_quantity) = match existing_qty {
                Some(qty) => {
                    let next = qty.saturating_add(1);
                    tx.execute(
                        "UPDATE inventory SET quantity = ?1, last_acquired_at = ?2 WHERE card_id = ?3",
                        params![next, &now, &card.id],
                    )?;
                    (false, next)
                }
                None => {
                    tx.execute(
                        "INSERT INTO inventory (card_id, quantity, first_acquired_at, last_acquired_at)
                         VALUES (?1, 1, ?2, ?2)",
                        params![&card.id, &now],
                    )?;
                    (true, 1)
                }
            };

            pulled.push(PulledCard {
                slot_label: (*slot_label).to_string(),
                card: card.clone(),
                is_new,
                new_quantity,
            });
        }

        let card_ids_json = serde_json::to_string(&card_ids)?;
        tx.execute(
            "INSERT INTO pack_history (pack_config_id, pack_name, universe_id, collection_id, opened_at, card_ids_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pack_config.id,
                pack_config.name,
                universe_id,
                collection_id,
                now,
                card_ids_json,
            ],
        )?;

        tx.commit()?;
        Ok(pulled)
    }

    fn list_pack_history(&self, limit: usize) -> AppResult<Vec<PackHistoryEntry>> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, pack_config_id, pack_name, universe_id, collection_id, opened_at, card_ids_json
             FROM pack_history
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            let json_ids: String = row.get(6)?;
            let card_ids: Vec<String> = serde_json::from_str(&json_ids).unwrap_or_default();
            Ok(PackHistoryEntry {
                id: row.get(0)?,
                pack_config_id: row.get(1)?,
                pack_name: row.get(2)?,
                universe_id: row.get(3)?,
                collection_id: row.get(4)?,
                opened_at: row.get(5)?,
                card_ids,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

impl SettingsRepository for SqliteStore {
    fn load_settings(&self) -> AppResult<AppSettings> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut settings = AppSettings::default();

        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            Ok((k, v))
        })?;

        for pair in rows {
            let (k, v) = pair?;
            match k.as_str() {
                "protocol_mode" => settings.protocol_mode = ProtocolMode::from_db_str(&v),
                "image_quality" => settings.image_quality = ImageQuality::from_db_str(&v),
                "fast_animations" => settings.fast_animations = v == "true",
                "cards_per_sync" => {
                    if let Ok(n) = v.parse::<usize>() {
                        settings.cards_per_sync = n.clamp(10, 60);
                    }
                }
                _ => {}
            }
        }
        Ok(settings)
    }

    fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let mut conn = self.conn.lock().expect("sqlite mutex poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            )?;
            stmt.execute(params!["protocol_mode", settings.protocol_mode.as_str()])?;
            stmt.execute(params!["image_quality", settings.image_quality.as_str()])?;
            stmt.execute(params![
                "fast_animations",
                if settings.fast_animations { "true" } else { "false" }
            ])?;
            stmt.execute(params!["cards_per_sync", settings.cards_per_sync.to_string()])?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardMetadata;

    fn sample_card(id: &str, num: &str, rarity: Rarity) -> Card {
        Card {
            id: id.to_string(),
            universe_id: "pokemon".to_string(),
            collection_id: "base1".to_string(),
            card_number: num.to_string(),
            name: format!("Card {id}"),
            rarity,
            raw_rarity: rarity.label().to_string(),
            image_url: None,
            thumbnail_url: None,
            local_image_path: None,
            metadata: CardMetadata::default(),
        }
    }

    #[test]
    fn persists_cards_inventory_and_settings() {
        let store = SqliteStore::open_in_memory().unwrap();
        let cards = vec![
            sample_card("base1-4", "4", Rarity::HoloRare),
            sample_card("base1-58", "58", Rarity::Common),
        ];
        store.upsert_cards(&cards).unwrap();
        assert_eq!(store.count_cards_in_collection("base1").unwrap(), 2);

        let pack = PackConfig::default_configs().remove(0);
        let first_pull = store
            .record_pack_opening(
                &pack,
                "pokemon",
                "base1",
                &[
                    ("Base", cards[1].clone()),
                    ("Hit", cards[0].clone()),
                ],
            )
            .unwrap();
        assert!(first_pull[0].is_new);
        assert_eq!(first_pull[0].new_quantity, 1);

        let second_pull = store
            .record_pack_opening(&pack, "pokemon", "base1", &[("Hit", cards[0].clone())])
            .unwrap();
        assert!(!second_pull[0].is_new);
        assert_eq!(second_pull[0].new_quantity, 2);

        let owned = store.list_owned_cards(Some("base1")).unwrap();
        assert_eq!(owned.len(), 2);
        let zard = owned.iter().find(|o| o.card.id == "base1-4").unwrap();
        assert_eq!(zard.quantity, 2);

        let history = store.list_pack_history(10).unwrap();
        assert_eq!(history.len(), 2);

        let s = AppSettings {
            protocol_mode: ProtocolMode::Halfblocks,
            fast_animations: true,
            ..Default::default()
        };
        store.save_settings(&s).unwrap();
        assert_eq!(store.load_settings().unwrap(), s);
    }
}
