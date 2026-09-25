use super::chunker::{chunk_markdown, Chunk};
use super::reranker::Reranker;
use super::{error::MemoryError, utils};
use chrono::Utc;
use kalosm::language::ModelLoadingProgress;
use rbert::{Bert, EmbedderExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::sql::Thing;
use surrealdb::Surreal;

// -------------------------------------------------------
//  GENERAL UTILS
// -------------------------------------------------------
const DOCUMENT_TABLE: &str = "documents";
const CHUNK_TABLE: &str = "chunk";

/// Bumped whenever what gets embedded for a chunk changes. It is mixed into the chunk
/// hash, so existing chunks stop matching and are re-embedded on the next index pass.
pub const EMBED_VERSION: &str = "v3-sections";

/// Chunks scoring below this cosine similarity are never considered a match.
const MIN_SIMILARITY: f32 = 0.3;
/// Chunks more than this far below the best hit are dropped, so one strong match
/// doesn't get padded out with loosely related paragraphs.
const RELATIVE_SIMILARITY_WINDOW: f32 = 0.12;

/// Generates a SHA256 fingerprint of everything embedded for a chunk except the
/// note's title (so renaming a note doesn't re-embed it), versioned by `EMBED_VERSION`.
fn chunk_hash(chunk: &Chunk) -> String {
    let mut hasher = Sha256::new();
    hasher.update(EMBED_VERSION);
    for part in [chunk.headings.join(SECTION_SEP), chunk.overlap.clone(), chunk.text.clone()] {
        hasher.update(":");
        hasher.update(part.len().to_le_bytes());
        hasher.update(part);
    }
    hex::encode(hasher.finalize())
}

const SECTION_SEP: &str = " › ";

/// What gets embedded for a chunk: where it sits (note and headings), the end of
/// the passage before it, then the passage itself.
fn embed_input(title: &str, chunk: &Chunk) -> String {
    let mut out = title.to_string();
    for heading in &chunk.headings {
        out.push_str(SECTION_SEP);
        out.push_str(heading);
    }
    out.push_str("\n\n");
    if !chunk.overlap.is_empty() {
        out.push_str(&chunk.overlap);
        out.push(' ');
    }
    out.push_str(&chunk.text);
    out
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
    /// Set once the HNSW index exists. Without it the nearest-neighbour operator
    /// quietly returns nothing, so searches must scan instead.
    pub vector_index: AtomicBool,
    /// Loaded on first use; `None` inside if it couldn't be (offline on first
    /// run, say), in which case searches keep their fused order.
    pub reranker: tokio::sync::OnceCell<Option<Arc<Reranker>>>,
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

        let memory = Memory {
            db,
            embedding_model: Some(model),
            vector_index: AtomicBool::new(false),
            reranker: tokio::sync::OnceCell::new(),
        };
        // Search still works without the indexes (by scanning), so a failure here
        // is logged rather than keeping the app from starting.
        let dimension = match memory.generate_embedding("dimension probe").await {
            Ok(v) if !v.is_empty() => Some(v.len()),
            _ => None,
        };
        if let Err(e) = memory.ensure_indexes(dimension).await {
            eprintln!("Could not set up search indexes: {e}");
        }
        Ok(memory)
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

    /// Embeds a search query. bge-small-en-v1.5 was trained with this instruction in
    /// front of short queries (and nothing in front of passages); rbert's preset for
    /// it doesn't set one, so it is added here. Passages are embedded with
    /// `generate_embedding`, so changing this never requires re-indexing.
    pub async fn generate_query_embedding(&self, query: &str) -> Result<Vec<f32>, MemoryError> {
        self.generate_embedding(&format!("{BGE_QUERY_PREFIX}{query}")).await
    }

    /// The reranker, loading (and on first run downloading) it if needed. The
    /// app warms this up at startup so the first search doesn't wait on it.
    pub async fn reranker(&self) -> Option<Arc<Reranker>> {
        self.reranker
            .get_or_init(|| async {
                let _gpu = crate::gpu::lock().await;
                match Reranker::load().await {
                    Ok(reranker) => Some(Arc::new(reranker)),
                    Err(e) => {
                        eprintln!("Reranker unavailable, keeping search order: {e}");
                        None
                    }
                }
            })
            .await
            .clone()
    }

    /// Reorders `results` by the cross-encoder's judgement of how well each answers
    /// `query` and keeps the best `limit`. Without a reranker, or if it fails, the
    /// incoming order is kept.
    async fn rerank(&self, query: &str, mut results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
        if results.len() > 1 {
            if let Some(reranker) = self.reranker().await {
                // The reranker sees where a passage sits, as the embedding did.
                let passages: Vec<String> = results
                    .iter()
                    .map(|r| {
                        let mut place = pretty_title(&r.title);
                        if !r.section.is_empty() {
                            place.push_str(SECTION_SEP);
                            place.push_str(&r.section);
                        }
                        format!("{place}\n{}", r.content)
                    })
                    .collect();
                let query = query.to_string();
                let started = std::time::Instant::now();
                let _gpu = crate::gpu::lock().await;
                let scored = tokio::task::spawn_blocking(move || reranker.score(&query, &passages)).await;
                match scored {
                    Ok(Ok(scores)) if scores.len() == results.len() => {
                        for (result, score) in results.iter_mut().zip(scores) {
                            result.rerank_score = Some(score);
                        }
                        results.sort_by(|a, b| {
                            b.rerank_score.unwrap_or(f32::MIN).total_cmp(&a.rerank_score.unwrap_or(f32::MIN))
                        });
                        println!(">> Reranked {} passages in {:?}", results.len(), started.elapsed());
                    }
                    Ok(Ok(_)) => eprintln!("Reranker returned the wrong number of scores"),
                    Ok(Err(e)) => eprintln!("Reranking failed, keeping search order: {e}"),
                    Err(e) => eprintln!("Reranking failed, keeping search order: {e}"),
                }
            }
        }
        results.truncate(limit);
        results
    }

    /// Helper function to generate an embedding vector for a given text
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        match &self.embedding_model {
            Some(model) => {
                let _gpu = crate::gpu::lock().await;
                let embeddings = model.embed(text).await?.vector().to_vec();
                Ok(embeddings)
            }
            None => Err(MemoryError::ModelNotLoaded),
        }
    }

    /// Makes sure the search indexes exist: HNSW over chunk embeddings of `dimension`
    /// (the embedding model's output size) when it is known, and BM25 full-text over
    /// chunk text. Embeddings of any other length (empty ones from older versions, or
    /// another model's) can't go in the index, so they are cleared first; their notes
    /// are re-embedded when `EMBED_VERSION` changes.
    pub async fn ensure_indexes(&self, dimension: Option<usize>) -> Result<(), MemoryError> {
        let mut sql = String::from(
            "DEFINE ANALYZER IF NOT EXISTS note_text TOKENIZERS blank, class \
                 FILTERS lowercase, ascii, snowball(english); \
             DEFINE INDEX IF NOT EXISTS chunk_content_fts ON chunk FIELDS content \
                 SEARCH ANALYZER note_text BM25;",
        );
        if let Some(dimension) = dimension {
            // The dimension is part of the name, so a model with a different output
            // size gets a fresh index instead of failing every insert.
            sql.push_str(&format!(
                "UPDATE chunk SET embedding = NONE \
                     WHERE embedding IS NOT NONE AND array::len(embedding) != {dimension}; \
                 DEFINE INDEX IF NOT EXISTS chunk_embedding_{dimension} ON chunk \
                     FIELDS embedding HNSW DIMENSION {dimension} DIST COSINE;"
            ));
        }
        self.db.query(sql).await?.check()?;
        if dimension.is_some() {
            self.vector_index.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    /// Up to `k` embedded chunks nearest to `query_embedding`, best first, scored by
    /// cosine similarity. Uses the HNSW index when there is one, and compares against
    /// every chunk when there isn't or the index query fails.
    ///
    /// With `within`, only chunks of those notes are considered, always by comparing
    /// against each: the index finds the nearest `k` overall and filters afterwards,
    /// which would leave a binder whose passages aren't among them with nothing.
    async fn nearest_chunks(
        &self,
        query_embedding: &[f32],
        k: usize,
        within: Option<&[Thing]>,
    ) -> Result<Vec<ScoredChunk>, MemoryError> {
        let k = k.max(1);
        if let Some(ids) = within {
            let sql = format!(
                "SELECT {CHUNK_FIELDS}, vector::similarity::cosine(embedding, $query_vec) AS score \
                 FROM chunk WHERE parent IN $ids \
                     AND embedding IS NOT NONE AND array::len(embedding) > 0 \
                 ORDER BY score DESC LIMIT $limit"
            );
            let mut response = self
                .db
                .query(sql)
                .bind(("query_vec", query_embedding.to_vec()))
                .bind(("ids", ids.to_vec()))
                .bind(("limit", k))
                .await?;
            return Ok(response.take(0)?);
        }
        let ef = (k * 2).max(40);
        let knn = format!(
            "SELECT {CHUNK_FIELDS}, 1 - vector::distance::knn() AS score \
             FROM chunk WHERE embedding <|{k},{ef}|> $query_vec ORDER BY score DESC"
        );
        if self.vector_index.load(Ordering::SeqCst) {
            let indexed = async {
                let mut response =
                    self.db.query(knn).bind(("query_vec", query_embedding.to_vec())).await?;
                response.take::<Vec<ScoredChunk>>(0)
            }
            .await;
            match indexed {
                Ok(chunks) => return Ok(chunks),
                Err(e) => eprintln!("Vector index search failed, scanning all passages: {e}"),
            }
        }

        let scan = format!(
            "SELECT {CHUNK_FIELDS}, vector::similarity::cosine(embedding, $query_vec) AS score \
             FROM chunk WHERE embedding IS NOT NONE AND array::len(embedding) > 0 \
             ORDER BY score DESC LIMIT $limit"
        );
        let mut response = self
            .db
            .query(scan)
            .bind(("query_vec", query_embedding.to_vec()))
            .bind(("limit", k))
            .await?;
        Ok(response.take(0)?)
    }

    /// Up to `k` chunks containing any of the query's keywords, best BM25 match first.
    /// `score` is still the chunk's cosine similarity to the query (0 when it has no
    /// embedding), so results from both searches are reported on the same scale.
    async fn keyword_chunks(
        &self,
        query: &str,
        query_embedding: &[f32],
        k: usize,
        within: Option<&[Thing]>,
    ) -> Result<Vec<ScoredChunk>, MemoryError> {
        let terms = keyword_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        // `@@` needs every term to match, so each term gets its own clause and
        // reference; `search::score(n)` is 0 for a term that didn't match.
        let refs = 1..=terms.len();
        let matches = refs.clone().map(|n| format!("content @{n}@ $t{n}")).collect::<Vec<_>>();
        let scores = refs.map(|n| format!("search::score({n})")).collect::<Vec<_>>();
        let sql = format!(
            "SELECT {CHUNK_FIELDS}, {bm25} AS bm25, \
                 IF embedding IS NOT NONE AND array::len(embedding) > 0 \
                     THEN vector::similarity::cosine(embedding, $query_vec) ELSE 0 END AS score \
             FROM chunk WHERE ({matches}){scope} ORDER BY bm25 DESC LIMIT $limit",
            bm25 = scores.join(" + "),
            matches = matches.join(" OR "),
            scope = if within.is_some() { " AND parent IN $ids" } else { "" },
        );
        let mut request = self
            .db
            .query(sql)
            .bind(("query_vec", query_embedding.to_vec()))
            .bind(("limit", k));
        if let Some(ids) = within {
            request = request.bind(("ids", ids.to_vec()));
        }
        for (i, term) in terms.into_iter().enumerate() {
            request = request.bind((format!("t{}", i + 1), term));
        }
        Ok(request.await?.take(0)?)
    }

    /// Ranks embedded chunks by cosine similarity to `query_embedding`, keeping only
    /// strong matches close to the best one, each tagged with its note's title.
    /// The vector half of `search_hybrid`, kept for testing it on its own.
    #[cfg(test)]
    pub async fn search_by_embedding(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        let chunks = self.nearest_chunks(&query_embedding, limit, None).await?;
        self.with_titles(strong_matches(chunks)).await
    }

    /// Vector search and keyword search, merged by reciprocal rank fusion: a passage
    /// ranked high by either comes out near the top, and one found by both ranks
    /// highest. Keyword search catches names and rare words that embeddings blur;
    /// if it fails, the vector results are returned alone.
    ///
    /// With `binder`, only notes inside that top-level folder are searched.
    pub async fn search_hybrid(
        &self,
        query: &str,
        query_embedding: Vec<f32>,
        limit: usize,
        binder: Option<&str>,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        let within = match binder {
            Some(binder) => {
                let ids = self.binder_documents(binder).await?;
                if ids.is_empty() {
                    return Ok(Vec::new());
                }
                Some(ids)
            }
            None => None,
        };
        let within = within.as_deref();

        let pool = (limit * 3).max(24);
        let vector = strong_matches(self.nearest_chunks(&query_embedding, pool, within).await?);
        let keyword = match self.keyword_chunks(query, &query_embedding, pool, within).await {
            Ok(chunks) => strong_keyword_matches(chunks),
            Err(e) => {
                eprintln!("Keyword search failed, using vector results only: {e}");
                Vec::new()
            }
        };
        // Fuse a few more than asked for, so the reranker has room to reorder.
        let fused = fuse_ranked([vector, keyword], limit.max(RERANK_POOL));
        let results = self.with_titles(fused).await?;
        Ok(self.rerank(query, results, limit).await)
    }

    /// Ids of the notes inside `binder` (a top-level folder), at any depth.
    async fn binder_documents(&self, binder: &str) -> Result<Vec<Thing>, MemoryError> {
        let mut response = self
            .db
            .query("SELECT VALUE id FROM documents WHERE string::starts_with(title, $prefix)")
            .bind(("prefix", format!("{binder}/")))
            .await?;
        Ok(response.take(0)?)
    }

    /// Attaches each chunk's note title, dropping chunks whose note is gone.
    async fn with_titles(&self, chunks: Vec<ScoredChunk>) -> Result<Vec<SearchResult>, MemoryError> {
        let db = &self.db;
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
                    section: c.section.unwrap_or_default(),
                    start_line: c.start_line.unwrap_or_default(),
                    end_line: c.end_line.unwrap_or_default(),
                    found_by: c.found_by,
                    rerank_score: None,
                })
            })
            .collect();
        Ok(results)
    }
}

