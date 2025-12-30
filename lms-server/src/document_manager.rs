use crate::documents::{TextChunker, ChunkConfig, DocumentParser, DocumentType};
use crate::inference::InferenceEngine;
use crate::repository::Repository;
use crate::vector_store::{ChunkMetadata, VectorStore};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Document record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Document {
    pub id: String,
    pub workspace_id: String,
    pub filename: String,
    pub file_path: String,
    pub file_type: String,
    pub file_size: i64,
    pub page_count: Option<i32>,
    pub chunk_count: i32,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// Document chunk record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub page_number: Option<i32>,
    pub char_start: i32,
    pub char_end: i32,
    pub token_count: Option<i32>,
    pub vector_id: Option<String>,
    pub created_at: String,
}

/// Request to upload a document
#[derive(Debug, Clone, Deserialize)]
pub struct UploadDocumentRequest {
    pub filename: String,
    pub content: String, // Base64 encoded for binary files, plain text for .txt
    pub file_type: Option<String>,
}

/// Response from document upload
#[derive(Debug, Clone, Serialize)]
pub struct UploadDocumentResponse {
    pub document_id: String,
    pub chunks_created: usize,
    pub status: String,
}

/// Manages document uploads and processing
pub struct DocumentManager {
    pool: SqlitePool,
    repository: Repository,
    vector_store: Arc<VectorStore>,
    inference_engine: Arc<InferenceEngine>,
    text_chunker: TextChunker,
    storage_path: PathBuf,
}

impl DocumentManager {
    pub fn new(
        pool: SqlitePool,
        repository: &Repository,
        vector_store: Arc<VectorStore>,
        inference_engine: Arc<InferenceEngine>,
        storage_path: PathBuf,
    ) -> Self {
        std::fs::create_dir_all(&storage_path).ok();

        Self {
            pool,
            repository: repository.clone(),
            vector_store,
            inference_engine,
            text_chunker: TextChunker::new(ChunkConfig::default()),
            storage_path,
        }
    }

    /// Upload and process a document
    pub async fn upload_document(
        &self,
        workspace_id: &str,
        embedding_model_id: &str,
        request: UploadDocumentRequest,
    ) -> Result<UploadDocumentResponse> {
        info!(
            "Uploading document {} to workspace {}",
            request.filename, workspace_id
        );

        // Generate document ID
        let document_id = Uuid::new_v4().to_string();
        let file_type = request
            .file_type
            .unwrap_or_else(|| self.detect_file_type(&request.filename));

        // Save file to storage
        let file_path = self.save_file(&document_id, &request.content).await?;

        // Create document record (status: processing)
        let file_size = request.content.len() as i64;
        self.repository
            .create_document(
                &document_id,
                workspace_id,
                &request.filename,
                &file_path,
                &file_type,
                file_size,
            )
            .await?;

        // Process document in background (parse + chunk + embed + store)
        match self
            .process_document(
                &document_id,
                workspace_id,
                embedding_model_id,
                &request.content,
                &request.filename,
                &file_type,
            )
            .await
        {
            Ok((chunk_count, page_count)) => {
                // Update status to ready
                self.repository
                    .update_document_status(&document_id, "ready", None, Some(chunk_count as i32))
                    .await?;

                // Update page count if available
                if let Some(pages) = page_count {
                    sqlx::query("UPDATE documents SET page_count = ? WHERE id = ?")
                        .bind(pages as i32)
                        .bind(&document_id)
                        .execute(&self.pool)
                        .await?;
                }

                Ok(UploadDocumentResponse {
                    document_id,
                    chunks_created: chunk_count,
                    status: "ready".to_string(),
                })
            }
            Err(e) => {
                warn!("Failed to process document {}: {}", document_id, e);
                self.repository
                    .update_document_status(&document_id, "error", Some(&e.to_string()), None)
                    .await?;

                Err(e)
            }
        }
    }

    /// Save file content to storage
    async fn save_file(&self, document_id: &str, content: &str) -> Result<String> {
        let file_path = self.storage_path.join(format!("{}.txt", document_id));
        tokio::fs::write(&file_path, content).await?;
        Ok(file_path.to_string_lossy().to_string())
    }

    /// Detect file type from filename
    fn detect_file_type(&self, filename: &str) -> String {
        if let Some(extension) = filename.rsplit('.').next() {
            // If there's no '.', rsplit returns the whole string
            // Check if the extension is different from the filename
            if extension != filename && !extension.is_empty() {
                return extension.to_lowercase();
            }
        }
        "txt".to_string()
    }

