use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── Format detection ──────────────────────────────────────────────────────────

pub fn archive_format(path: &Path) -> Option<&'static str> {
  let name = path.file_name()?.to_str()?.to_lowercase();
  if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
    return Some("tar.gz");
  }
  if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
    return Some("tar.bz2");
  }
  if name.ends_with(".tar.xz") || name.ends_with(".txz") {
    return Some("tar.xz");
  }
  if name.ends_with(".tar") {
    return Some("tar");
  }
  if name.ends_with(".zip") {
    return Some("zip");
  }
  None
}

pub fn is_archive(path: &Path) -> bool {
  archive_format(path).is_some()
}

// ── Extraction entry point ────────────────────────────────────────────────────

pub fn extract(
  src: &Path,
  dest: &Path,
  overwrite: bool,
  cancel: Arc<AtomicBool>,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  fs::create_dir_all(dest).map_err(|e| e.to_string())?;
  match archive_format(src) {
    Some("zip")     => extract_zip(src, dest, overwrite, cancel, progress),
    Some("tar.gz")  => extract_tar_gz(src, dest, overwrite, cancel, progress),
    Some("tar.bz2") => extract_tar_bz2(src, dest, overwrite, cancel, progress),
    Some("tar.xz")  => extract_tar_xz(src, dest, overwrite, cancel, progress),
    Some("tar")     => extract_tar_plain(src, dest, overwrite, cancel, progress),
    _ => Err(format!("unsupported archive format: {}", src.display())),
  }
}

// ── Creation entry point ──────────────────────────────────────────────────────

pub fn create(
  paths: &[PathBuf],
  output: &Path,
  format: &str,
  cancel: Arc<AtomicBool>,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  if let Some(parent) = output.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  match format {
    "zip"     => create_zip(paths, output, cancel, progress),
    "tar.gz"  => create_tar_gz(paths, output, cancel, progress),
    "tar.bz2" => create_tar_bz2(paths, output, cancel, progress),
    "tar.xz"  => create_tar_xz(paths, output, cancel, progress),
    "tar"     => create_tar_plain(paths, output, cancel, progress),
    "gz"      => create_single_gz(paths, output, progress),
    "bz2"     => create_single_bz2(paths, output, progress),
    "xz"      => create_single_xz(paths, output, progress),
    _ => Err(format!("unsupported format: {format}")),
  }
}

// ── Collect files for archiving ───────────────────────────────────────────────

struct CollectedEntry {
  /// Absolute path to the file on disk
  abs: PathBuf,
  /// Relative path stored inside the archive (uses forward slashes)
  rel: String,
}

fn collect_entries(paths: &[PathBuf]) -> Vec<CollectedEntry> {
  let mut out = Vec::new();
  for root in paths {
    if root.is_file() {
      let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
      out.push(CollectedEntry { abs: root.clone(), rel: name });
    } else if root.is_dir() {
      let base_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dir".to_string());
      for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        if entry.file_type().is_file() {
          if let Ok(rel_path) = entry.path().strip_prefix(root.parent().unwrap_or(root)) {
            let rel = format!(
              "{}/{}",
              base_name,
              rel_path
                .to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches(&format!("{base_name}/"))
            );
            // Strip double prefix if strip_prefix already included base_name
            let rel = if rel.starts_with(&format!("{base_name}/{base_name}/")) {
              rel[base_name.len() + 1..].to_string()
            } else {
              rel
            };
            out.push(CollectedEntry { abs: entry.path().to_path_buf(), rel });
          }
        }
      }
    }
  }
  out
}

// ── ZIP extract ───────────────────────────────────────────────────────────────

fn extract_zip(
  src: &Path,
  dest: &Path,
  overwrite: bool,
  cancel: Arc<AtomicBool>,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let file = fs::File::open(src).map_err(|e| e.to_string())?;
  let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
  let total = archive.len() as u64;

  for i in 0..archive.len() {
    if cancel.load(Ordering::Relaxed) {
      return Err("cancelled".to_string());
    }
    let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
    let name = entry.name().to_string();
    progress(i as u64, total, &name);

    let out_path = dest.join(sanitize_path(&name));
    if entry.is_dir() {
      fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
    } else {
      if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
      }
      if !overwrite && out_path.exists() {
        continue;
      }
      let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
      io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
    }
  }
  progress(total, total, "");
  Ok(())
}

