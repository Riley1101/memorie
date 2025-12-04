use super::error::MemoryError;
use super::utils;
use super::responses::Response;
use chrono::Utc;
use kalosm::language::{Document, DocumentTable, DocumentTableSurrealExt, EmbeddingId, SemanticChunker};
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;

const TABLE: &str = "documents";

#[derive(Serialize, Deserialize)]
pub struct EmbeddingDocument {
    pub id: EmbeddingId,
    pub title: String,
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatSession {
    pub job_id: String,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Memory {
    pub db: Surreal<Db>,
    pub document_table: DocumentTable<Db>,
}

pub trait UserMemory {
    async fn get_chat_sessions(&self) -> Result<Option<ChatSession>, MemoryError>;
    async fn save_chat_session(&self, session_id: &str) -> Result<Response<String>, MemoryError>;
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

        db.use_ns("embeddings_ns").use_db("embeddings").await?;

        let chunker = SemanticChunker::new();

        let document_table = db
            .document_table_builder(TABLE)
            .with_chunker(chunker)
            .at(root_dir.join("embeddings.db"))
            .build::<Document>()
            .await?;

        Ok(Memory { db, document_table })
    }

    /// Searches for documents in the memory that are relevant to the given query.
    /// Returns a vector of `EmbeddingDocument` containing the search results.
    ///  # Arguments
    ///  * `query` - The search query string.
    ///  * `limit` - The maximum number of results to return.
    ///  # Returns
    ///  A vector of `EmbeddingDocument` containing the search results.
    ///  # Errors
    ///  Returns a `MemoryError` if the search operation fails.
    pub async fn search_documents(&self, query: &str, limit: usize) -> Result<Response<Vec<EmbeddingDocument>>, MemoryError> {
        let table = &self.document_table;
        let context = table
            .search(&query)
            .with_results(limit)
            .await?
            .into_iter()
            .map(|document|{
                EmbeddingDocument{
                    id: document.id,
                    title: document.record.title().to_string(),
                    body: document.record.body().to_string(),
                }
            })
            .collect::<Vec<EmbeddingDocument>>();
        let response = Response::success(context);
        Ok(response)
    }

    /// Creates and stores embeddings for the given document name and content.
    /// # Arguments
    /// * `name` - The name of the document.
    /// * `content` - The content of the document.
    /// # Returns
    /// A success message indicating that the document was created.
    /// # Errors
    /// Returns a `MemoryError` if the insertion operation fails.
    pub async fn create_embeddings(&self, name:String, content:String) -> Result<Response<String>, MemoryError> {
        let table = &self.document_table;
        let document = Document::from_parts(name, content);
        let _= table.insert(document).await?;
        let response = Response::success("Document created".to_string());
        Ok(response)
    }
}

/// Implements user-specific memory operations for managing chat sessions.
/// This trait provides methods to retrieve and save chat sessions associated with a user.
impl UserMemory for Memory {
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