/// Query instruction for bge-small-en-v1.5; see `generate_query_embedding`.
const BGE_QUERY_PREFIX: &str = "Represent this sentence for searching relevant passages: ";

/// Candidates handed to the reranker per search.
const RERANK_POOL: usize = 16;

/// Chunk columns every search selects, alongside its own score.
const CHUNK_FIELDS: &str = "id, parent, content, sequence, section, start_line, end_line";

/// Reciprocal rank fusion constant: how much rank 1 beats rank 10. 60 is the
/// value from the original paper and the usual default.
const RRF_K: f32 = 60.0;

/// Most keywords taken from a query; each costs a full-text lookup.
const MAX_QUERY_TERMS: usize = 8;

/// Keyword hits scoring below this fraction of the best keyword hit are dropped,
/// the keyword-side counterpart of `RELATIVE_SIMILARITY_WINDOW`.
const MIN_RELATIVE_BM25: f32 = 0.3;

/// Words too common to say anything about which passage is meant.
const STOPWORDS: &[&str] = &[
    "a", "about", "an", "and", "are", "as", "at", "be", "but", "by", "did", "do", "does",
    "for", "from", "had", "has", "have", "he", "her", "his", "how", "i", "in", "is", "it",
    "its", "me", "my", "notes", "of", "on", "or", "our", "she", "so", "that", "the", "their",
    "them", "they", "this", "to", "was", "we", "were", "what", "when", "where", "which",
    "who", "why", "will", "with", "you", "your",
];

