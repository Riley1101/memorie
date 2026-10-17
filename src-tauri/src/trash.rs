//! Soft delete for writings and folders.
//!
//! Deleting used to call `fs::remove_file` straight away, which left a writer
//! one mis-click from losing work in an app whose whole promise is that their
//! writing is safe on their own disk. Deleted items now move into a `.trash`
//! folder inside the content directory, where every scan already skips them
//! (dotfiles and dot-directories are ignored), and can be put back.
//!
//! Layout:
//!
//! ```text
//! .trash/
//!   <id>/
//!     meta.json      original path, kind and the time it was deleted
//!     payload/       the file, or the folder with everything in it
//! ```
//!
//! The id keeps two writings of the same name, deleted at different times,
//! from colliding.

use crate::error::FileError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Directory, relative to the content directory, that holds deleted items.
/// The leading dot is what keeps it out of every listing and every scan.
pub const TRASH_DIR: &str = ".trash";

/// How long a deleted item is kept before `purge_expired` may remove it.
pub const RETENTION_DAYS: u64 = 30;

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrashKind {
    File,
    Folder,
}

/// One item in the trash, as the frontend sees it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntry {
    /// Opaque id; also the folder name under `.trash`.
    pub id: String,
    /// Content-relative path the item had when it was deleted.
    pub original_path: String,
    pub kind: TrashKind,
    /// Unix seconds.
    pub deleted_at: u64,
    /// Writings inside, for a folder; 1 for a single writing.
    #[serde(default)]
    pub item_count: usize,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Enough to be unique per deletion without pulling in a uuid dependency here:
/// the timestamp in milliseconds, plus a counter for same-millisecond deletes.
fn new_id(trash_root: &Path) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mut id = stamp.to_string();
    let mut n = 1;
    while trash_root.join(&id).exists() {
        id = format!("{stamp}-{n}");
        n += 1;
    }
    id
}

fn trash_root(content_dir: &Path) -> PathBuf {
    content_dir.join(TRASH_DIR)
}

fn meta_path(entry_dir: &Path) -> PathBuf {
    entry_dir.join("meta.json")
}

/// The deleted item itself. A file keeps its basename inside `payload/` so a
/// restore can put it back under the name it had.
fn payload_path(entry_dir: &Path, original_path: &str) -> PathBuf {
    let base = Path::new(original_path)
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("item"));
    entry_dir.join("payload").join(base)
}

fn count_markdown(path: &Path) -> usize {
    if path.is_file() {
        return 1;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                count_markdown(&p)
            } else if p.extension().and_then(|x| x.to_str()) == Some(crate::fs::DEFAULT_EXTENSION) {
                1
            } else {
                0
            }
        })
        .sum()
}

