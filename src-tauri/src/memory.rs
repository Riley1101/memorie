use super::{error::MemoryError, responses::Response, utils};
use chrono::Utc;
use kalosm::sound::ModelLoadingProgress;
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

/// Splits text by double newline (paragraphs).
/// This is more stable than fixed - character chunking for editors.
fn split_by_paragraph(text: &str) -> Vec<&str> {
    text.split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

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

/// Generates a SHA256 hash of the text content.
/// This acts as a unique fingerprint for the paragraph.
fn generate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
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
        match self.find_document_by_title(&title).await? {
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
                let embeddings = model
                    .embed(text)
                    .await?
                    .vector()
                    .into_iter()
                    .copied()
                    .collect();
                Ok(embeddings)
            }
            None => Err(MemoryError::ModelNotLoaded),
        }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub parent: Thing,
    pub content: String,
    pub sequence: usize,
    pub title: String,
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
        let db = &self.db;

        let query_embedding = self.generate_embedding(query).await?;
        let threshold = 0.3;

        let sql = r#"
    SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score 
    FROM chunk 
    WHERE embedding IS NOT NONE 
    AND vector::similarity::cosine(embedding, $query_vec) > $threshold
    ORDER BY score DESC, created_at DESC 
    LIMIT $limit
"#;
        let mut response = db
            .query(sql)
            .bind(("query_vec", query_embedding))
            .bind(("limit", limit))
            .bind(("threshold", threshold))
            .await?;

        let chunks: Vec<TextChunk> = response.take(0)?;

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
            .map(|c| {
                let title = doc_map
                    .get(&c.parent)
                    .cloned()
                    .unwrap_or_else(|| "Unknown Document".to_string());
                SearchResult {
                    parent: c.parent,
                    content: c.content,
                    sequence: c.sequence,
                    title,
                }
            })
            .collect();
        Ok(results)
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
            let new_hash = generate_hash(segment);
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
                let embedding = match self.generate_embedding(segment).await {
                    Ok(emb) => emb,
                    Err(e) => {
                        println!("Failed to generate embedding: {:?}", e);
                        vec![]
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
                        embedding: Some(embedding),
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
}

// -------------------------------------------------------
// USER CHAT SESSIONS
// -------------------------------------------------------

/// Represents a chat session associated with a user.
/// This struct contains the job ID, user ID, creation timestamp,
/// and last updated timestamp.
#[derive(Serialize, Deserialize)]
pub struct ChatSession {
    pub job_id: String,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

pub trait UserMemoryExt {
    async fn get_chat_sessions(&self) -> Result<Option<ChatSession>, MemoryError>;
    async fn save_chat_session(&self, session_id: &str) -> Result<Response<String>, MemoryError>;
}

/// Implements user-specific memory operations for managing chat sessions.
/// This trait provides methods to retrieve and save chat sessions associated with a user.
impl UserMemoryExt for Memory {
    /// Retrieves the chat sessions for the user.
    /// Returns an optional `ChatSession` if found, or `None` if no sessions exist.
    // TODO! Replace "root" with actual user ID
    async fn get_chat_sessions(&self) -> Result<Option<ChatSession>, MemoryError> {
        self.db.use_ns("user_ns").use_db("user").await?;
        let sessions: Option<ChatSession> = self.db.select(("chat_session", "root")).await?;
        Ok(sessions)
    }

    /// Saves a new chat session for the user with the provided session ID.
    /// # Arguments
    /// * `session_id` - The ID of the chat session to be saved.
    /// # Returns
    /// A success response indicating that the chat session was created.
    ///    TODO! Replace "root" with actual user ID
    async fn save_chat_session(&self, session_id: &str) -> Result<Response<String>, MemoryError> {
        self.db.use_ns("user_ns").use_db("user").await?;
        let _: Option<ChatSession> = self
            .db
            .create("chat_sessions")
            .content(ChatSession {
                job_id: session_id.to_string(),
                user_id: "root".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            })
            .await?;
        let response = Response::success("Chat Session Created".to_string());
        Ok(response)
    }
}
