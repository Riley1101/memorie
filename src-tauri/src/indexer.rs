//! Keeps the note index (see `memory`) in step with the content directory.
//!
//! A manifest records, per note, the mtime and size it had and the hash of its
//! text when it was last indexed. A pass stats every note, reads only the ones
//! whose mtime or size moved, and re-embeds only those whose text actually
//! changed, so a pass over an unchanged folder is a directory walk. A file
//! watcher starts a pass whenever notes change on disk, whether the app saved
//! them or a git pull, Dropbox pull or another editor did; when only notes
//! changed, that pass checks just those files instead of walking the folder.

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

/// What a batch of watcher events asks the indexer to look at.
#[derive(Debug, PartialEq, Eq)]
enum WatchChange {
    Nothing,
    /// Only these notes changed (created, edited, renamed or deleted).
    Notes(HashSet<PathBuf>),
    /// A folder changed, which may have moved or removed any number of notes.
    Everything,
}

fn classify_changes<'a>(root: &Path, paths: impl IntoIterator<Item = &'a Path>) -> WatchChange {
    let mut notes = HashSet::new();
    for path in paths.into_iter().filter(|p| is_note_path(root, p)) {
        if path.extension().is_none() {
            return WatchChange::Everything;
        }
        notes.insert(path.to_path_buf());
    }
    if notes.is_empty() {
        WatchChange::Nothing
    } else {
        WatchChange::Notes(notes)
    }
}

/// The note name (`Novel/Chapter 1/Scene.md`, as `fs::File::name` spells it) and
/// its folders for a path under `root`, or `None` for a path outside it.
fn note_name(root: &Path, path: &Path) -> Option<(String, Vec<String>)> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| match c {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let file_name = parts.pop()?;
    let name = if parts.is_empty() {
        file_name
    } else {
        format!("{}/{file_name}", parts.join("/"))
    };
    Some((name, parts))
}

/// Clears the running flag even if a pass panics partway through.
struct RunningGuard<'a>(&'a AtomicBool);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// What a pass looks at.
enum Scope {
    /// Every note in the content directory; `force` ignores the manifest.
    Full { force: bool },
    /// Just these notes, which may have been deleted since.
    Notes(HashSet<PathBuf>),
}

