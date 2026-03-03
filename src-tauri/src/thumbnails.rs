use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::Engine;
use image::imageops::FilterType;
use rayon::prelude::*;

// ── Shared state ─────────────────────────────────────────────────────────────

/// Managed Tauri state for thumbnail generation.
/// Owns the bounded thread pool and the cancel flag.
pub struct ThumbnailState {
  /// Set to true by `cancel_thumbnails_cmd`; checked between items in a batch.
  pub cancel: Arc<AtomicBool>,
  /// Dedicated rayon pool — intentionally small to avoid saturating the CPU.
  /// Size: half of logical cores, capped at 3 (leaves room for UI + OS).
  pub pool: Arc<rayon::ThreadPool>,
}

impl ThumbnailState {
  pub fn new() -> Self {
    let num_threads = std::thread::available_parallelism()
      .map(|n| (n.get() / 2).max(1).min(3))
      .unwrap_or(2);

    let pool = rayon::ThreadPoolBuilder::new()
      .num_threads(num_threads)
      .build()
      .expect("failed to build thumbnail thread pool");

    Self {
      cancel: Arc::new(AtomicBool::new(false)),
      pool: Arc::new(pool),
    }
  }
}

// ── Core functions ────────────────────────────────────────────────────────────

/// Generate or serve a cached square JPEG thumbnail.
/// `size` is the logical CSS pixel size (e.g. 48); rendered at 2× for HiDPI.
/// Returns a base64-encoded JPEG string.
pub fn get_thumbnail(path: &str, size: u32, thumb_dir: &Path) -> Result<String, String> {
  // Never generate thumbnails for files that are already inside our thumbnail
  // cache directory; otherwise we recursively create thumbnails of thumbnails.
  if Path::new(path).starts_with(thumb_dir) {
    return Err("skip thumbnail cache path".to_string());
  }

  let mtime = mtime_secs(path);
  let filename = cache_filename(path, mtime, size);
  let cache_path = thumb_dir.join(&filename);

  // Cache hit — return without any decode work
  if let Ok(data) = std::fs::read(&cache_path) {
    return Ok(base64::engine::general_purpose::STANDARD.encode(&data));
  }

  let img = image::open(path).map_err(|e| format!("decode: {e}"))?;

  let px = size.saturating_mul(2);
  let thumb = img.resize_to_fill(px, px, FilterType::Triangle);

  // Flatten to RGB (JPEG cannot carry an alpha channel)
  let rgb = thumb.to_rgb8();
  let mut buf = Vec::new();
  image::DynamicImage::ImageRgb8(rgb)
    .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg)
    .map_err(|e| format!("encode: {e}"))?;

  let _ = std::fs::create_dir_all(thumb_dir);
  let _ = std::fs::write(&cache_path, &buf);

  Ok(base64::engine::general_purpose::STANDARD.encode(&buf))
}

/// Process a batch of paths using the bounded thread pool.
/// Checks `cancel` before starting each item — on cancellation all remaining
/// items return `None` immediately without touching the disk or decoder.
pub fn get_thumbnails_parallel(
  paths: &[String],
  size: u32,
  thumb_dir: &Path,
  cancel: &Arc<AtomicBool>,
  pool: &Arc<rayon::ThreadPool>,
) -> Vec<Option<String>> {
  pool.install(|| {
    paths
      .par_iter()
      .map(|path| {
        if cancel.load(Ordering::Relaxed) {
          return None;
        }
        get_thumbnail(path, size, thumb_dir).ok()
      })
      .collect()
  })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn mtime_secs(path: &str) -> u64 {
  std::fs::metadata(path)
    .ok()
    .and_then(|m| m.modified().ok())
    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|d| d.as_secs())
    .unwrap_or(0)
}

fn cache_filename(path: &str, mtime: u64, size: u32) -> String {
  let mut h = DefaultHasher::new();
  path.hash(&mut h);
  mtime.hash(&mut h);
  size.hash(&mut h);
  format!("{:016x}.jpg", h.finish())
}
