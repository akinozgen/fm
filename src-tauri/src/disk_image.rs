use std::path::Path;

// ── Format detection ──────────────────────────────────────────────────────────

pub fn is_disk_image(path: &Path) -> bool {
  let name = match path.file_name().and_then(|n| n.to_str()) {
    Some(n) => n.to_lowercase(),
    None => return false,
  };
  name.ends_with(".dmg")
    || name.ends_with(".iso")
    || name.ends_with(".img")
    || name.ends_with(".cdr")
    || name.ends_with(".toast")
}

// ── Mount ─────────────────────────────────────────────────────────────────────

/// Mount a disk image and return the mount point path, or `None` if the
/// platform/format combination is not supported (caller falls back to open).
pub fn mount(path: &Path) -> Result<Option<String>, String> {
  let path_str = path.to_string_lossy();

  #[cfg(target_os = "macos")]
  {
    return mount_macos(&path_str);
  }

  #[cfg(target_os = "linux")]
  {
    let name = path
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or("")
      .to_lowercase();
    let supported = name.ends_with(".iso") || name.ends_with(".img");
    if !supported {
      return Ok(None);
    }
    return mount_linux(&path_str);
  }

  #[cfg(target_os = "windows")]
  {
    let name = path
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or("")
      .to_lowercase();
    let supported = name.ends_with(".iso") || name.ends_with(".img");
    if !supported {
      return Ok(None);
    }
    return mount_windows(&path_str);
  }

  #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
  Ok(None)
}

// ── macOS: hdiutil attach ─────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn mount_macos(path: &str) -> Result<Option<String>, String> {
  let output = std::process::Command::new("hdiutil")
    .args(["attach", path])
    .output()
    .map_err(|e| e.to_string())?;

  if !output.status.success() {
    let msg = String::from_utf8_lossy(&output.stderr);
    return Err(msg.trim().to_string());
  }

  // hdiutil prints tab-separated lines. Each line has up to 3 columns:
  //   /dev/disk2          <scheme>
  //   /dev/disk2s1        <type>       /Volumes/MyDisk
  // We want the last non-empty 3rd column.
  let stdout = String::from_utf8_lossy(&output.stdout);
  let mount_point = stdout
    .lines()
    .filter_map(|line| {
      let cols: Vec<&str> = line.splitn(3, '\t').collect();
      cols.get(2).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string())
    })
    .last();

  Ok(mount_point)
}

// ── Linux: udisksctl loop-setup + mount ──────────────────────────────────────

#[cfg(target_os = "linux")]
fn mount_linux(path: &str) -> Result<Option<String>, String> {
  // Step 1: create loop device
  let setup = std::process::Command::new("udisksctl")
    .args(["loop-setup", "--file", path, "--no-user-interaction"])
    .output()
    .map_err(|e| e.to_string())?;

  if !setup.status.success() {
    let msg = String::from_utf8_lossy(&setup.stderr);
    return Err(msg.trim().to_string());
  }

  // Stdout: "Mapped file /path as /dev/loop0."
  let setup_out = String::from_utf8_lossy(&setup.stdout);
  let device = setup_out
    .split_whitespace()
    .rev()
    .find(|s| s.starts_with("/dev/loop"))
    .map(|s| s.trim_end_matches('.').to_string())
    .ok_or_else(|| format!("could not parse loop device from: {setup_out}"))?;

  // Step 2: mount the loop device
  let mnt = std::process::Command::new("udisksctl")
    .args(["mount", "--block-device", &device, "--no-user-interaction"])
    .output()
    .map_err(|e| e.to_string())?;

  if !mnt.status.success() {
    let msg = String::from_utf8_lossy(&mnt.stderr);
    return Err(msg.trim().to_string());
  }

  // Stdout: "Mounted /dev/loop0 at /run/media/user/label."
  let mnt_out = String::from_utf8_lossy(&mnt.stdout);
  let mount_point = mnt_out
    .split(" at ")
    .nth(1)
    .map(|s| s.trim_end_matches('.').trim().to_string())
    .filter(|s| !s.is_empty());

  Ok(mount_point)
}

// ── Windows: PowerShell Mount-DiskImage ──────────────────────────────────────

#[cfg(target_os = "windows")]
fn mount_windows(path: &str) -> Result<Option<String>, String> {
  // Mount-DiskImage lets Windows assign the next available drive letter.
  // -PassThru | Get-Volume returns the volume(s) created; we take the first
  // non-empty drive letter (multi-partition images can yield several rows).
  let script = format!(
    "Mount-DiskImage -ImagePath '{}' -PassThru | Get-Volume | \
     Where-Object {{ $_.DriveLetter }} | \
     Select-Object -First 1 -ExpandProperty DriveLetter",
    path.replace('\'', "''")
  );
  let output = std::process::Command::new("powershell")
    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
    .output()
    .map_err(|e| e.to_string())?;

  if !output.status.success() {
    let msg = String::from_utf8_lossy(&output.stderr);
    return Err(msg.trim().to_string());
  }

  let letter = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if letter.is_empty() {
    return Ok(None);
  }
  // letter is a single character like "E"; build the root path "E:\"
  Ok(Some(format!("{}:\\", letter)))
}