/// Distinct lowercase words of `query` worth matching on, in order.
fn keyword_terms(query: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() > 1 && !STOPWORDS.contains(&w.as_str()))
        .filter(|w| seen.insert(w.clone()))
        .take(MAX_QUERY_TERMS)
        .collect()
}

/// Keeps vector hits that clear `MIN_SIMILARITY` and sit within
/// `RELATIVE_SIMILARITY_WINDOW` of the best, so one strong match isn't padded out
/// with loosely related paragraphs. Expects best first.
fn strong_matches(chunks: Vec<ScoredChunk>) -> Vec<ScoredChunk> {
    let best = chunks.first().map(|c| c.score).unwrap_or(0.0);
    let cutoff = MIN_SIMILARITY.max(best - RELATIVE_SIMILARITY_WINDOW);
    chunks.into_iter().filter(|c| c.score >= cutoff).collect()
}

/// Keeps keyword hits within `MIN_RELATIVE_BM25` of the best. Expects best first.
fn strong_keyword_matches(chunks: Vec<ScoredChunk>) -> Vec<ScoredChunk> {
    let best = chunks.first().and_then(|c| c.bm25).unwrap_or(0.0);
    chunks
        .into_iter()
        .filter(|c| c.bm25.unwrap_or(0.0) >= best * MIN_RELATIVE_BM25)
        .collect()
}

