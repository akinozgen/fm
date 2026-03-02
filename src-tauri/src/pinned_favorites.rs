use std::fs;
use std::path::Path;

const PINNED_FAVORITES_FILENAME: &str = "pinned_favorites.json";
const MAX_PINNED: usize = 30;

fn normalize_path(path: &str) -> String {
  let s = path.trim().replace('\\', "/");
  let s = s.trim_end_matches('/');
  if s.is_empty() {
    path.to_string()
  } else {
    s.to_string()
  }
}

fn path_file(config_dir: &str) -> std::path::PathBuf {
  Path::new(config_dir).join(PINNED_FAVORITES_FILENAME)
}

fn load_pinned(config_dir: &str) -> Vec<String> {
  let p = path_file(config_dir);
  let contents = match fs::read_to_string(&p) {
    Ok(c) => c,
    Err(_) => return Vec::new(),
  };
  let parsed: Option<Vec<String>> = serde_json::from_str(&contents).ok();
  parsed.unwrap_or_default()
}

fn save_pinned(config_dir: &str, paths: &[String]) -> Result<(), String> {
  let p = path_file(config_dir);
  let json = serde_json::to_string_pretty(paths).map_err(|e| e.to_string())?;
  fs::write(&p, json).map_err(|e| e.to_string())?;
  Ok(())
}

pub fn get_pinned_favorites(config_dir: &str) -> Vec<String> {
  let paths = load_pinned(config_dir);
  paths.into_iter().map(|p| normalize_path(&p)).collect()
}

pub fn add_pinned_favorite(config_dir: &str, path: String) -> Result<(), String> {
  let normalized = normalize_path(&path);
  if normalized.is_empty() {
    return Ok(());
  }
  let mut paths = load_pinned(config_dir);
  paths.retain(|p| normalize_path(p) != normalized);
  paths.insert(0, normalized.clone());
  if paths.len() > MAX_PINNED {
    paths.truncate(MAX_PINNED);
  }
  save_pinned(config_dir, &paths)
}

pub fn remove_pinned_favorite(config_dir: &str, path: String) -> Result<(), String> {
  let normalized = normalize_path(&path);
  if normalized.is_empty() {
    return Ok(());
  }
  let mut paths = load_pinned(config_dir);
  paths.retain(|p| normalize_path(p) != normalized);
  save_pinned(config_dir, &paths)
}

pub fn set_pinned_favorites(config_dir: &str, paths: Vec<String>) -> Result<(), String> {
  let normalized: Vec<String> = paths
    .into_iter()
    .map(|p| normalize_path(&p))
    .filter(|p| !p.is_empty())
    .collect();
  let deduped: Vec<String> = normalized
    .into_iter()
    .fold(Vec::new(), |mut acc, p| {
      if !acc.contains(&p) {
        acc.push(p);
      }
      acc
    });
  let truncated = if deduped.len() > MAX_PINNED {
    deduped.into_iter().take(MAX_PINNED).collect()
  } else {
    deduped
  };
  save_pinned(config_dir, &truncated)
}
