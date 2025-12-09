use std::collections::HashMap;

use super::error::MemoryError;
use super::responses::Response;
use super::utils;
use chrono::Utc;
use kalosm::sound::ModelLoadingProgress;
use rbert::Bert;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::sql::Thing;
use surrealdb::Surreal;

// -------------------------------------------------------
//  GENERAL UTILS
// -------------------------------------------------------
const TABLE: &str = "documents";

/// Splits text by double newline (paragraphs).
/// This is more stable than fixed - character chunking for editors.
fn split_by_paragraph(text: &str) -> Vec<&str> {
    text.split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
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
}

// -------------------------------------------------------
//  Note DOCUMENTS and CHUNKS
// -------------------------------------------------------

/// Represents a note document with a title and body content.
/// This struct is used for storing and managing documents in the RAG memory system.
#[derive(Debug, Serialize, Deserialize)]
pub struct NoteDocument {
    id: Option<Thing>,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextChunk {
    id: Option<Thing>,
    pub parent: Thing,
    pub content: String,
    pub sequence: usize,
    content_hash: String,
    grammar_check: Option<String>,
    is_dirty: bool,
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
}

// -------------------------------------------------------
// MEMORY DOCUMENT ANALYSIS EXTENSION
// -------------------------------------------------------

/// Trait for converting a NoteDocument into a DocumentContext using embeddings.
/// This trait defines an asynchronous method to perform the conversion.
pub trait MemoryDocumentAnalysisExt {
    async fn to_document_context(
        &self,
        embedding_document: NoteDocument,
    ) -> Option<String>;
}

/// Implements the MemoryDocumentAnalysisExt trait for the Memory struct.
/// This implementation provides the logic to convert a note_document
/// into a DocumentContext by splitting the document into paragraphs,
/// generating hashes, and reconciling with existing chunks in the database.
/// It handles new, unchanged, and deleted paragraphs accordingly.
impl MemoryDocumentAnalysisExt for Memory {
    async fn to_document_context(
        &self,
        embedding_document: NoteDocument,
    ) -> Option<String> {
        let db = &self.db;

        let note_document: Option<NoteDocument> = db
            .create(TABLE)
            .content(NoteDocument {
                id: None,
                title: embedding_document.title.clone(),
                body: embedding_document.body.clone(),
            })
            .await
            .unwrap();

        let parent_id = note_document.as_ref().and_then(|doc| doc.id.clone());
        let new_segments = split_by_paragraph(&embedding_document.body);
        let sql = "SELECT * FROM chunk WHERE parent = $id";
        let mut response = db.query(sql).bind(("id", parent_id.clone())).await.unwrap();
        let existing_chunks: Vec<TextChunk> = response.take(0).unwrap();

        println!("Created Note Document: {:?}", note_document);
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

            if let Some(mut old_chunk) = old_chunk_map.remove(&new_hash) {
                // --- CASE A: UNCHANGED ---
                // We found a chunk in the DB with the exact same hash.
                // We preserve the 'grammar_check' and 'id'.
                // We only update 'sequence' (in case the paragraph moved up/down).

                if old_chunk.sequence != i {
                    // It moved position
                    // let _: Option<TextChunk> = db
                    //     .update(("chunk", old_chunk.id.as_ref().unwrap().id.clone()))
                    //     .merge(TextChunk {
                    //         id: old_chunk.id.clone(),
                    //         parent: parent_id,
                    //         content: segment.to_string(),
                    //         sequence: i,
                    //         content_hash: new_hash,
                    //         grammar_check: old_chunk.grammar_check, // PRESERVED!
                    //         is_dirty: false,
                    //     })
                    //     .await.unwrap();
                    println!(
                        "   [MOVED]  idx {}: '{}...' (Analysis Preserved)",
                        i, short_preview
                    );
                } else {
                    // Exact match, same position. Often we can skip writing to DB entirely here,
                    // but for safety we ensure the record is confirmed.
                    println!(
                        "   [MATCH]  idx {}: '{}...' (Skipping DB Write)",
                        i, short_preview
                    );
                }
            } else {
                // --- CASE B: NEW / EDITED ---
                // No hash match found. This is a new paragraph or an edit.
                // Create a new chunk and mark is_dirty = true.
                let _: Option<TextChunk> = db
                    .create("chunk")
                    .content(TextChunk {
                        id: None,
                        parent: parent_id.clone().unwrap(),
                        content: segment.to_string(),
                        sequence: i,
                        content_hash: new_hash,
                        grammar_check: None, // No analysis yet
                        is_dirty: true,      // NEEDS LLM
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
                println!(
                    "   [DELETED] idx {}: '{}...'",
                    unused_chunk.sequence,
                    unused_chunk
                        .content
                        .chars()
                        .take(20)
                        .collect::<String>()
                        .replace('\n', " ")
                );
                // delete unused chunks
                // let _: Option<TextChunk> = db.delete(("chunk", unused_chunk.id.unwrap().id)).await?;
            }
        }
        None
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