    /// Process document: parse, chunk, embed, store
    async fn process_document(
        &self,
        document_id: &str,
        workspace_id: &str,
        embedding_model_id: &str,
        content: &str,
        filename: &str,
        file_type: &str,
    ) -> Result<(usize, Option<usize>)> {  // Returns (chunk_count, page_count)
        // 1. Parse the document based on type
        info!("Parsing document {} (type: {})", document_id, file_type);
        let doc_type = DocumentType::from_filename(filename);
        let content_bytes = content.as_bytes();

        let parsed = DocumentParser::parse(content_bytes, doc_type)?;
        let page_count = parsed.page_count;
        info!(
            "Parsed document: {} pages, {} bytes of text",
            page_count.unwrap_or(0),
            parsed.text.len()
        );

        // 2. Chunk the extracted text
        info!("Chunking document {}", document_id);
        let chunks = self.text_chunker.chunk_text(&parsed.text)?;
        info!("Created {} chunks", chunks.len());

        if chunks.is_empty() {
            return Ok((0, page_count));
        }

        // 3. Generate embeddings for all chunks
        info!("Generating embeddings for {} chunks", chunks.len());
        let chunk_texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = self
            .inference_engine
            .generate_embeddings(embedding_model_id, chunk_texts)
            .await?;

        if embeddings.len() != chunks.len() {
            return Err(anyhow!(
                "Embedding count mismatch: expected {}, got {}",
                chunks.len(),
                embeddings.len()
            ));
        }

        // 4. Prepare chunks for vector store
        let created_at = Utc::now().to_rfc3339();
        let vector_chunks: Vec<(String, Vec<f32>, String, ChunkMetadata)> = chunks
            .iter()
            .zip(embeddings.iter())
            .map(|(chunk, embedding)| {
                let chunk_id = Uuid::new_v4().to_string();
                // Calculate page number from chunk position using page boundaries
                let page_number = parsed.get_page_for_position(chunk.start_pos);
                let metadata = ChunkMetadata {
                    workspace_id: workspace_id.to_string(),
                    document_id: document_id.to_string(),
                    document_name: filename.to_string(),
                    chunk_index: chunk.index,
                    page_number,
                    file_type: file_type.to_string(),
                    created_at: created_at.clone(),
                    char_start: chunk.start_pos,
                    char_end: chunk.end_pos,
                };

                (
                    chunk_id,
                    embedding.clone(),
                    chunk.text.clone(),
                    metadata,
                )
            })
            .collect();

        // 5. Insert into vector store
        info!("Inserting {} chunks into vector store", vector_chunks.len());
        let vector_ids = self
            .vector_store
            .insert_chunks(workspace_id, vector_chunks)
            .await?;

        // 6. Store chunk records in database
        info!("Storing chunk records in database");
        for (i, (chunk, vector_id)) in chunks.iter().zip(vector_ids.iter()).enumerate() {
            let chunk_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO document_chunks
                (id, document_id, chunk_index, chunk_text, char_start, char_end, vector_id, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(chunk_id)
            .bind(document_id)
            .bind(i as i32)
            .bind(&chunk.text)
            .bind(chunk.start_pos as i32)
            .bind(chunk.end_pos as i32)
            .bind(vector_id)
            .bind(&created_at)
            .execute(&self.pool)
            .await?;
        }

        Ok((chunks.len(), page_count))
    }

    /// Get document by ID
    pub async fn get_document(&self, document_id: &str) -> Result<Option<Document>> {
        let doc = sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id = ?")
            .bind(document_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(doc)
    }

    /// List documents in workspace
    pub async fn list_documents(&self, workspace_id: &str) -> Result<Vec<Document>> {
        let docs = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE workspace_id = ? ORDER BY created_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(docs)
    }

    /// Delete document and its chunks
    pub async fn delete_document(&self, workspace_id: &str, document_id: &str) -> Result<()> {
        // Delete from vector store
        self.vector_store
            .delete_by_document(workspace_id, document_id)
            .await?;

        // Delete from database (cascades to chunks)
        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(document_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    async fn create_test_document_manager() -> DocumentManager {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_docs_{}.db", Uuid::new_v4()));
        let storage_path = temp_dir.join(format!("test_storage_{}", Uuid::new_v4()));

        let db = Database::new(&db_path).await.unwrap();
        let repository = Repository::new(db.pool().clone());
        let vector_store = Arc::new(VectorStore::new(db.pool().clone()));
        let inference_engine = Arc::new(InferenceEngine::new());

        let manager = DocumentManager::new(
            db.pool().clone(),
            &repository,
            vector_store,
            inference_engine,
            storage_path.clone(),
        );

        manager
    }

    #[tokio::test]
    async fn test_detect_file_type() {
        let manager = create_test_document_manager().await;

        assert_eq!(manager.detect_file_type("test.txt"), "txt");
        assert_eq!(manager.detect_file_type("test.pdf"), "pdf");
        assert_eq!(manager.detect_file_type("test.docx"), "docx");
        assert_eq!(manager.detect_file_type("test.md"), "md");
        assert_eq!(manager.detect_file_type("noextension"), "txt");
    }

    #[tokio::test]
    async fn test_save_file() {
        let manager = create_test_document_manager().await;

        let doc_id = "test-doc-123";
        let content = "Hello, world!";

        let file_path = manager.save_file(doc_id, content).await.unwrap();
        assert!(file_path.contains(doc_id));

        let saved_content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(saved_content, content);

        // Cleanup
        tokio::fs::remove_file(&file_path).await.ok();
    }
}
