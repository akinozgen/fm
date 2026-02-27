use serde::Serialize;
use std::collections::HashSet;
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
  let disks = Disks::new_with_refreshed_list();
  for disk in disks.list() {
    let mount = disk.mount_point().to_path_buf();
    let mount_str = mount.to_string_lossy().to_string();
    if !seen_mounts.insert(mount_str.clone()) {
      continue;
    }

    let label = disk
      .name()
      .to_str()
      .map(|s| s.to_string())
      .unwrap_or_else(|| mount_str.clone());

    let item = SidebarItem {
      label,
      path: mount_str,
      kind: if disk.is_removable() {
        "device_removable".to_string()
      } else {
        "device".to_string()
      },
    };

    if disk.is_removable() {
      removable.push(item);
    } else {
      drives.push(item);
    }
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
