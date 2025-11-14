use super::error::MemoryError;
use super::utils;
use chrono::Utc;
use kalosm::language::{Bert, Document, DocumentTable, DocumentTableSurrealExt, ModelLoadingProgress, SemanticChunker};
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;

const TABLE: &str = "documents";

#[derive(Serialize, Deserialize)]
pub struct ChatSession {
    pub job_id: String,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

// A wrapper around the document table for managing RAG memory.
pub struct Memory {
    db: Surreal<Db>,
    bert: Option<Bert>,
    pub document_table: DocumentTable<Db>,
}

pub trait UserMemory {
    async fn get_chat_sessions(&self) -> Result<Option<ChatSession>, MemoryError>;
    async fn save_chat_session(&self, session_id: &str) -> Result<(), MemoryError>;
}

pub trait EmbeddingMemory {
    fn document_table(&self) -> &DocumentTable<Db>;

    async fn download_model (&mut self) ->Result<(), MemoryError>;

    async fn generate_embeddings(&self) -> Result<(), MemoryError>;
}

impl Memory {
    /// Establishes a connection to the SurrealDB database and prepares the document table.
    ///
    /// This function initializes a local SurrealDB instance, sets the namespace,
    /// and builds a document table configured to store embeddings in a separate file.
    pub async fn new() -> Result<Self, MemoryError> {
        // TODO! Make the path configurable via app settings
        let root_dir = utils::get_app_dir()?.join("db/memory/");

        let db = Surreal::new::<SurrealKv>(root_dir.clone()).await?;

        db.use_ns("lexical_ns").use_db("files_db").await?;

        let chunker = SemanticChunker::new();

        let document_table = db
            .document_table_builder(TABLE)
            .with_chunker(chunker)
            .at(root_dir.join("db/memory/embeddings.db"))
            .build::<Document>()
            .await?;

        Ok(Memory { db,bert:None, document_table })
    }
}

impl EmbeddingMemory for Memory {
    fn document_table(&self) -> &DocumentTable<Db> {
        &self.document_table
    }
    async fn download_model (&mut self) -> Result<(), MemoryError> {
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
            .await.unwrap();
        self.bert = Some(bert);
        Ok(())
    }
}

impl UserMemory for Memory {
    /// Retrieves the chat sessions for the user.
    /// /// Returns an optional `ChatSession` if found, or `None` if no sessions exist.
    async fn get_chat_sessions(&self) -> Result<Option<ChatSession>, MemoryError> {
        self.db.use_ns("user_ns").use_db("user_ns").await?;
        let sessions: Option<ChatSession> = self.db.select(("chat_session", "me")).await?;
        Ok(sessions)
    }

    /// Saves a new chat session for the user with the provided session ID.
    /// /// Returns `Ok(())` if the session is saved successfully, or a `MemoryError` if an error occurs.
    async fn save_chat_session(&self, session_id: &str) -> Result<(), MemoryError> {
        self.db.use_ns("user_ns").use_db("user_ns").await?;
        let _: Option<ChatSession> = self
            .db
            .create("chat_sessions")
            .content(ChatSession {
                job_id: session_id.to_string(),
                user_id: "tobie".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            })
            .await?;
        Ok(())
    }
}
