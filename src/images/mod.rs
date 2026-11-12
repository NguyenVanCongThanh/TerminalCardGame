pub mod renderer;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use image::DynamicImage;
use reqwest::Client;

use crate::domain::{Card, ImageQuality};
use crate::error::{AppError, AppResult};

const MAX_MEMORY_IMAGES: usize = 64;

/// Statistics about the on-disk artwork cache.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub file_count: usize,
    pub total_bytes: u64,
}

/// Manages asynchronous image downloads, atomic disk caching, and bounded in-memory decoded images.
pub struct ImageCache {
    cache_dir: PathBuf,
    client: Client,
    memory: Mutex<MemoryImageLru>,
}

struct MemoryImageLru {
    map: HashMap<String, DynamicImage>,
    order: VecDeque<String>,
}

impl MemoryImageLru {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<DynamicImage> {
        self.map.get(key).cloned()
    }

    fn insert(&mut self, key: String, img: DynamicImage) {
        if !self.map.contains_key(&key) {
            if self.order.len() >= MAX_MEMORY_IMAGES
                && let Some(evicted) = self.order.pop_front()
            {
                self.map.remove(&evicted);
            }
            self.order.push_back(key.clone());
        }
        self.map.insert(key, img);
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

impl ImageCache {
    pub fn new(cache_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("terminal-card-game/0.1")
            .build()?;
        Ok(Self {
            cache_dir,
            client,
            memory: Mutex::new(MemoryImageLru::new()),
        })
    }

    pub fn cache_key(card_id: &str, quality: ImageQuality) -> String {
        format!("{}_{}", card_id, quality.as_str())
    }

    pub fn disk_path_for(&self, card: &Card, quality: ImageQuality) -> PathBuf {
        let safe_collection: String = card
            .collection_id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let safe_id: String = card
            .id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        self.cache_dir
            .join(safe_collection)
            .join(format!("{safe_id}_{}.png", quality.as_str()))
    }

    /// Returns a cached `DynamicImage` synchronously from memory if already decoded.
    pub fn get_in_memory(&self, card_id: &str, quality: ImageQuality) -> Option<DynamicImage> {
        let key = Self::cache_key(card_id, quality);
        self.memory.lock().expect("image cache mutex").get(&key)
    }

    /// Loads the card image from memory, local disk cache, or remote URL (caching to disk & memory).
    pub async fn load_card_image(
        self: &Arc<Self>,
        card: &Card,
        quality: ImageQuality,
    ) -> AppResult<(DynamicImage, PathBuf)> {
        let key = Self::cache_key(&card.id, quality);
        let expected_path = self.disk_path_for(card, quality);

        if let Some(img) = self.memory.lock().expect("image cache mutex").get(&key) {
            return Ok((img, expected_path));
        }

        // Check expected quality-specific path first, then any existing `card.local_image_path`
        let existing_disk = if expected_path.exists() {
            Some(expected_path.clone())
        } else if let Some(ref saved) = card.local_image_path {
            let p = PathBuf::from(saved);
            if p.exists() { Some(p) } else { None }
        } else {
            None
        };

        if let Some(disk_path) = existing_disk {
            let path_clone = disk_path.clone();
            let decoded = tokio::task::spawn_blocking(move || -> AppResult<DynamicImage> {
                let bytes = std::fs::read(&path_clone)?;
                Ok(image::load_from_memory(&bytes)?)
            })
            .await
            .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))??;

            self.memory
                .lock()
                .expect("image cache mutex")
                .insert(key, decoded.clone());
            return Ok((decoded, disk_path));
        }

        let target_url = match quality {
            ImageQuality::High => card.image_url.as_ref().or(card.thumbnail_url.as_ref()),
            ImageQuality::Low => card.thumbnail_url.as_ref().or(card.image_url.as_ref()),
        }
        .ok_or_else(|| AppError::Provider(format!("Card '{}' has no image URL", card.name)))?;

        let resp = self.client.get(target_url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Provider(format!(
                "Artwork download failed (HTTP {}) for {}",
                resp.status(),
                card.name
            )));
        }
        let bytes = resp.bytes().await?;

        let write_path = expected_path.clone();
        let bytes_vec = bytes.to_vec();
        let decoded = tokio::task::spawn_blocking(move || -> AppResult<DynamicImage> {
            let img = image::load_from_memory(&bytes_vec)?;
            if let Some(parent) = write_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tmp_path = write_path.with_extension("png.tmp");
            std::fs::write(&tmp_path, &bytes_vec)?;
            std::fs::rename(&tmp_path, &write_path)?;
            Ok(img)
        })
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))??;

        self.memory
            .lock()
            .expect("image cache mutex")
            .insert(key, decoded.clone());

        Ok((decoded, expected_path))
    }

    pub fn stats(&self) -> CacheStats {
        let mut stats = CacheStats::default();
        collect_dir_stats(&self.cache_dir, &mut stats);
        stats
    }

    pub fn clear_cache(&self) -> AppResult<()> {
        self.memory.lock().expect("image cache mutex").clear();
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

fn collect_dir_stats(dir: &Path, stats: &mut CacheStats) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dir_stats(&path, stats);
        } else if let Ok(meta) = entry.metadata() {
            stats.file_count += 1;
            stats.total_bytes += meta.len();
        }
    }
}