/// Moves `relative` (a writing or a folder) into the trash and returns the
/// entry describing it.
pub fn send_to_trash(
    content_dir: &Path,
    relative: &str,
    kind: TrashKind,
) -> Result<TrashEntry, FileError> {
    let source = crate::fs::resolve_in(content_dir, relative)?;
    if !source.exists() {
        return Err(FileError::InvalidPath(relative.to_string()));
    }

    let root = trash_root(content_dir);
    fs::create_dir_all(&root)?;

    let id = new_id(&root);
    let entry_dir = root.join(&id);
    let target = payload_path(&entry_dir, relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let item_count = count_markdown(&source);

    // A plain rename keeps it instant and atomic within one filesystem. Across
    // filesystems it fails, so fall back to copy-then-remove.
    if fs::rename(&source, &target).is_err() {
        copy_recursive(&source, &target)?;
        if source.is_dir() {
            fs::remove_dir_all(&source)?;
        } else {
            fs::remove_file(&source)?;
        }
    }

    let entry = TrashEntry {
        id,
        original_path: relative.to_string(),
        kind,
        deleted_at: now_secs(),
        item_count,
    };
    write_meta(&entry_dir, &entry)?;
    Ok(entry)
}

fn write_meta(entry_dir: &Path, entry: &TrashEntry) -> Result<(), FileError> {
    let json = serde_json::to_string_pretty(entry)
        .map_err(|e| FileError::ContentConversionError(e.to_string()))?;
    fs::write(meta_path(entry_dir), json)?;
    Ok(())
}

fn read_meta(entry_dir: &Path) -> Option<TrashEntry> {
    let text = fs::read_to_string(meta_path(entry_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

fn copy_recursive(from: &Path, to: &Path) -> Result<(), FileError> {
    if from.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
    }
    Ok(())
}

/// Everything in the trash, most recently deleted first.
pub fn list(content_dir: &Path) -> Result<Vec<TrashEntry>, FileError> {
    let root = trash_root(content_dir);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<TrashEntry> = fs::read_dir(&root)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| read_meta(&e.path()))
        .collect();
    entries.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    Ok(entries)
}

/// Where a restore would put `entry`, avoiding a name already in use. Returns
/// the content-relative path.
fn free_restore_path(content_dir: &Path, original: &str) -> String {
    if !content_dir.join(original).exists() {
        return original.to_string();
    }
    let path = Path::new(original);
    let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Restored");
    let ext = path.extension().and_then(|s| s.to_str());

    for n in 2..1000 {
        let base = match ext {
            Some(ext) => format!("{stem} {n}.{ext}"),
            None => format!("{stem} {n}"),
        };
        let candidate = if parent.is_empty() {
            base
        } else {
            format!("{parent}/{base}")
        };
        if !content_dir.join(&candidate).exists() {
            return candidate;
        }
    }
    original.to_string()
}

/// Puts a deleted item back. Returns the path it was restored to, which gains
/// a " 2" suffix when something else has taken its old name.
pub fn restore(content_dir: &Path, id: &str) -> Result<String, FileError> {
    let entry_dir = entry_dir_of(content_dir, id)?;
    let entry = read_meta(&entry_dir).ok_or_else(|| FileError::InvalidPath(id.to_string()))?;

    let source = payload_path(&entry_dir, &entry.original_path);
    if !source.exists() {
        return Err(FileError::InvalidPath(entry.original_path.clone()));
    }

    let restored = free_restore_path(content_dir, &entry.original_path);
    let target = crate::fs::resolve_in(content_dir, &restored)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    if fs::rename(&source, &target).is_err() {
        copy_recursive(&source, &target)?;
    }
    fs::remove_dir_all(&entry_dir)?;
    Ok(restored)
}

fn entry_dir_of(content_dir: &Path, id: &str) -> Result<PathBuf, FileError> {
    // The id comes from the frontend, so it gets the same treatment as any
    // other name: one path segment, inside the trash, nowhere else.
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(FileError::InvalidPath(id.to_string()));
    }
    let dir = trash_root(content_dir).join(id);
    if !dir.is_dir() {
        return Err(FileError::InvalidPath(id.to_string()));
    }
    Ok(dir)
}

/// Deletes one trashed item for good.
pub fn purge(content_dir: &Path, id: &str) -> Result<TrashEntry, FileError> {
    let entry_dir = entry_dir_of(content_dir, id)?;
    let entry = read_meta(&entry_dir).ok_or_else(|| FileError::InvalidPath(id.to_string()))?;
    fs::remove_dir_all(&entry_dir)?;
    Ok(entry)
}

/// Empties the trash. Returns how many items were removed.
pub fn purge_all(content_dir: &Path) -> Result<usize, FileError> {
    let mut removed = 0;
    for entry in list(content_dir)? {
        if purge(content_dir, &entry.id).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Drops anything deleted more than `RETENTION_DAYS` ago. Runs at startup, so
/// the trash can't grow without bound on its own.
pub fn purge_expired(content_dir: &Path) -> Result<usize, FileError> {
    let cutoff = now_secs().saturating_sub(RETENTION_DAYS * SECONDS_PER_DAY);
    let mut removed = 0;
    for entry in list(content_dir)? {
        if entry.deleted_at < cutoff && purge(content_dir, &entry.id).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(dir: &Path, name: &str, body: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    #[test]
    fn file_round_trips_through_the_trash() {
        let dir = tempdir().unwrap();
        write(dir.path(), "Novel/Scene.md", "hello");

        let entry = send_to_trash(dir.path(), "Novel/Scene.md", TrashKind::File).unwrap();
        assert!(!dir.path().join("Novel/Scene.md").exists());
        assert_eq!(entry.item_count, 1);
        assert_eq!(list(dir.path()).unwrap().len(), 1);

        let restored = restore(dir.path(), &entry.id).unwrap();
        assert_eq!(restored, "Novel/Scene.md");
        assert_eq!(
            fs::read_to_string(dir.path().join("Novel/Scene.md")).unwrap(),
            "hello"
        );
        assert!(list(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn restore_does_not_overwrite_a_new_file_of_the_same_name() {
        let dir = tempdir().unwrap();
        write(dir.path(), "Scene.md", "old");
        let entry = send_to_trash(dir.path(), "Scene.md", TrashKind::File).unwrap();
        write(dir.path(), "Scene.md", "new");

        let restored = restore(dir.path(), &entry.id).unwrap();
        assert_eq!(restored, "Scene 2.md");
        assert_eq!(
            fs::read_to_string(dir.path().join("Scene.md")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("Scene 2.md")).unwrap(),
            "old"
        );
    }

    #[test]
    fn folder_keeps_everything_inside_it() {
        let dir = tempdir().unwrap();
        write(dir.path(), "Novel/Chapter 1/A.md", "a");
        write(dir.path(), "Novel/Chapter 1/B.md", "b");

        let entry = send_to_trash(dir.path(), "Novel/Chapter 1", TrashKind::Folder).unwrap();
        assert_eq!(entry.item_count, 2);
        assert!(!dir.path().join("Novel/Chapter 1").exists());

        restore(dir.path(), &entry.id).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("Novel/Chapter 1/B.md")).unwrap(),
            "b"
        );
    }

    #[test]
    fn purge_expired_only_takes_old_entries() {
        let dir = tempdir().unwrap();
        write(dir.path(), "Old.md", "old");
        write(dir.path(), "New.md", "new");
        let old = send_to_trash(dir.path(), "Old.md", TrashKind::File).unwrap();
        send_to_trash(dir.path(), "New.md", TrashKind::File).unwrap();

        // Backdate the first entry past the retention window.
        let entry_dir = trash_root(dir.path()).join(&old.id);
        let mut meta = read_meta(&entry_dir).unwrap();
        meta.deleted_at = now_secs() - (RETENTION_DAYS + 1) * SECONDS_PER_DAY;
        write_meta(&entry_dir, &meta).unwrap();

        assert_eq!(purge_expired(dir.path()).unwrap(), 1);
        let left = list(dir.path()).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].original_path, "New.md");
    }

    #[test]
    fn an_id_cannot_escape_the_trash_folder() {
        let dir = tempdir().unwrap();
        assert!(restore(dir.path(), "../..").is_err());
        assert!(purge(dir.path(), "../secrets").is_err());
    }
}