/// Merges ranked lists by reciprocal rank fusion and keeps the top `limit`. Each
/// chunk's `found_by` records which lists it came from (bit `1 << n` for list `n`).
fn fuse_ranked<const N: usize>(lists: [Vec<ScoredChunk>; N], limit: usize) -> Vec<ScoredChunk> {
    let mut fused: Vec<(f32, ScoredChunk)> = Vec::new();
    let mut position: HashMap<Thing, usize> = HashMap::new();
    for (n, list) in lists.into_iter().enumerate() {
        let bit = 1u8 << n;
        for (rank, mut chunk) in list.into_iter().enumerate() {
            let points = 1.0 / (RRF_K + rank as f32 + 1.0);
            match position.get(&chunk.id) {
                Some(&i) => {
                    fused[i].0 += points;
                    fused[i].1.found_by |= bit;
                }
                None => {
                    chunk.found_by = bit;
                    position.insert(chunk.id.clone(), fused.len());
                    fused.push((points, chunk));
                }
            }
        }
    }
    fused.sort_by(|a, b| b.0.total_cmp(&a.0));
    fused.into_iter().take(limit).map(|(_, chunk)| chunk).collect()
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

    /// Headings above the passage joined with " › ", empty when it has none.
    #[serde(default)]
    pub section: String,
    /// Lines of the note the passage covers, 1-based.
    #[serde(default)]
    pub start_line: usize,
    #[serde(default)]
    pub end_line: usize,
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
    pub section: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Which searches found it: `FOUND_BY_MEANING`, `FOUND_BY_KEYWORDS`, or both.
    pub found_by: u8,
    /// The reranker's logit when it ran: above 0 usually means the passage answers.
    pub rerank_score: Option<f32>,
}