// ── TAR extract helpers ───────────────────────────────────────────────────────

fn extract_tar_entries<R: Read>(
  archive: &mut tar::Archive<R>,
  dest: &Path,
  overwrite: bool,
  cancel: Arc<AtomicBool>,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  // tar doesn't give a count upfront; use entry index as "done", 0 total = unknown
  let mut done = 0u64;
  for entry in archive.entries().map_err(|e| e.to_string())? {
    if cancel.load(Ordering::Relaxed) {
      return Err("cancelled".to_string());
    }
    let mut entry = entry.map_err(|e| e.to_string())?;
    let path = entry.path().map_err(|e| e.to_string())?.to_path_buf();
    let name = path.to_string_lossy().into_owned();
    progress(done, 0, &name);

    let out_path = dest.join(sanitize_path(&name));
    if entry.header().entry_type().is_dir() {
      fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
    } else {
      if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
      }
      if !overwrite && out_path.exists() {
        done += 1;
        continue;
      }
      let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
      io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
    }
    done += 1;
  }
  progress(done, done, "");
  Ok(())
}

fn extract_tar_plain(
  src: &Path, dest: &Path, overwrite: bool,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let file = fs::File::open(src).map_err(|e| e.to_string())?;
  let mut archive = tar::Archive::new(file);
  extract_tar_entries(&mut archive, dest, overwrite, cancel, progress)
}

fn extract_tar_gz(
  src: &Path, dest: &Path, overwrite: bool,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let file = fs::File::open(src).map_err(|e| e.to_string())?;
  let gz = flate2::read::GzDecoder::new(file);
  let mut archive = tar::Archive::new(gz);
  extract_tar_entries(&mut archive, dest, overwrite, cancel, progress)
}

fn extract_tar_bz2(
  src: &Path, dest: &Path, overwrite: bool,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let file = fs::File::open(src).map_err(|e| e.to_string())?;
  let bz = bzip2::read::BzDecoder::new(file);
  let mut archive = tar::Archive::new(bz);
  extract_tar_entries(&mut archive, dest, overwrite, cancel, progress)
}

fn extract_tar_xz(
  src: &Path, dest: &Path, overwrite: bool,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let file = fs::File::open(src).map_err(|e| e.to_string())?;
  let xz = xz2::read::XzDecoder::new(file);
  let mut archive = tar::Archive::new(xz);
  extract_tar_entries(&mut archive, dest, overwrite, cancel, progress)
}

// ── ZIP create ────────────────────────────────────────────────────────────────

fn create_zip(
  paths: &[PathBuf],
  output: &Path,
  cancel: Arc<AtomicBool>,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let entries = collect_entries(paths);
  let total = entries.len() as u64;
  let file = fs::File::create(output).map_err(|e| e.to_string())?;
  let mut zip = zip::ZipWriter::new(file);
  let options = zip::write::SimpleFileOptions::default()
    .compression_method(zip::CompressionMethod::Deflated);

  for (i, entry) in entries.iter().enumerate() {
    if cancel.load(Ordering::Relaxed) {
      drop(zip);
      let _ = fs::remove_file(output);
      return Err("cancelled".to_string());
    }
    progress(i as u64, total, &entry.rel);
    zip.start_file(&entry.rel, options).map_err(|e| e.to_string())?;
    let mut src = fs::File::open(&entry.abs).map_err(|e| e.to_string())?;
    io::copy(&mut src, &mut zip).map_err(|e| e.to_string())?;
  }
  zip.finish().map_err(|e| e.to_string())?;
  progress(total, total, "");
  Ok(())
}

// ── TAR create helpers ────────────────────────────────────────────────────────

fn append_entries<W: Write>(
  builder: &mut tar::Builder<W>,
  entries: &[CollectedEntry],
  cancel: &Arc<AtomicBool>,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let total = entries.len() as u64;
  for (i, entry) in entries.iter().enumerate() {
    if cancel.load(Ordering::Relaxed) {
      return Err("cancelled".to_string());
    }
    progress(i as u64, total, &entry.rel);
    let mut file = fs::File::open(&entry.abs).map_err(|e| e.to_string())?;
    builder
      .append_file(&entry.rel, &mut file)
      .map_err(|e| e.to_string())?;
  }
  progress(total, total, "");
  Ok(())
}

