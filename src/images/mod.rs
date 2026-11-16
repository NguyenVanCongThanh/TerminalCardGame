pub mod renderer;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use image::DynamicImage;
use reqwest::Client;
use tokio::sync::Semaphore;

use crate::domain::{Card, ImageQuality};
use crate::error::{AppError, AppResult};

const MAX_MEMORY_IMAGES: usize = 64;
const MAX_CONCURRENT_DOWNLOADS: usize = 6;

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
    download_semaphore: Semaphore,
    generation: AtomicU64,
    temp_seq: AtomicU64,
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

    fn touch(&mut self, key: &str) {
        self.order.retain(|k| k != key);
        self.order.push_back(key.to_string());
    }

    fn get(&mut self, key: &str) -> Option<DynamicImage> {
        let img = self.map.get(key).cloned()?;
        self.touch(key);
        Some(img)
    }

    fn insert(&mut self, key: String, img: DynamicImage) {
        if self.map.contains_key(&key) {
            self.touch(&key);
        } else {
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
            download_semaphore: Semaphore::new(MAX_CONCURRENT_DOWNLOADS),
            generation: AtomicU64::new(0),
            temp_seq: AtomicU64::new(0),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
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

    fn is_quality_cache_file(&self, path: &Path) -> bool {
        if !path.starts_with(&self.cache_dir) {
            return false;
        }
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| name.ends_with("_high.png") || name.ends_with("_low.png"))
    }

    /// Returns a cached `DynamicImage` synchronously from memory if already decoded.
    pub fn get_in_memory(&self, card_id: &str, quality: ImageQuality) -> Option<DynamicImage> {
        let key = Self::cache_key(card_id, quality);
        self.memory.lock().expect("image cache mutex").get(&key)
    }

    async fn decode_from_disk(disk_path: PathBuf) -> AppResult<DynamicImage> {
        tokio::task::spawn_blocking(move || -> AppResult<DynamicImage> {
            let bytes = std::fs::read(&disk_path)?;
            Ok(image::load_from_memory(&bytes)?)
        })
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?
    }

    /// Loads the card image from memory, local disk cache, or remote URL (caching to disk & memory).
    pub async fn load_card_image(
        self: &Arc<Self>,
        card: &Card,
        quality: ImageQuality,
    ) -> AppResult<(DynamicImage, PathBuf)> {
        let start_gen = self.generation.load(Ordering::Acquire);
        let key = Self::cache_key(&card.id, quality);
        let expected_path = self.disk_path_for(card, quality);

        if let Some(img) = self.memory.lock().expect("image cache mutex").get(&key) {
            return Ok((img, expected_path));
        }

        // Check quality-specific disk cache first; only skip network for non-cache custom paths.
        let custom_local_path = card.local_image_path.as_ref().and_then(|saved| {
            let p = PathBuf::from(saved);
            if p.exists() && !self.is_quality_cache_file(&p) {
                Some(p)
            } else {
                None
            }
        });

        let direct_disk = if expected_path.exists() {
            Some(expected_path.clone())
        } else {
            custom_local_path
        };

        if let Some(disk_path) = direct_disk {
            let decoded = Self::decode_from_disk(disk_path.clone()).await?;
            if self.generation.load(Ordering::Acquire) != start_gen {
                return Err(AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "Image cache cleared while decoding",
                )));
            }
            self.memory
                .lock()
                .expect("image cache mutex")
                .insert(key, decoded.clone());
            return Ok((decoded, disk_path));
        }

        let target_url = match quality {
            ImageQuality::High => card.image_url.as_ref().or(card.thumbnail_url.as_ref()),
            ImageQuality::Low => card.thumbnail_url.as_ref().or(card.image_url.as_ref()),
        };

        let offline_fallback_path = card
            .local_image_path
            .as_ref()
            .map(PathBuf::from)
            .filter(|p| p.exists());

        let Some(url) = target_url else {
            if let Some(fallback) = offline_fallback_path {
                let decoded = Self::decode_from_disk(fallback.clone()).await?;
                return Ok((decoded, fallback));
            }
            return Err(AppError::Provider(format!(
                "Card '{}' has no image URL",
                card.name
            )));
        };

        let download_res: AppResult<Vec<u8>> = async {
            let _permit = self
                .download_semaphore
                .acquire()
                .await
                .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;
            let resp = self.client.get(url).send().await?;
            if !resp.status().is_success() {
                return Err(AppError::Provider(format!(
                    "Artwork download failed (HTTP {}) for {}",
                    resp.status(),
                    card.name
                )));
            }
            Ok(resp.bytes().await?.to_vec())
        }
        .await;

        let bytes_vec = match download_res {
            Ok(bytes) => bytes,
            Err(err) => {
                if let Some(fallback) = offline_fallback_path {
                    let decoded = Self::decode_from_disk(fallback.clone()).await?;
                    return Ok((decoded, fallback));
                }
                return Err(err);
            }
        };

        if self.generation.load(Ordering::Acquire) != start_gen {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Image cache cleared during download",
            )));
        }

        let write_path = expected_path.clone();
        let tmp_seq = self.temp_seq.fetch_add(1, Ordering::Relaxed);
        let decoded = tokio::task::spawn_blocking(move || -> AppResult<DynamicImage> {
            let img = image::load_from_memory(&bytes_vec)?;
            if let Some(parent) = write_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tmp_path = write_path.with_extension(format!("png.{tmp_seq}.tmp"));
            std::fs::write(&tmp_path, &bytes_vec)?;
            std::fs::rename(&tmp_path, &write_path)?;
            Ok(img)
        })
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))??;

        if self.generation.load(Ordering::Acquire) != start_gen {
            let _ = std::fs::remove_file(&expected_path);
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Image cache cleared before commit",
            )));
        }

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
        self.generation.fetch_add(1, Ordering::Release);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_lru_evicts_least_recently_used() {
        let mut lru = MemoryImageLru::new();
        let img = DynamicImage::new_rgba8(1, 1);
        for i in 0..MAX_MEMORY_IMAGES {
            lru.insert(format!("k{i}"), img.clone());
        }
        // Touch k0 so it becomes most recently used; k1 should be evicted next.
        assert!(lru.get("k0").is_some());
        lru.insert("extra".to_string(), img);
        assert!(lru.get("k0").is_some(), "touched key k0 should stay cached");
        assert!(lru.get("k1").is_none(), "untouched oldest key k1 should be evicted");
    }
}
