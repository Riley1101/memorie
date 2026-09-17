use super::{error::MemoryError, utils};
use chrono::Utc;
use kalosm::language::ModelLoadingProgress;
use rbert::{Bert, EmbedderExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::sql::Thing;
use surrealdb::Surreal;

// -------------------------------------------------------
//  GENERAL UTILS
// -------------------------------------------------------
const DOCUMENT_TABLE: &str = "documents";
const CHUNK_TABLE: &str = "chunk";

const CHUNK_SIZE_LIMIT: usize = 1000;

/// Bumped whenever what gets embedded for a chunk changes. It is mixed into the chunk
/// hash, so existing chunks stop matching and are re-embedded on the next index pass.
const EMBED_VERSION: &str = "v2-title";

/// Chunks scoring below this cosine similarity are never considered a match.
const MIN_SIMILARITY: f32 = 0.3;
/// Chunks more than this far below the best hit are dropped, so one strong match
/// doesn't get padded out with loosely related paragraphs.
const RELATIVE_SIMILARITY_WINDOW: f32 = 0.12;

fn chunk_text(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    // Split by double newlines first to preserve paragraph structure
    let paragraphs = text.split("\n\n");

    for paragraph in paragraphs {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check if adding this paragraph would exceed the limit
        // We add 2 to account for the "\n\n" separator we might add
        if !current_chunk.is_empty()
            && (current_chunk.len() + trimmed.len() + 2 > CHUNK_SIZE_LIMIT)
        {
            chunks.push(current_chunk.clone());
            current_chunk.clear();
        }

        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
        }
        current_chunk.push_str(trimmed);
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

/// Generates a SHA256 fingerprint for a chunk, versioned by `EMBED_VERSION`.
fn chunk_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(EMBED_VERSION);
    hasher.update(":");
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Turns a content-relative file name like `Novel/Chapter 3.md` into a readable
/// title (`Novel › Chapter 3`) for embedding and for citing notes to the model.
pub fn pretty_title(name: &str) -> String {
    name.strip_suffix(".md")
        .unwrap_or(name)
        .split('/')
        .collect::<Vec<_>>()
        .join(" › ")
}

// -------------------------------------------------------
// MEMORY STRUCTS
// -------------------------------------------------------

/// Represents the memory management system for RAG using SurrealDB.
/// This struct contains the database connection, document table,
/// and optional embedding model.
pub struct Memory {
    pub db: Surreal<Db>,
    pub embedding_model: Option<Bert>,
}

/// Implements the Memory struct for managing RAG memory using SurrealDB.
/// This struct provides methods to connect to the database and handle document storage.
impl Memory {
    /// Establishes a connection to the SurrealDB database and prepares the document table.
    ///
    /// This function initializes a local SurrealDB instance, sets the namespace,
    /// and builds a document table configured to store embeddings in a separate file.
    pub async fn new() -> Result<Self, MemoryError> {
        // TODO! Make the path configurable via app settings
        let root_dir = utils::get_app_dir()?.join("db/memory/");
        let db = Surreal::new::<SurrealKv>(root_dir.clone()).await?;
        // TODO! Make the model configurable via app settings
        let model = Bert::builder()
            .build_with_loading_handler(|progress| match &progress {
                ModelLoadingProgress::Downloading {
                    source,
                    progress: file_loading_progress,
                } => {
                    let elapsed = file_loading_progress.start_time.elapsed().as_secs_f32();
                    let progress = (progress.progress() * 100.0) as u32;
                    println!("Downloading file {source} {progress}% ({elapsed}s)");
                }
                ModelLoadingProgress::Loading { progress } => {
                    let progress = (progress * 100.0) as u32;
                    println!("Loading model {progress}%");
                }
            })
            .await?;

        db.use_ns("embeddings_ns").use_db("embeddings").await?;

        Ok(Memory {
            db,
            embedding_model: Some(model),
        })
    }

    pub async fn find_text_chunk_by_id(
        &self,
        chunk_id: &str,
    ) -> Result<Option<TextChunk>, MemoryError> {
        let db = &self.db;
        let chunk: Option<TextChunk> = db.select((CHUNK_TABLE, chunk_id.to_string())).await?;
        Ok(chunk)
    }

    pub async fn find_document_by_title(
        &self,
        title: &str,
    ) -> Result<Option<NoteDocument>, MemoryError> {
        let db = &self.db;
        let mut response = db
            .query("SELECT * FROM documents WHERE title = $title LIMIT 1")
            .bind(("title", title.to_string()))
            .await?;
        let mut result: Vec<NoteDocument> = response.take(0)?;
        let first = result.pop();
        Ok(first)
    }

    pub async fn create_or_update_document(
        &self,
        title: &str,
        body: &str,
    ) -> Result<Option<NoteDocument>, MemoryError> {
        let db = &self.db;
        match self.find_document_by_title(title).await? {
            Some(mut result) => {
                println!(">> Updating existing document: {}", title);
                result.set_body(body);
                result.set_title(title);
                let mut result: Vec<NoteDocument> =
                    db.upsert(DOCUMENT_TABLE).content(result).await.unwrap();
                Ok(result.pop())
            }
            None => {
                println!(">> created a new doc: {}", title);
                let document = NoteDocument::from_parts(title, body);
                let note_document: Option<NoteDocument> =
                    db.create(DOCUMENT_TABLE).content(document).await?;
                Ok(note_document)
            }
        }
    }

    /// Helper function to generate an embedding vector for a given text
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        match &self.embedding_model {
            Some(model) => {
                let embeddings = model.embed(text).await?.vector().to_vec();
                Ok(embeddings)
            }
            None => Err(MemoryError::ModelNotLoaded),
        }
    }

    /// Ranks embedded chunks by cosine similarity to `query_embedding`, keeping only
    /// strong matches close to the best one, each tagged with its note's title.
    pub async fn search_by_embedding(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        let db = &self.db;

        let sql = r#"
    SELECT parent, content, sequence, vector::similarity::cosine(embedding, $query_vec) AS score
    FROM chunk
    WHERE embedding IS NOT NONE AND array::len(embedding) > 0
    ORDER BY score DESC
    LIMIT $limit
"#;
        let mut response = db
            .query(sql)
            .bind(("query_vec", query_embedding))
            .bind(("limit", limit))
            .await?;

        let chunks: Vec<ScoredChunk> = response.take(0)?;

        let best = chunks.first().map(|c| c.score).unwrap_or(0.0);
        let cutoff = MIN_SIMILARITY.max(best - RELATIVE_SIMILARITY_WINDOW);
        let chunks: Vec<ScoredChunk> = chunks.into_iter().filter(|c| c.score >= cutoff).collect();

        let mut parent_ids: Vec<Thing> = chunks.iter().map(|c| c.parent.clone()).collect();
        parent_ids.sort();
        parent_ids.dedup();

        let mut doc_map: HashMap<Thing, String> = HashMap::new();
        if !parent_ids.is_empty() {
            let sql_docs = "SELECT * FROM $ids";
            let mut doc_response = db.query(sql_docs).bind(("ids", parent_ids)).await?;
            let note_documents: Vec<NoteDocument> = doc_response.take(0)?;

            for doc in note_documents {
                if let Some(id) = doc.id {
                    doc_map.insert(id, doc.title);
                }
            }
        }

        let results: Vec<SearchResult> = chunks
            .into_iter()
            // A chunk whose parent note is gone is an orphan; never cite it.
            .filter_map(|c| {
                let title = doc_map.get(&c.parent).cloned()?;
                Some(SearchResult {
                    parent: c.parent,
                    content: c.content,
                    sequence: c.sequence,
                    title,
                    score: c.score,
                })
            })
            .collect();
        Ok(results)
    }
}

// -------------------------------------------------------
//  Note DOCUMENTS and CHUNKS
// -------------------------------------------------------

/// Represents a note document with a title and body content.
/// This struct is used for storing and managing documents in the RAG memory system.
#[derive(Debug, Serialize, Deserialize)]
pub struct NoteDocument {
    pub id: Option<Thing>,
    pub title: String,
    pub body: String,
}

/// Implements methods for the NoteDocument struct.
/// This implementation includes methods for creating a NoteDocument
impl NoteDocument {
    pub fn from_parts(title: &str, body: &str) -> Self {
        NoteDocument {
            id: None,
            title: title.to_string(),
            body: body.to_string(),
        }
    }

    pub fn set_body(&mut self, body: &str) {
        self.body = body.to_string();
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    pub fn get_thing_id(&self) -> Option<Thing> {
        self.id.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextChunk {
    id: Option<Thing>,
    pub parent: Thing,
    pub content: String,
    pub sequence: usize,
    content_hash: String,
    correction: Option<String>,
    pub is_dirty: bool,

    pub embedding: Option<Vec<f32>>,

    #[serde(default)]
    pub created_at: i64,
}

impl TextChunk {
    pub fn get_thing_id(&self) -> Option<Thing> {
        self.id.clone()
    }
}

/// A chunk matched by a notes search, with the title of the note it came from and
/// its cosine similarity to the query. Results are ordered best match first.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub parent: Thing,
    pub content: String,
    pub sequence: usize,
    pub title: String,
    pub score: f32,
}

#[derive(Debug, Deserialize)]
struct ScoredChunk {
    parent: Thing,
    content: String,
    sequence: usize,
    score: f32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct IndexStats {
    pub documents: usize,
    pub passages: usize,
}

#[derive(Debug, Deserialize)]
struct CountRow {
    n: usize,
}

// -------------------------------------------------------
// MEMORY DOCUMENT ANALYSIS EXTENSION
// -------------------------------------------------------

/// Trait for converting a NoteDocument into a DocumentContext using embeddings.
/// This trait defines an asynchronous method to perform the conversion.
pub trait MemoryDocumentAnalysisExt {
    async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, MemoryError>;
    async fn get_dirty_document_chunk(&self, document_id: Thing) -> Vec<TextChunk>;
    async fn to_document_context(&self, embedding_document: NoteDocument) -> Option<NoteDocument>;
    async fn update_dirty_chunk(&self, chunk: TextChunk, new_content: &str) -> Vec<TextChunk>;
    async fn rename_document(&self, old_title: &str, new_title: &str) -> Result<(), MemoryError>;
    /// Re-titles every document whose title starts with `old_prefix/` so it
    /// starts with `new_prefix/` instead (folder rename or move).
    async fn rename_document_prefix(
        &self,
        old_prefix: &str,
        new_prefix: &str,
    ) -> Result<(), MemoryError>;
    /// Removes a note and all of its embedded chunks, so deleted notes stop
    /// showing up in search results.
    async fn delete_document(&self, title: &str) -> Result<(), MemoryError>;
    async fn document_titles(&self) -> Result<Vec<String>, MemoryError>;
    async fn index_stats(&self) -> Result<IndexStats, MemoryError>;
}

/// Implements the MemoryDocumentAnalysisExt trait for the Memory struct.
/// This implementation provides the logic to convert a note_document
/// into a DocumentContext by splitting the document into paragraphs,
/// generating hashes, and reconciling with existing chunks in the database.
/// It handles new, unchanged, and deleted paragraphs accordingly.
impl MemoryDocumentAnalysisExt for Memory {
    async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        let query_embedding = self.generate_embedding(query).await?;
        self.search_by_embedding(query_embedding, limit).await
    }

    async fn update_dirty_chunk(&self, chunk: TextChunk, new_content: &str) -> Vec<TextChunk> {
        let db = &self.db;
        let mut updated_chunk = chunk;
        updated_chunk.content = new_content.to_string();
        updated_chunk.is_dirty = false;
        let updated: Vec<TextChunk> = db.upsert(CHUNK_TABLE).content(updated_chunk).await.unwrap();
        updated
    }

    async fn get_dirty_document_chunk(&self, document_id: Thing) -> Vec<TextChunk> {
        let db = &self.db;
        let sql =
            "SELECT * FROM chunk WHERE parent = $id AND is_dirty = true ORDER BY sequence ASC";
        let mut response = db.query(sql).bind(("id", document_id)).await.unwrap();
        response.take(0).unwrap()
    }

    async fn to_document_context(&self, embedding_document: NoteDocument) -> Option<NoteDocument> {
        let db = &self.db;

        let note_document = self
            .create_or_update_document(&embedding_document.title, &embedding_document.body)
            .await
            .unwrap();

        println!(
            ">> Starting Document Reconciliation for '{}'",
            embedding_document.title
        );

        let parent_id = note_document.as_ref().and_then(|doc| doc.id.clone());
        let title = pretty_title(&embedding_document.title);
        let new_segments = chunk_text(&embedding_document.body);
        let sql = "SELECT * FROM chunk WHERE parent = $id";
        let mut response = db.query(sql).bind(("id", parent_id.clone())).await.unwrap();
        let existing_chunks: Vec<TextChunk> = response.take(0).unwrap();

        let mut old_chunk_map: HashMap<String, TextChunk> = HashMap::new();

        for chunk in existing_chunks {
            old_chunk_map.insert(chunk.content_hash.clone(), chunk);
        }
        println!(
            ">> Reconciliation Start: {} new paragraphs vs {} existing",
            new_segments.len(),
            old_chunk_map.len()
        );

        for (i, segment) in new_segments.iter().enumerate() {
            let new_hash = chunk_hash(segment);
            let short_preview = segment
                .chars()
                .take(20)
                .collect::<String>()
                .replace('\n', " ");

            if let Some(old_chunk) = old_chunk_map.remove(&new_hash) {
                if old_chunk.sequence != i {
                    let _: Vec<TextChunk> = db
                        .upsert(CHUNK_TABLE)
                        .content(TextChunk {
                            id: old_chunk.id.clone(),
                            parent: parent_id.clone().unwrap(),
                            content: segment.to_string(),
                            sequence: i,
                            content_hash: new_hash,
                            correction: old_chunk.correction,
                            is_dirty: false,
                            embedding: old_chunk.embedding,
                            created_at: old_chunk.created_at,
                        })
                        .await
                        .unwrap();
                } else {
                    println!(
                        "   [MATCH]  idx {}: '{}...' (Skipping DB Write)",
                        i, short_preview
                    );
                }
            } else {
                // The note title is embedded with every chunk so a question that names
                // the note ("my dragon backstory notes") still matches its paragraphs.
                let embedding = match self
                    .generate_embedding(&format!("{title}\n\n{segment}"))
                    .await
                {
                    Ok(emb) if !emb.is_empty() => Some(emb),
                    Ok(_) => None,
                    Err(e) => {
                        println!("Failed to generate embedding: {:?}", e);
                        None
                    }
                };
                let _: Option<TextChunk> = db
                    .create(CHUNK_TABLE)
                    .content(TextChunk {
                        id: None,
                        parent: parent_id.clone().unwrap(),
                        content: segment.to_string(),
                        sequence: i,
                        content_hash: new_hash,
                        correction: None,
                        is_dirty: true,
                        embedding,
                        created_at: Utc::now().timestamp(),
                    })
                    .await
                    .unwrap();
                println!(
                    "   [NEW]    idx {}: '{}...' (Marked Dirty)",
                    i, short_preview
                );
            }
        }

        if !old_chunk_map.is_empty() {
            println!(
                ">> Cleaning up {} deleted paragraphs...",
                old_chunk_map.len()
            );
            for (_, unused_chunk) in old_chunk_map {
                let unused_chunkid = unused_chunk.get_thing_id().unwrap();
                println!(
                    "   [DELETED] idx {}: '{}...'",
                    unused_chunkid,
                    unused_chunk
                        .content
                        .chars()
                        .take(20)
                        .collect::<String>()
                        .replace('\n', " ")
                );

                let _: Option<TextChunk> = db
                    .delete((CHUNK_TABLE, unused_chunkid.id.to_string()))
                    .await
                    .unwrap();
                println!("      -> Deleted chunk ID: {:?}", unused_chunk);
            }
        }
        note_document
    }

    async fn rename_document(&self, old_title: &str, new_title: &str) -> Result<(), MemoryError> {
        let db = &self.db;
        let response = db
            .query("UPDATE documents SET title = $new_title WHERE title = $old_title")
            .bind(("old_title", old_title.to_string()))
            .bind(("new_title", new_title.to_string()))
            .await?;
        response.check()?;
        Ok(())
    }

    async fn rename_document_prefix(
        &self,
        old_prefix: &str,
        new_prefix: &str,
    ) -> Result<(), MemoryError> {
        let old_dir = format!("{old_prefix}/");
        let new_dir = format!("{new_prefix}/");
        let response = self
            .db
            .query(
                "UPDATE documents SET title = string::concat($new_dir, string::slice(title, string::len($old_dir))) WHERE string::starts_with(title, $old_dir)",
            )
            .bind(("old_dir", old_dir))
            .bind(("new_dir", new_dir))
            .await?;
        response.check()?;
        Ok(())
    }

    async fn delete_document(&self, title: &str) -> Result<(), MemoryError> {
        let Some(id) = self
            .find_document_by_title(title)
            .await?
            .and_then(|doc| doc.id)
        else {
            return Ok(());
        };
        let response = self
            .db
            .query("DELETE chunk WHERE parent = $id; DELETE $id;")
            .bind(("id", id))
            .await?;
        response.check()?;
        Ok(())
    }

    async fn document_titles(&self) -> Result<Vec<String>, MemoryError> {
        let mut response = self.db.query("SELECT VALUE title FROM documents").await?;
        let titles: Vec<String> = response.take(0)?;
        Ok(titles)
    }

    async fn index_stats(&self) -> Result<IndexStats, MemoryError> {
        let mut response = self
            .db
            .query(
                "SELECT count() AS n FROM documents GROUP ALL; \
                 SELECT count() AS n FROM chunk WHERE embedding IS NOT NONE AND array::len(embedding) > 0 GROUP ALL;",
            )
            .await?;
        let documents: Option<CountRow> = response.take(0)?;
        let passages: Option<CountRow> = response.take(1)?;
        Ok(IndexStats {
            documents: documents.map(|r| r.n).unwrap_or(0),
            passages: passages.map(|r| r.n).unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod memory_tests {
    use super::*;
    use surrealdb::engine::local::Mem;

    async fn test_memory() -> Memory {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        Memory {
            db,
            embedding_model: None,
        }
    }

    async fn insert_document(memory: &Memory, title: &str) -> Thing {
        memory
            .create_or_update_document(title, "body")
            .await
            .unwrap()
            .unwrap()
            .id
            .unwrap()
    }

    async fn insert_chunk(memory: &Memory, parent: &Thing, content: &str, embedding: Option<Vec<f32>>) {
        let _: Option<TextChunk> = memory
            .db
            .create(CHUNK_TABLE)
            .content(TextChunk {
                id: None,
                parent: parent.clone(),
                content: content.to_string(),
                sequence: 0,
                content_hash: chunk_hash(content),
                correction: None,
                is_dirty: false,
                embedding,
                created_at: 0,
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn search_ranks_matches_and_skips_unembedded_chunks() {
        let memory = test_memory().await;
        let dragons = insert_document(&memory, "Lore/Dragons.md").await;
        let recipes = insert_document(&memory, "Recipes.md").await;
        insert_chunk(&memory, &dragons, "dragon backstory", Some(vec![1.0, 0.0, 0.0])).await;
        insert_chunk(&memory, &dragons, "dragon diet", Some(vec![0.95, 0.31, 0.0])).await;
        insert_chunk(&memory, &recipes, "soup", Some(vec![0.0, 1.0, 0.0])).await;
        insert_chunk(&memory, &recipes, "failed embedding", None).await;
        insert_chunk(&memory, &recipes, "legacy empty embedding", Some(vec![])).await;

        let results = memory.search_by_embedding(vec![1.0, 0.0, 0.0], 8).await.unwrap();

        let contents: Vec<&str> = results.iter().map(|r| r.content.as_str()).collect();
        assert_eq!(contents, vec!["dragon backstory", "dragon diet"]);
        assert_eq!(results[0].title, "Lore/Dragons.md");
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn search_drops_hits_far_below_the_best_match() {
        let memory = test_memory().await;
        let doc = insert_document(&memory, "Notes.md").await;
        insert_chunk(&memory, &doc, "strong", Some(vec![1.0, 0.0])).await;
        insert_chunk(&memory, &doc, "loosely related", Some(vec![0.6, 0.8])).await;

        let results = memory.search_by_embedding(vec![1.0, 0.0], 8).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "strong");
    }

    #[tokio::test]
    async fn search_ignores_chunks_whose_note_was_deleted() {
        let memory = test_memory().await;
        let orphan_parent = Thing::from((DOCUMENT_TABLE, "gone"));
        insert_chunk(&memory, &orphan_parent, "orphan", Some(vec![1.0, 0.0])).await;

        let results = memory.search_by_embedding(vec![1.0, 0.0], 8).await.unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn delete_document_removes_the_note_and_only_its_chunks() {
        let memory = test_memory().await;
        let keep = insert_document(&memory, "Keep.md").await;
        let remove = insert_document(&memory, "Remove.md").await;
        insert_chunk(&memory, &keep, "kept", Some(vec![1.0])).await;
        insert_chunk(&memory, &remove, "removed a", Some(vec![1.0])).await;
        insert_chunk(&memory, &remove, "removed b", Some(vec![1.0])).await;

        memory.delete_document("Remove.md").await.unwrap();
        memory.delete_document("Never existed.md").await.unwrap();

        assert_eq!(memory.document_titles().await.unwrap(), vec!["Keep.md".to_string()]);
        let stats = memory.index_stats().await.unwrap();
        assert_eq!((stats.documents, stats.passages), (1, 1));
    }

    #[tokio::test]
    async fn index_stats_counts_only_embedded_passages() {
        let memory = test_memory().await;
        let empty = memory.index_stats().await.unwrap();
        assert_eq!((empty.documents, empty.passages), (0, 0));

        let doc = insert_document(&memory, "Doc.md").await;
        insert_chunk(&memory, &doc, "embedded", Some(vec![0.5, 0.5])).await;
        insert_chunk(&memory, &doc, "not embedded", None).await;
        insert_chunk(&memory, &doc, "legacy empty", Some(vec![])).await;

        let stats = memory.index_stats().await.unwrap();
        assert_eq!((stats.documents, stats.passages), (1, 1));
    }

    #[test]
    fn pretty_title_strips_extension_and_joins_folders() {
        assert_eq!(pretty_title("Novel/Part 1/Chapter 3.md"), "Novel › Part 1 › Chapter 3");
        assert_eq!(pretty_title("Ideas.md"), "Ideas");
    }
}
