use super::error::MemoryError;
use kalosm::language::{Document, DocumentTable, DocumentTableSurrealExt, SemanticChunker};
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexicalDoc {
    pub name: String,
    pub content: Option<String>,
}

const MEMORY_DB_PATH: &str = "/Users/arkar/.memorie/db/memory/";
const MEMORY_DB_VECTOR_STORE: &str = "/Users/arkar/.memorie/db/memory/embeddings.db";

const TABLE: &str = "documents";

// A wrapper around the document table for managing RAG memory.
pub struct Memory {
    db: Surreal<Db>,
    pub document_table: DocumentTable<Db>,
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