/// `SearchResult::found_by` bits.
pub const FOUND_BY_MEANING: u8 = 1;
pub const FOUND_BY_KEYWORDS: u8 = 2;

#[derive(Debug, Deserialize)]
struct ScoredChunk {
    id: Thing,
    parent: Thing,
    content: String,
    sequence: usize,
    score: f32,
    #[serde(default)]
    section: Option<String>,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
    /// Keyword search only: summed BM25 over the matched terms.
    #[serde(default)]
    bm25: Option<f32>,
    /// Set by `fuse_ranked`: bit `1 << n` for each input list it appeared in.
    #[serde(skip)]
    found_by: u8,
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
    /// Searches the notes in `binder`, or every note when `None`.
    async fn search_documents(
        &self,
        query: &str,
        limit: usize,
        binder: Option<&str>,
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
        binder: Option<&str>,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        let query_embedding = self.generate_query_embedding(query).await?;
        self.search_hybrid(query, query_embedding, limit, binder).await
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
        let new_segments = chunk_markdown(&embedding_document.body);
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
            let section = segment.headings.join(SECTION_SEP);
            let short_preview = segment
                .text
                .chars()
                .take(20)
                .collect::<String>()
                .replace('\n', " ");

            if let Some(old_chunk) = old_chunk_map.remove(&new_hash) {
                // Same text, but edits above it may have moved it.
                if old_chunk.sequence != i
                    || old_chunk.start_line != segment.start_line
                    || old_chunk.end_line != segment.end_line
                {
                    let _: Vec<TextChunk> = db
                        .upsert(CHUNK_TABLE)
                        .content(TextChunk {
                            id: old_chunk.id.clone(),
                            parent: parent_id.clone().unwrap(),
                            content: segment.text.clone(),
                            sequence: i,
                            content_hash: new_hash,
                            correction: old_chunk.correction,
                            is_dirty: false,
                            embedding: old_chunk.embedding,
                            created_at: old_chunk.created_at,
                            section,
                            start_line: segment.start_line,
                            end_line: segment.end_line,
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
                // The note title and headings are embedded with every chunk so a
                // question that names the note or section ("my dragon backstory
                // notes") still matches its paragraphs.
                let embedding = match self
                    .generate_embedding(&embed_input(&title, segment))
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
                        content: segment.text.clone(),
                        sequence: i,
                        content_hash: new_hash,
                        correction: None,
                        is_dirty: true,
                        embedding,
                        created_at: Utc::now().timestamp(),
                        section,
                        start_line: segment.start_line,
                        end_line: segment.end_line,
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
            vector_index: AtomicBool::new(false),
            reranker: tokio::sync::OnceCell::new_with(Some(None)),
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
                content_hash: content.to_string(),
                correction: None,
                is_dirty: false,
                embedding,
                created_at: 0,
                section: String::new(),
                start_line: 0,
                end_line: 0,
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

    async fn chunks_of(memory: &Memory, title: &str) -> Vec<TextChunk> {
        let id = memory.find_document_by_title(title).await.unwrap().unwrap().id.unwrap();
        let mut response = memory
            .db
            .query("SELECT * FROM chunk WHERE parent = $id ORDER BY sequence ASC")
            .bind(("id", id))
            .await
            .unwrap();
        response.take(0).unwrap()
    }

    #[tokio::test]
    async fn indexing_stores_sections_and_follows_moved_passages() {
        let memory = test_memory().await;
        let note = "---\nstatus: \"Done\"\n---\n\n# Part 1\n\nOpening.\n\n## Scene 2\n\nShe runs.\n";
        memory.to_document_context(NoteDocument::from_parts("Novel.md", note)).await;

        let chunks = chunks_of(&memory, "Novel.md").await;
        let got: Vec<(&str, &str, usize, usize)> = chunks
            .iter()
            .map(|c| (c.content.as_str(), c.section.as_str(), c.start_line, c.end_line))
            .collect();
        assert_eq!(
            got,
            vec![
                ("# Part 1\n\nOpening.", "Part 1", 5, 7),
                ("## Scene 2\n\nShe runs.", "Part 1 › Scene 2", 9, 11),
            ]
        );

        // A line added above moves both passages; they keep their records.
        let ids: Vec<_> = chunks.iter().map(|c| c.get_thing_id()).collect();
        let edited = note.replace("# Part 1", "Preface.\n\n# Part 1");
        memory.to_document_context(NoteDocument::from_parts("Novel.md", &edited)).await;

        let chunks = chunks_of(&memory, "Novel.md").await;
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "Preface.");
        assert_eq!((chunks[1].start_line, chunks[2].start_line), (7, 11));
        assert_eq!(vec![chunks[1].get_thing_id(), chunks[2].get_thing_id()], ids);
    }

    #[tokio::test]
    async fn indexed_search_clears_bad_embeddings_and_ranks_by_similarity() {
        let memory = test_memory().await;
        let doc = insert_document(&memory, "Notes.md").await;
        insert_chunk(&memory, &doc, "near", Some(vec![1.0, 0.0, 0.0])).await;
        insert_chunk(&memory, &doc, "close", Some(vec![0.95, 0.31, 0.0])).await;
        insert_chunk(&memory, &doc, "far", Some(vec![0.0, 1.0, 0.0])).await;
        insert_chunk(&memory, &doc, "legacy empty", Some(vec![])).await;
        insert_chunk(&memory, &doc, "other model", Some(vec![1.0, 0.0])).await;

        memory.ensure_indexes(Some(3)).await.unwrap();
        // Idempotent: runs on every launch.
        memory.ensure_indexes(Some(3)).await.unwrap();

        let results = memory.search_by_embedding(vec![1.0, 0.0, 0.0], 8).await.unwrap();
        let contents: Vec<&str> = results.iter().map(|r| r.content.as_str()).collect();
        assert_eq!(contents, vec!["near", "close"]);
        assert!((results[0].score - 1.0).abs() < 1e-4);

        // New chunks of the right size go into the index.
        insert_chunk(&memory, &doc, "newest", Some(vec![0.99, 0.1, 0.0])).await;
        let results = memory.search_by_embedding(vec![1.0, 0.0, 0.0], 8).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn hybrid_search_adds_keyword_hits_the_vectors_miss() {
        let memory = test_memory().await;
        memory.ensure_indexes(Some(3)).await.unwrap();
        let doc = insert_document(&memory, "World.md").await;
        insert_chunk(&memory, &doc, "The dragon sleeps under the mountain", Some(vec![1.0, 0.0, 0.0])).await;
        insert_chunk(&memory, &doc, "Zephyrine draws the maps", Some(vec![0.0, 1.0, 0.0])).await;
        insert_chunk(&memory, &doc, "Soup with carrots", Some(vec![0.0, 0.0, 1.0])).await;

        let results = memory
            .search_hybrid("Where does Zephyrine keep the dragon?", vec![1.0, 0.0, 0.0], 8, None)
            .await
            .unwrap();
        let contents: Vec<&str> = results.iter().map(|r| r.content.as_str()).collect();
        // Found by both searches, so first; then the name the embedding missed.
        assert_eq!(contents, vec!["The dragon sleeps under the mountain", "Zephyrine draws the maps"]);
        assert_eq!(results[0].found_by, FOUND_BY_MEANING | FOUND_BY_KEYWORDS);
        assert_eq!(results[1].found_by, FOUND_BY_KEYWORDS);
        // Scores stay cosine similarities, whichever search found the passage.
        assert!(results[1].score.abs() < 1e-4);
    }

    #[tokio::test]
    async fn hybrid_search_without_indexes_falls_back_to_vectors() {
        let memory = test_memory().await;
        let doc = insert_document(&memory, "Notes.md").await;
        insert_chunk(&memory, &doc, "match", Some(vec![1.0, 0.0])).await;
        insert_chunk(&memory, &doc, "keyword only", Some(vec![0.0, 1.0])).await;

        let results = memory.search_hybrid("keyword", vec![1.0, 0.0], 8, None).await.unwrap();
        let contents: Vec<&str> = results.iter().map(|r| r.content.as_str()).collect();
        assert_eq!(contents, vec!["match"]);
    }

    #[tokio::test]
    async fn binder_search_only_sees_that_binder() {
        let memory = test_memory().await;
        memory.ensure_indexes(Some(2)).await.unwrap();
        let novel = insert_document(&memory, "Novel/Part 1/Ch 3.md").await;
        let lore = insert_document(&memory, "Lore/Dragons.md").await;
        let loose = insert_document(&memory, "Novelettes.md").await;
        // Lore holds the passages nearest the query; the index would return only those.
        for i in 0..30 {
            insert_chunk(&memory, &lore, &format!("dragon lore {i}"), Some(vec![1.0, i as f32 * 0.001])).await;
        }
        insert_chunk(&memory, &novel, "the dragon attacks the keep", Some(vec![1.0, 0.3])).await;
        insert_chunk(&memory, &loose, "dragon in a loose note", Some(vec![1.0, 0.0])).await;

        let results = memory
            .search_hybrid("dragon keep", vec![1.0, 0.0], 8, Some("Novel"))
            .await
            .unwrap();
        let contents: Vec<&str> = results.iter().map(|r| r.content.as_str()).collect();
        assert_eq!(contents, vec!["the dragon attacks the keep"]);
        assert_eq!(results[0].title, "Novel/Part 1/Ch 3.md");

        let everywhere = memory.search_hybrid("dragon keep", vec![1.0, 0.0], 8, None).await.unwrap();
        assert!(everywhere.iter().any(|r| r.title == "Lore/Dragons.md"));

        let empty = memory.search_hybrid("dragon", vec![1.0, 0.0], 8, Some("Missing")).await.unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn keyword_terms_drop_stopwords_and_repeats() {
        assert_eq!(
            keyword_terms("What did Zephyrine say about the dragon's DRAGON hoard?"),
            vec!["zephyrine", "say", "dragon", "hoard"]
        );
        assert!(keyword_terms("what is it?").is_empty());
        assert_eq!(keyword_terms(&"word ".repeat(3)), vec!["word"]);
        let many: String = (0..20).map(|i| format!("term{i} ")).collect();
        assert_eq!(keyword_terms(&many).len(), MAX_QUERY_TERMS);
    }

    /// Two models embedding at once. Without `gpu::lock` this aborts the process
    /// on Apple Silicon (Metal); with it they take turns. Downloads bge-small on
    /// first run: `cargo test --release -- --ignored models_can_run_side_by_side`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn models_can_run_side_by_side() {
        let text = "She walked along the harbour wall. ".repeat(20);
        let mut jobs = Vec::new();
        for _ in 0..2 {
            let model = Bert::builder().build().await.unwrap();
            let text = text.clone();
            jobs.push(tokio::spawn(async move {
                for _ in 0..40 {
                    let _gpu = crate::gpu::lock().await;
                    model.embed(&text).await.unwrap();
                }
            }));
        }
        for job in jobs {
            job.await.unwrap();
        }
    }

    /// Embedding speed on this machine; downloads bge-small on first run.
    /// Meaningful only in release: `cargo test --release -- --ignored embedder_speed`.
    #[tokio::test]
    #[ignore]
    async fn embedder_speed() {
        // Holds the GPU throughout, as app code does, so it can run beside the other tests.
        let _gpu = crate::gpu::lock().await;
        let model = Bert::builder().build().await.unwrap();
        let passage = "She walked along the harbour wall, counting the boats. ".repeat(18);
        model.embed("warm up").await.unwrap();

        let started = std::time::Instant::now();
        model.embed("where did she walk?").await.unwrap();
        println!("1 query in {:?}", started.elapsed());

        let batch: Vec<String> = (0..32).map(|i| format!("Novel › Part {i}\n{passage}")).collect();
        let started = std::time::Instant::now();
        for text in &batch {
            model.embed(text).await.unwrap();
        }
        println!("32 full passages in {:?}", started.elapsed());
    }

    #[test]
    fn pretty_title_strips_extension_and_joins_folders() {
        assert_eq!(pretty_title("Novel/Part 1/Chapter 3.md"), "Novel › Part 1 › Chapter 3");
        assert_eq!(pretty_title("Ideas.md"), "Ideas");
    }
}


