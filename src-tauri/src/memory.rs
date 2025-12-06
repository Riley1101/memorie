use super::error::MemoryError;
use super::responses::Response;
use super::utils;
use chrono::Utc;
use kalosm::language::{
    Document, DocumentTable, DocumentTableSurrealExt, EmbeddingId, SemanticChunker,
};
use kalosm::sound::ModelLoadingProgress;
use rbert::Bert;
use rbert::EmbedderExt;
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::sql::Thing;
use surrealdb::Surreal;

const TABLE: &str = "documents";

#[derive(Serialize, Deserialize)]
pub struct EmbeddingDocument {
    pub distance: f32,
    pub id: EmbeddingId,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QueryResult {
    pub id: Thing,
    pub object: Document,
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

        let bert = Bert::builder()
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

        let chunker = SemanticChunker::new().with_target_score(90 as f32);

        let document_table = db
            .document_table_builder(TABLE)
            .with_embedding_model(bert)
            .with_chunker(chunker)
            .at(root_dir.join("embeddings.db"))
            .build::<Document>()
            .await?;

        Ok(Memory { db, document_table })
    }

    /// Finds a document in the memory by its title.
    /// Returns a vector of `QueryResult` containing the matching documents.
    /// # Arguments
    ///  * `title` - The title of the document to search for.
    ///  # Returns
    ///  A vector of `QueryResult` containing the matching documents.
    ///  # Errors
    ///  Returns a `MemoryError` if the search operation fails.
    pub async fn find_document_by_title(
        &self,
        title: String,
    ) -> Result<Option<QueryResult>, MemoryError> {
        let db = &self.db;
        let mut response = db
            .query("SELECT * FROM documents where object.title = $title LIMIT 1")
            .bind(("title", title))
            .await?;
        let mut result: Vec<QueryResult> = response.take(0)?;
        let first = result.pop();
        Ok(first)
    }

    /// Creates or updates a document in the memory with the given title and body.
    /// If a document with the specified title already exists, it is updated; otherwise,  
    /// a new document is created.
    /// # Arguments
    /// * `title` - The title of the document.
    /// * `body` - The body content of the document.
    /// # Returns
    /// A success response indicating that the document was created or updated.
    /// # Errors
    /// Returns a `MemoryError` if the create or update operation fails.
    pub async fn create_or_update_document(
        &self,
        title: &str,
        body: &str,
    ) -> Result<Response<String>, MemoryError> {
        let document = Document::from_parts(title.to_string(), body);
        match self.find_document_by_title(title.to_string()).await? {
            Some(result) => {
                let id = result.id.to_string();
                let query = format!("UPDATE {} SET object = $document", id);
                self.db
                    .query(query)
                    .bind(("document", document))
                    .await
                    .map_err(|e| MemoryError::DocumentUpdateInsertError(e.to_string()))?;
                Ok(Response::success("Document Updated".to_string()))
            }
            None => {
                let _ = self
                    .document_table
                    .insert(document)
                    .await
                    .map_err(|e| MemoryError::DocumentUpdateInsertError(e.to_string()))?;
                Ok(Response::success("Document Created".to_string()))
            }
        }
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
    pub async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Response<Vec<EmbeddingDocument>>, MemoryError> {
        let table = &self.document_table;
        println!("Searching documents with query: {}", query);

        let user_question_embedding = table.embedding_model().embed(&query).await?;

        let context = table
            .search(user_question_embedding)
            .with_results(3)
            .await?
            .into_iter()
            .map(|document| EmbeddingDocument {
                distance: document.distance,
                id: document.id,
                title: document.record.title().to_string(),
                body: document.record.body().to_string(),
            })
            .collect::<Vec<EmbeddingDocument>>();
        println!("Found {} documents", context.len());
        let response = Response::success(context);
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