fn create_tar_plain(
  paths: &[PathBuf], output: &Path,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let entries = collect_entries(paths);
  let file = fs::File::create(output).map_err(|e| e.to_string())?;
  let mut builder = tar::Builder::new(file);
  append_entries(&mut builder, &entries, &cancel, progress)?;
  builder.finish().map_err(|e| e.to_string())?;
  Ok(())
}

fn create_tar_gz(
  paths: &[PathBuf], output: &Path,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let entries = collect_entries(paths);
  let file = fs::File::create(output).map_err(|e| e.to_string())?;
  let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
  let mut builder = tar::Builder::new(gz);
  append_entries(&mut builder, &entries, &cancel, progress)?;
  builder.into_inner().map_err(|e| e.to_string())?.finish().map_err(|e| e.to_string())?;
  Ok(())
}

fn create_tar_bz2(
  paths: &[PathBuf], output: &Path,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let entries = collect_entries(paths);
  let file = fs::File::create(output).map_err(|e| e.to_string())?;
  let bz = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
  let mut builder = tar::Builder::new(bz);
  append_entries(&mut builder, &entries, &cancel, progress)?;
  builder.into_inner().map_err(|e| e.to_string())?.finish().map_err(|e| e.to_string())?;
  Ok(())
}

fn create_tar_xz(
  paths: &[PathBuf], output: &Path,
  cancel: Arc<AtomicBool>, progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let entries = collect_entries(paths);
  let file = fs::File::create(output).map_err(|e| e.to_string())?;
  let xz = xz2::write::XzEncoder::new(file, 6);
  let mut builder = tar::Builder::new(xz);
  append_entries(&mut builder, &entries, &cancel, progress)?;
  builder.into_inner().map_err(|e| e.to_string())?.finish().map_err(|e| e.to_string())?;
  Ok(())
}

// ── Single-file compress ──────────────────────────────────────────────────────

fn create_single_gz(
  paths: &[PathBuf], output: &Path,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let src_path = paths.first().ok_or("no input file")?;
  let name = src_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
  progress(0, 1, &name);
  let mut src = fs::File::open(src_path).map_err(|e| e.to_string())?;
  let dst = fs::File::create(output).map_err(|e| e.to_string())?;
  let mut enc = flate2::write::GzEncoder::new(dst, flate2::Compression::default());
  io::copy(&mut src, &mut enc).map_err(|e| e.to_string())?;
  enc.finish().map_err(|e| e.to_string())?;
  progress(1, 1, "");
  Ok(())
}

fn create_single_bz2(
  paths: &[PathBuf], output: &Path,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let src_path = paths.first().ok_or("no input file")?;
  let name = src_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
  progress(0, 1, &name);
  let mut src = fs::File::open(src_path).map_err(|e| e.to_string())?;
  let dst = fs::File::create(output).map_err(|e| e.to_string())?;
  let mut enc = bzip2::write::BzEncoder::new(dst, bzip2::Compression::default());
  io::copy(&mut src, &mut enc).map_err(|e| e.to_string())?;
  enc.finish().map_err(|e| e.to_string())?;
  progress(1, 1, "");
  Ok(())
}

fn create_single_xz(
  paths: &[PathBuf], output: &Path,
  progress: impl Fn(u64, u64, &str),
) -> Result<(), String> {
  let src_path = paths.first().ok_or("no input file")?;
  let name = src_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
  progress(0, 1, &name);
  let mut src = fs::File::open(src_path).map_err(|e| e.to_string())?;
  let dst = fs::File::create(output).map_err(|e| e.to_string())?;
  let mut enc = xz2::write::XzEncoder::new(dst, 6);
  io::copy(&mut src, &mut enc).map_err(|e| e.to_string())?;
  enc.finish().map_err(|e| e.to_string())?;
  progress(1, 1, "");
  Ok(())
}

// ── Path sanitization (prevent path traversal) ────────────────────────────────

fn sanitize_path(raw: &str) -> PathBuf {
  let mut out = PathBuf::new();
  for component in Path::new(raw).components() {
    match component {
      std::path::Component::Normal(c) => out.push(c),
      _ => {}
    }
  }
  out
}
