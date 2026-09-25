//! Keeps the note index (see `memory`) in step with the content directory.
//!
//! A manifest records, per note, the mtime and size it had and the hash of its
//! text when it was last indexed. A pass stats every note, reads only the ones
//! whose mtime or size moved, and re-embeds only those whose text actually
//! changed, so a pass over an unchanged folder is a directory walk. A file
//! watcher starts a pass whenever notes change on disk, whether the app saved
//! them or a git pull, Dropbox pull or another editor did.

use crate::fs;
use crate::memory::{IndexStats, MemoryDocumentAnalysisExt, NoteDocument, EMBED_VERSION};
use crate::AppState;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

/// Changes are gathered for this long before a pass starts, so an autosave
/// or a pull that touches many files triggers one pass, not one per file.
const WATCH_DEBOUNCE: Duration = Duration::from_millis(1500);

/// A file modified this close to when it was last checked may have changed
/// again within the same mtime tick, so its mtime isn't trusted (git calls
/// this "racily clean").
const RACY_WINDOW_MS: i64 = 2000;

/// The manifest is written every this many indexed notes, so an interrupted
/// pass doesn't start over.
const SAVE_EVERY: usize = 25;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Entry {
    mtime_ms: i64,
    size: u64,
    hash: String,
    checked_at_ms: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Manifest {
    /// The `EMBED_VERSION` the entries were indexed with.
    version: String,
    entries: HashMap<String, Entry>,
}

fn millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn content_hash(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

impl Entry {
    /// Whether the file can be assumed unchanged from its metadata alone.
    fn matches(&self, mtime_ms: i64, size: u64) -> bool {
        self.mtime_ms == mtime_ms
            && self.size == size
            && self.checked_at_ms - self.mtime_ms > RACY_WINDOW_MS
    }
}

impl Manifest {
    fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|e| {
                eprintln!("Ignoring unreadable index manifest {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Written via a temp file + rename, so a crash mid-write can't leave a
    /// truncated manifest behind.
    fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        {
            let mut writer = io::BufWriter::new(std::fs::File::create(&tmp)?);
            serde_json::to_writer(&mut writer, self).map_err(io::Error::other)?;
            writer.flush()?;
        }
        std::fs::rename(&tmp, path)
    }
}

/// Whether a watcher event under `root` can affect the index: a note, or a
/// folder that might hold notes. Hidden paths (`.git`, editor swap folders)
/// are ignored, so a git pull's object writes don't each start a pass.
fn is_note_path(root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    let hidden = rel.components().any(|c| match c {
        Component::Normal(name) => name.to_string_lossy().starts_with('.'),
        _ => false,
    });
    if hidden {
        return false;
    }
    match rel.extension() {
        Some(ext) => ext == fs::DEFAULT_EXTENSION,
        None => true,
    }
}

/// Clears the running flag even if a pass panics partway through.
struct RunningGuard<'a>(&'a AtomicBool);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

pub struct Indexer {
    manifest_path: PathBuf,
    manifest: tokio::sync::Mutex<Manifest>,
    running: AtomicBool,
    /// Another pass was asked for while one was running.
    pending: AtomicBool,
    /// That pass should ignore the manifest and re-read every note.
    pending_force: AtomicBool,
    watcher: std::sync::Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

impl Indexer {
    pub fn new(manifest_path: PathBuf) -> Self {
        Self {
            manifest: tokio::sync::Mutex::new(Manifest::load(&manifest_path)),
            manifest_path,
            running: AtomicBool::new(false),
            pending: AtomicBool::new(false),
            pending_force: AtomicBool::new(false),
            watcher: std::sync::Mutex::new(None),
        }
    }

    pub fn is_indexing(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Starts a pass in the background, or queues one if a pass is already
    /// running; any number of requests made during a pass collapse into one
    /// more. Reports `index-progress`, then `index-complete` or `index-error`.
    /// `force` re-reads every note instead of trusting the manifest.
    pub fn request(&self, handle: &AppHandle, force: bool) {
        if force {
            self.pending_force.store(true, Ordering::SeqCst);
        }
        self.pending.store(true, Ordering::SeqCst);
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            let state = handle.state::<AppState>();
            let indexer = &state.indexer;
            loop {
                {
                    let _guard = RunningGuard(&indexer.running);
                    while indexer.pending.swap(false, Ordering::SeqCst) {
                        let force = indexer.pending_force.swap(false, Ordering::SeqCst);
                        match indexer.run_pass(&handle, force).await {
                            Ok(Some(stats)) => {
                                let _ = handle.emit("index-complete", stats);
                            }
                            Ok(None) => {}
                            Err(message) => {
                                let _ = handle.emit(
                                    "index-error",
                                    serde_json::json!({ "message": message }),
                                );
                            }
                        }
                    }
                }
                // A request that landed between the last check and the guard
                // clearing `running` saw a pass in progress and left it to us.
                if !indexer.pending.load(Ordering::SeqCst)
                    || indexer.running.swap(true, Ordering::SeqCst)
                {
                    break;
                }
            }
        });
    }

    /// One pass over the content directory. Returns `None` when AI is off and
    /// nothing was indexed.
    async fn run_pass(&self, handle: &AppHandle, force: bool) -> Result<Option<IndexStats>, String> {
        let state = handle.state::<AppState>();
        let content_dir = {
            let config = state.config.lock().await;
            if !config.ai_enabled {
                return Ok(None);
            }
            config.content_directory.clone()
        };
        let memory = &state.memory;

        let files = fs::discover_files(&content_dir).map_err(|e| e.to_string())?;
        let on_disk: HashSet<&str> = files.iter().map(|f| f.name.as_str()).collect();

        let mut in_db: HashSet<String> = HashSet::new();
        for title in memory.document_titles().await.map_err(|e| e.to_string())? {
            if on_disk.contains(title.as_str()) {
                in_db.insert(title);
            } else {
                memory.delete_document(&title).await.map_err(|e| e.to_string())?;
            }
        }

        let mut manifest = self.manifest.lock().await;
        let mut dirty = false;
        if force || manifest.version != EMBED_VERSION {
            manifest.entries.clear();
            manifest.version = EMBED_VERSION.to_string();
            dirty = true;
        }
        let before = manifest.entries.len();
        manifest.entries.retain(|name, _| on_disk.contains(name.as_str()));
        dirty |= manifest.entries.len() != before;

        // A note missing from the database is re-indexed whatever the manifest
        // says, so a wiped or rebuilt database fills back up on the next pass.
        let changed: Vec<(&fs::File, i64, u64)> = files
            .iter()
            .filter_map(|file| {
                let meta = std::fs::metadata(&file.path).ok()?;
                let mtime_ms = meta.modified().map(millis).unwrap_or(0);
                let size = meta.len();
                let fresh = in_db.contains(&file.name)
                    && manifest
                        .entries
                        .get(&file.name)
                        .is_some_and(|e| e.matches(mtime_ms, size));
                (!fresh).then_some((file, mtime_ms, size))
            })
            .collect();

        let total = changed.len();
        for (done, (file, mtime_ms, size)) in changed.into_iter().enumerate() {
            let _ = handle.emit(
                "index-progress",
                serde_json::json!({ "done": done, "total": total, "current": file.name }),
            );
            let content = match file.read_content() {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Skipping '{}' while indexing: {e}", file.name);
                    continue;
                }
            };
            let hash = content_hash(&content);
            let same_text = in_db.contains(&file.name)
                && manifest.entries.get(&file.name).is_some_and(|e| e.hash == hash);
            if !same_text {
                // No lock on Memory: chat searches run alongside indexing.
                memory
                    .to_document_context(NoteDocument::from_parts(&file.name, &content))
                    .await;
            }
            manifest.entries.insert(
                file.name.clone(),
                Entry {
                    mtime_ms,
                    size,
                    hash,
                    checked_at_ms: millis(SystemTime::now()),
                },
            );
            dirty = true;
            if (done + 1) % SAVE_EVERY == 0 {
                self.save(&manifest);
            }
        }

        if total > 0 {
            let _ = handle.emit(
                "index-progress",
                serde_json::json!({ "done": total, "total": total, "current": null }),
            );
        }
        if dirty {
            self.save(&manifest);
        }
        drop(manifest);

        memory.index_stats().await.map(Some).map_err(|e| e.to_string())
    }

    /// A failed write only costs re-reading some notes next pass, so it is logged, not raised.
    fn save(&self, manifest: &Manifest) {
        if let Err(e) = manifest.save(&self.manifest_path) {
            eprintln!("Could not save index manifest: {e}");
        }
    }

    /// Watches `dir` recursively and requests a pass whenever notes in it
    /// change. Replaces any earlier watch.
    pub fn watch(&self, handle: &AppHandle, dir: &Path) -> Result<(), String> {
        // FSEvents reports resolved paths (/private/var/... for /var/...).
        let root = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let events_root = root.clone();
        let handle = handle.clone();
        let mut debouncer = new_debouncer(WATCH_DEBOUNCE, move |result: DebounceEventResult| {
            match result {
                Ok(events) if events.iter().any(|e| is_note_path(&events_root, &e.path)) => {
                    handle.state::<AppState>().indexer.request(&handle, false);
                }
                Ok(_) => {}
                Err(e) => eprintln!("Note watcher error: {e}"),
            }
        })
        .map_err(|e| e.to_string())?;
        debouncer
            .watcher()
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;
        *self.watcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(debouncer);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_paths_skip_hidden_and_other_files() {
        let root = Path::new("/notes");
        assert!(is_note_path(root, Path::new("/notes/Ideas.md")));
        assert!(is_note_path(root, Path::new("/notes/Novel/Ch 1/Scene.md")));
        // A deleted or renamed folder may have held notes.
        assert!(is_note_path(root, Path::new("/notes/Novel")));
        assert!(!is_note_path(root, Path::new("/notes/.git/objects/ab/cdef")));
        assert!(!is_note_path(root, Path::new("/notes/Novel/.hidden.md")));
        assert!(!is_note_path(root, Path::new("/notes/cover.png")));
        assert!(!is_note_path(root, Path::new("/elsewhere/Ideas.md")));
    }

    #[test]
    fn recently_written_files_are_not_trusted_by_mtime() {
        let settled = Entry { mtime_ms: 1_000, size: 10, hash: String::new(), checked_at_ms: 10_000 };
        assert!(settled.matches(1_000, 10));
        assert!(!settled.matches(1_001, 10));
        assert!(!settled.matches(1_000, 11));

        let racy = Entry { checked_at_ms: 1_500, ..settled };
        assert!(!racy.matches(1_000, 10));
    }

    #[test]
    fn manifest_roundtrips_and_tolerates_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db/index-manifest.json");
        assert!(Manifest::load(&path).entries.is_empty());

        let mut manifest = Manifest { version: EMBED_VERSION.to_string(), ..Manifest::default() };
        manifest.entries.insert(
            "a.md".into(),
            Entry { mtime_ms: 1, size: 2, hash: content_hash("x"), checked_at_ms: 3 },
        );
        manifest.save(&path).unwrap();
        let loaded = Manifest::load(&path);
        assert_eq!(loaded.version, EMBED_VERSION);
        assert_eq!(loaded.entries["a.md"], manifest.entries["a.md"]);

        std::fs::write(&path, "not json").unwrap();
        assert!(Manifest::load(&path).entries.is_empty());
    }
}
