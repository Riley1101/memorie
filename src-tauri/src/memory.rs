use super::error::MemoryError;
use chrono::Utc;
use kalosm::language::{Document, DocumentTable, DocumentTableSurrealExt, SemanticChunker};
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::{RecordId, Surreal};

const MEMORY_DB_PATH: &str = "/home/arkar/.memorie/db/memory/";
const MEMORY_DB_VECTOR_STORE: &str = "/home/arkar/.memorie/db/memory/embeddings.db";

const TABLE: &str = "documents";

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatSession {
    pub job_id: String,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

// A wrapper around the document table for managing RAG memory.
pub struct Memory {
    db: Surreal<Db>,
    pub document_table: DocumentTable<Db>,
}

pub trait UserMemory {
    async fn get_chat_sessions(&self) -> Result<Option<ChatSession>, MemoryError>;
    async fn save_chat_session(&self, session_id: &str) -> Result<(), MemoryError>;
}

impl Memory {
    /// Establishes a connection to the SurrealDB database and prepares the document table.
    ///
    /// This function initializes a local SurrealDB instance, sets the namespace,
    /// and builds a document table configured to store embeddings in a separate file.
    pub async fn new() -> Result<Self, MemoryError> {
        let db = Surreal::new::<SurrealKv>(MEMORY_DB_PATH).await?;

        db.use_ns("lexical_ns").use_db("files_db").await?;

        let chunker = SemanticChunker::new();

        let document_table = db
            .document_table_builder(TABLE)
            .with_chunker(chunker)
            .at(MEMORY_DB_VECTOR_STORE)
            .build::<Document>()
            .await?;

        Ok(Memory { db, document_table })
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
        let session: Option<ChatSession> = self
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
