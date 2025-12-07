use super::error::MemoryError;
use super::responses::Response;
use super::utils;
use chrono::Utc;
use kalosm::language::{
    Document, DocumentTable, DocumentTableSurrealExt, EmbeddingId, SemanticChunker,
};
use kalosm::sound::ModelLoadingProgress;
use rbert::EmbedderExt;
use rbert::{Bert, Embedding};
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::sql::Thing;
use surrealdb::Surreal;

const TABLE: &str = "documents";

/// Represents a note document with a title and body content.
/// This struct is used for storing and managing documents in the RAG memory system.
#[derive(Serialize, Deserialize)]
pub struct NoteDocument {
    pub title: String,
    pub body: String,
}

/// Implements methods for the NoteDocument struct.
/// This implementation includes methods for creating a NoteDocument
impl NoteDocument {
    pub fn from_parts(title: &str, body: &str) -> Self {
        NoteDocument {
            title: title.to_string(),
            body: body.to_string(),
        }
    }

    pub async fn embedding(&self, model: &Bert) -> Option<Embedding> {
        if let Ok(embeddings) = model.embed(&self.body).await {
            return Some(embeddings);
        }
        None
    }
}

/// Represents the memory management system for RAG using SurrealDB.
/// This struct contains the database connection, document table,
/// and optional embedding model.
pub struct Memory {
    pub db: Surreal<Db>,
    pub document_table: DocumentTable<Db>,
    pub embedding_model: Option<Bert>,
}

/// Represents lines of context extracted from documents.
/// This struct contains the content of the document context.
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentContext {
    document_title: String,
    content: String,
    embedding: Option<Embedding>,
}

impl DocumentContext {
    /// Returns the content of the document context.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the title of the document.
    pub fn document_title(&self) -> &str {
        &self.document_title
    }

    /// Returns the embedding associated with the document context.
    pub fn embeddings(&self) -> &Option<Embedding> {
        &self.embedding
    }

    /// Creates a new DocumentContext from the given parts.
    pub fn from_parts(
        document_title: String,
        content: String,
        embedding: Option<Embedding>,
    ) -> Self {
        DocumentContext {
            document_title,
            content,
            embedding,
        }
    }
}

/// Trait for converting a NoteDocument into a DocumentContext using embeddings.
/// This trait defines an asynchronous method to perform the conversion.
pub trait MemoryDocumentContextExt {
    async fn to_document_context(&self, embedding_document: NoteDocument) -> DocumentContext;
}

/// Implements the MemoryDocumentContext trait for the Memory struct.
/// This implementation converts a NoteDocument into a MemoryDocumentContext
/// by generating embeddings using the BERT model.
impl MemoryDocumentContextExt for Memory {
    async fn to_document_context(&self, embedding_document: NoteDocument) -> DocumentContext {
        let model = self
            .embedding_model
            .as_ref()
            .expect("Embedding model not initialized");
        let embeddings = embedding_document.embedding(&model).await;
        DocumentContext::from_parts(
            embedding_document.title,
            embedding_document.body,
            embeddings,
        )
    }
}

/// Represents a document with its embedding information.
/// This struct contains the distance to the query, the document ID,
/// /// title, and body content.
#[derive(Serialize, Deserialize)]
pub struct EmbeddingDocument {
    pub distance: f32,
    pub id: EmbeddingId,
    pub title: String,
    pub body: String,
}

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

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct QueryResult {
    pub id: Thing,
    pub object: Document,
}

#[allow(dead_code)]
pub trait DocumentMemory {
    async fn find_document_by_title(
        &self,
        title: String,
    ) -> Result<Option<QueryResult>, MemoryError>;
    async fn create_or_update_document(
        &self,
        title: &str,
        body: &str,
    ) -> Result<Response<String>, MemoryError>;
    async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Response<Vec<EmbeddingDocument>>, MemoryError>;
}

#[allow(dead_code)]
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

        let chunker = SemanticChunker::new().with_target_score(90 as f32);

        let document_table = db
            .document_table_builder(TABLE)
            .with_chunker(chunker)
            .at(root_dir.join("embeddings.db"))
            .build::<Document>()
            .await?;

        Ok(Memory {
            db,
            document_table,
            embedding_model: Some(model),
        })
    }
}

/// Implements document-specific memory operations for managing documents in the RAG memory.
/// This trait provides methods to find, create/update, and search documents.
impl DocumentMemory for Memory {
    /// Finds a document in the memory by its title.
    /// Returns a vector of `QueryResult` containing the matching documents.
    /// # Arguments
    ///  * `title` - The title of the document to search for.
    ///  # Returns
    ///  A vector of `QueryResult` containing the matching documents.
    ///  # Errors
    ///  Returns a `MemoryError` if the search operation fails.
    async fn find_document_by_title(
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
    async fn create_or_update_document(
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
    async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Response<Vec<EmbeddingDocument>>, MemoryError> {
        let table = &self.document_table;
        println!("Searching documents with query: {}", query);

        let user_question_embedding = table.embedding_model().embed(&query).await?;

        let context = table
            .search(user_question_embedding)
            .with_results(limit)
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
