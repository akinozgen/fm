use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use directories::UserDirs;
use sysinfo::Disks;

#[derive(Debug, Serialize, Clone)]
pub struct SidebarItem {
  pub label: String,
  pub path: String,
  pub kind: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SidebarSection {
  pub title: String,
  pub items: Vec<SidebarItem>,
}

fn add_dir(items: &mut Vec<SidebarItem>, label: &str, path: Option<PathBuf>) {
  if let Some(path) = path {
    items.push(SidebarItem {
      label: label.to_string(),
      path: path.to_string_lossy().to_string(),
      kind: "folder".to_string(),
    });
  }
}

fn add_dir_if_exists(items: &mut Vec<SidebarItem>, label: &str, path: PathBuf) {
  if path.exists() && !items.iter().any(|i| i.path == path.to_string_lossy()) {
    items.push(SidebarItem {
      label: label.to_string(),
      path: path.to_string_lossy().to_string(),
      kind: "folder".to_string(),
    });
  }
}

fn format_drive_size(bytes: u64) -> String {
  const GB: u64 = 1024 * 1024 * 1024;
  const TB: u64 = GB * 1024;
  if bytes >= TB {
    format!("{:.1} TB", (bytes as f64) / (TB as f64))
  } else if bytes >= GB {
    format!("{:.0} GB", (bytes as f64) / (GB as f64))
  } else {
    format!("{:.0} MB", (bytes as f64) / (1024.0 * 1024.0))
  }
}

fn linux_mount_priority(mount: &str, home_mount: Option<&str>) -> (u8, usize) {
  if mount == "/" {
    return (0, 0);
  }
  if let Some(home) = home_mount {
    if mount == home {
      return (1, 0);
    }
  }
  let depth = mount.split('/').filter(|s| !s.is_empty()).count();
  (2, depth)
}

fn human_drive_label(mount: &str, device_name: &str, total_space: u64, home_mount: Option<&str>) -> String {
  let base = if mount == "/" {
    "Root".to_string()
  } else if let Some(home) = home_mount {
    if mount == home {
      "Home".to_string()
    } else {
      PathBuf::from(mount)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(device_name)
        .to_string()
    }
  } else {
    PathBuf::from(mount)
      .file_name()
      .and_then(|n| n.to_str())
      .filter(|s| !s.is_empty())
      .unwrap_or(device_name)
      .to_string()
  };

  format!("{base} ({})", format_drive_size(total_space))
}

pub fn build_sidebar() -> Vec<SidebarSection> {
  let mut sections: Vec<SidebarSection> = Vec::new();
  let username = whoami::username();

  let mut favorites: Vec<SidebarItem> = Vec::new();
  if let Some(user_dirs) = UserDirs::new() {
    let home = user_dirs.home_dir().to_path_buf();
    favorites.push(SidebarItem {
      label: username.clone(),
      path: home.to_string_lossy().to_string(),
      kind: "home".to_string(),
    });
    add_dir(&mut favorites, "Desktop", user_dirs.desktop_dir().map(|p| p.to_path_buf()));
    add_dir(&mut favorites, "Downloads", user_dirs.download_dir().map(|p| p.to_path_buf()));
    add_dir(&mut favorites, "Documents", user_dirs.document_dir().map(|p| p.to_path_buf()));
    add_dir(&mut favorites, "Pictures", user_dirs.picture_dir().map(|p| p.to_path_buf()));
    add_dir(&mut favorites, "Music", user_dirs.audio_dir().map(|p| p.to_path_buf()));
    add_dir(&mut favorites, "Videos", user_dirs.video_dir().map(|p| p.to_path_buf()));

    // Fallback paths for systems where XDG user-dirs are not configured.
    add_dir_if_exists(&mut favorites, "Desktop", home.join("Desktop"));
    add_dir_if_exists(&mut favorites, "Downloads", home.join("Downloads"));
    add_dir_if_exists(&mut favorites, "Documents", home.join("Documents"));
    add_dir_if_exists(&mut favorites, "Pictures", home.join("Pictures"));
    add_dir_if_exists(&mut favorites, "Music", home.join("Music"));
    add_dir_if_exists(&mut favorites, "Videos", home.join("Videos"));

    favorites.push(SidebarItem {
      label: "Trash".to_string(),
      path: "fm://trash".to_string(),
      kind: "trash".to_string(),
    });
  }

  sections.push(SidebarSection {
    title: format!("User ({})", username),
    items: favorites,
  });

  let mut drives: Vec<SidebarItem> = Vec::new();
  let mut removable: Vec<SidebarItem> = Vec::new();
  let mut seen_mounts: HashSet<String> = HashSet::new();
  let home_mount = UserDirs::new()
    .map(|u| u.home_dir().to_string_lossy().to_string());

  #[cfg(target_os = "linux")]
  let mut linux_by_device: HashMap<String, SidebarItem> = HashMap::new();

  let disks = Disks::new_with_refreshed_list();
  for disk in disks.list() {
    let mount = disk.mount_point().to_path_buf();
    let mount_str = mount.to_string_lossy().to_string();
    if !seen_mounts.insert(mount_str.clone()) {
      continue;
    }

    let device_name = disk
      .name()
      .to_str()
      .map(|s| s.to_string())
      .unwrap_or_else(|| mount_str.clone());

    let item = SidebarItem {
      label: human_drive_label(
        &mount_str,
        &device_name,
        disk.total_space(),
        home_mount.as_deref(),
      ),
      path: mount_str.clone(),
      kind: if disk.is_removable() {
        "device_removable".to_string()
      } else {
        "device".to_string()
      },
    };

    #[cfg(target_os = "linux")]
    {
      if disk.is_removable() {
        removable.push(item);
      } else {
        // Linux often mounts the same device at multiple mount points (subvolumes/binds).
        // Keep one representative mount per device with stable priority.
        let next_priority = linux_mount_priority(&mount_str, home_mount.as_deref());
        let key = device_name.clone();
        match linux_by_device.get(&key) {
          Some(existing) => {
            let existing_priority = linux_mount_priority(&existing.path, home_mount.as_deref());
            if next_priority < existing_priority {
              linux_by_device.insert(key, item);
            }
          }
          None => {
            linux_by_device.insert(key, item);
          }
        }
      }
      continue;
    }

    #[cfg(not(target_os = "linux"))]
    {
      if disk.is_removable() {
        removable.push(item);
      } else {
        drives.push(item);
      }
    }
  }

  #[cfg(target_os = "linux")]
  {
    drives = linux_by_device.into_values().collect();
    drives.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
  }

  sections.push(SidebarSection {
    title: "Drives".to_string(),
    items: drives,
  });

  sections.push(SidebarSection {
    title: "Removable".to_string(),
    items: removable,
  });

  sections
}