pub struct Indexer {
    manifest_path: PathBuf,
    manifest: tokio::sync::Mutex<Manifest>,
    running: AtomicBool,
    /// Another pass was asked for while one was running.
    pending: AtomicBool,
    /// That pass should walk the whole content directory.
    pending_full: AtomicBool,
    /// That pass should ignore the manifest and re-read every note.
    pending_force: AtomicBool,
    /// Notes the watcher saw change, for a pass that isn't full.
    pending_paths: std::sync::Mutex<HashSet<PathBuf>>,
    watcher: std::sync::Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

impl Indexer {
    pub fn new(manifest_path: PathBuf) -> Self {
        Self {
            manifest: tokio::sync::Mutex::new(Manifest::load(&manifest_path)),
            manifest_path,
            running: AtomicBool::new(false),
            pending: AtomicBool::new(false),
            pending_full: AtomicBool::new(false),
            pending_force: AtomicBool::new(false),
            pending_paths: std::sync::Mutex::new(HashSet::new()),
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
        self.pending_full.store(true, Ordering::SeqCst);
        self.start(handle);
    }

    /// Like `request`, but the pass only checks `paths` (notes under the content
    /// directory, which may since have been deleted), unless a full pass is also
    /// pending.
    fn request_notes(&self, handle: &AppHandle, paths: HashSet<PathBuf>) {
        self.pending_paths
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(paths);
        self.start(handle);
    }

    fn start(&self, handle: &AppHandle) {
        // Set after the pass's inputs, so a pass that sees it sees them too.
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
                        let full = indexer.pending_full.swap(false, Ordering::SeqCst);
                        let force = indexer.pending_force.swap(false, Ordering::SeqCst);
                        let paths = std::mem::take(
                            &mut *indexer.pending_paths.lock().unwrap_or_else(|e| e.into_inner()),
                        );
                        // A full pass covers any notes the watcher reported.
                        let scope = if full || force {
                            Scope::Full { force }
                        } else {
                            Scope::Notes(paths)
                        };
                        match indexer.run_pass(&handle, scope).await {
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

    /// One pass over the content directory, or over just the notes in `scope`.
    /// Returns `None` when AI is off and nothing was indexed.
    async fn run_pass(&self, handle: &AppHandle, scope: Scope) -> Result<Option<IndexStats>, String> {
        let state = handle.state::<AppState>();
        let content_dir = {
            let config = state.config.lock().await;
            if !config.ai_enabled {
                return Ok(None);
            }
            config.content_directory.clone()
        };
        let memory = &state.memory;

        let mut manifest = self.manifest.lock().await;
        // Notes indexed under another `EMBED_VERSION` all need a look, not just
        // the ones that happened to change.
        let scope = match scope {
            Scope::Notes(_) if manifest.version != EMBED_VERSION => Scope::Full { force: false },
            scope => scope,
        };
        let mut dirty = false;
        let mut files = Vec::new();
        let mut in_db: HashSet<String> = HashSet::new();

        match scope {
            Scope::Full { force } => {
                files = fs::discover_files(&content_dir).map_err(|e| e.to_string())?;
                let on_disk: HashSet<&str> = files.iter().map(|f| f.name.as_str()).collect();

                for title in memory.document_titles().await.map_err(|e| e.to_string())? {
                    if on_disk.contains(title.as_str()) {
                        in_db.insert(title);
                    } else {
                        memory.delete_document(&title).await.map_err(|e| e.to_string())?;
                    }
                }

                if force || manifest.version != EMBED_VERSION {
                    manifest.entries.clear();
                    manifest.version = EMBED_VERSION.to_string();
                    dirty = true;
                }
                let before = manifest.entries.len();
                manifest.entries.retain(|name, _| on_disk.contains(name.as_str()));
                dirty |= manifest.entries.len() != before;
            }
            Scope::Notes(paths) => {
                // Watcher paths are resolved (/private/var/... for /var/...).
                let root = content_dir.canonicalize().unwrap_or(content_dir);
                for path in paths {
                    let Some((name, dirs)) = note_name(&root, &path) else {
                        continue;
                    };
                    if path.is_file() {
                        if memory
                            .find_document_by_title(&name)
                            .await
                            .map_err(|e| e.to_string())?
                            .is_some()
                        {
                            in_db.insert(name);
                        }
                        files.push(fs::File::with_relative_dir(path, None, dirs));
                    } else {
                        memory.delete_document(&name).await.map_err(|e| e.to_string())?;
                        dirty |= manifest.entries.remove(&name).is_some();
                    }
                }
            }
        }

        // A note missing from the database is re-indexed whatever the manifest
        // says, so a wiped or rebuilt database fills back up on the next pass.
        let mut changed: Vec<(&fs::File, i64, u64)> = files
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
        // Most recently edited first, so what the user is working on becomes
        // searchable before the rest of a long first pass or a big pull.
        changed.sort_by_key(|&(_, mtime_ms, _)| std::cmp::Reverse(mtime_ms));

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
                // A note that fails stays out of the manifest, so the next pass retries it.
                if let Err(e) = memory
                    .to_document_context(NoteDocument::from_parts(&file.name, &content))
                    .await
                {
                    eprintln!("Could not index '{}': {e}", file.name);
                    continue;
                }
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
            let indexer = &handle.state::<AppState>().indexer;
            match result {
                Ok(events) => {
                    match classify_changes(&events_root, events.iter().map(|e| e.path.as_path())) {
                        WatchChange::Nothing => {}
                        WatchChange::Notes(paths) => indexer.request_notes(&handle, paths),
                        WatchChange::Everything => indexer.request(&handle, false),
                    }
                }
                // Events may have been lost, so check everything.
                Err(e) => {
                    eprintln!("Note watcher error: {e}");
                    indexer.request(&handle, false);
                }
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
    fn note_changes_are_targeted_and_folder_changes_are_not() {
        let root = Path::new("/notes");
        fn paths(list: &[&'static str]) -> Vec<&'static Path> {
            list.iter().map(|p| Path::new(*p)).collect()
        }

        assert_eq!(
            classify_changes(root, paths(&["/notes/.git/index", "/notes/cover.png"])),
            WatchChange::Nothing
        );
        assert_eq!(
            classify_changes(root, paths(&["/notes/a.md", "/notes/Novel/b.md", "/notes/a.md"])),
            WatchChange::Notes(
                [PathBuf::from("/notes/a.md"), PathBuf::from("/notes/Novel/b.md")].into()
            )
        );
        assert_eq!(
            classify_changes(root, paths(&["/notes/a.md", "/notes/Novel"])),
            WatchChange::Everything
        );
    }

    #[test]
    fn note_names_match_discovered_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Novel/Ch 1/Scene.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "x").unwrap();
        std::fs::write(dir.path().join("Top.md"), "y").unwrap();

        for file in fs::discover_files(dir.path()).unwrap() {
            let (name, dirs) = note_name(dir.path(), &file.path).unwrap();
            assert_eq!(name, file.name);
            assert_eq!(fs::File::with_relative_dir(file.path.clone(), None, dirs).name, file.name);
        }
        assert_eq!(note_name(dir.path(), Path::new("/elsewhere/a.md")), None);
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
