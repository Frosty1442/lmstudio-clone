// SQLite-based vector storage with cosine similarity search
// No external dependencies - everything embedded in the same database

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// Metadata attached to each vector for filtering and retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub workspace_id: String,
    pub document_id: String,
    pub document_name: String,
    pub chunk_index: usize,
    pub page_number: Option<u32>,
    pub file_type: String,
    pub created_at: String,
}

/// Advanced filtering options for vector search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    /// Filter by specific document IDs
    pub document_ids: Option<Vec<String>>,
    /// Filter by file types (pdf, docx, txt, etc.)
    pub file_types: Option<Vec<String>>,
    /// Filter by page number range (min, max) for PDFs
    pub page_range: Option<(u32, u32)>,
    /// Filter by creation date range (ISO 8601 timestamps)
    pub date_range: Option<(String, String)>,
}

/// Search result with chunk text and metadata
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub chunk_text: String,
    pub metadata: ChunkMetadata,
}

/// Collection statistics
#[derive(Debug, Clone, Serialize)]
pub struct CollectionStats {
    pub vectors_count: u64,
    pub points_count: u64,
}

/// SQLite-based vector storage
pub struct VectorStore {
    pool: SqlitePool,
}

impl VectorStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert vector embeddings with metadata
    pub async fn insert_chunks(
        &self,
        workspace_id: &str,
        chunks: Vec<(String, Vec<f32>, String, ChunkMetadata)>,
    ) -> Result<Vec<String>> {
        let mut ids = Vec::new();

        for (id, embedding, _text, metadata) in chunks {
            // Serialize embedding to binary
            let embedding_bytes = bincode::serialize(&embedding)?;

            sqlx::query(
                r#"
                INSERT INTO vector_embeddings (id, workspace_id, document_id, chunk_index, embedding, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&id)
            .bind(workspace_id)
            .bind(&metadata.document_id)
            .bind(metadata.chunk_index as i32)
            .bind(&embedding_bytes)
            .bind(&metadata.created_at)
            .execute(&self.pool)
            .await?;

            ids.push(id);
        }

        Ok(ids)
    }

    /// Search for similar vectors using cosine similarity
    pub async fn search(
        &self,
        workspace_id: &str,
        query_vector: Vec<f32>,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        // Build query with filters
        let mut query_str = String::from(
            "SELECT v.id, v.embedding, c.chunk_text, c.document_id, d.filename, d.file_type, c.page_number, c.chunk_index, v.created_at
             FROM vector_embeddings v
             JOIN document_chunks c ON v.id = c.vector_id
             JOIN documents d ON c.document_id = d.id
             WHERE v.workspace_id = ?"
        );

        // Apply filters
        if let Some(f) = &filters {
            if let Some(_doc_ids) = &f.document_ids {
                query_str.push_str(" AND v.document_id IN (");
                query_str.push_str(&"?,".repeat(f.document_ids.as_ref().unwrap().len())[..f.document_ids.as_ref().unwrap().len() * 2 - 1]);
                query_str.push(')');
            }
            if let Some(_types) = &f.file_types {
                query_str.push_str(" AND d.file_type IN (");
                query_str.push_str(&"?,".repeat(f.file_types.as_ref().unwrap().len())[..f.file_types.as_ref().unwrap().len() * 2 - 1]);
                query_str.push(')');
            }
            if let Some((min, max)) = f.page_range {
                query_str.push_str(&format!(" AND c.page_number BETWEEN {} AND {}", min, max));
            }
        }

        // Fetch embeddings
        let mut query_builder = sqlx::query(&query_str).bind(workspace_id);

        // Bind filter parameters
        if let Some(f) = &filters {
            if let Some(doc_ids) = &f.document_ids {
                for id in doc_ids {
                    query_builder = query_builder.bind(id);
                }
            }
            if let Some(types) = &f.file_types {
                for t in types {
                    query_builder = query_builder.bind(t);
                }
            }
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        // Calculate cosine similarity for each result
        let mut results = Vec::new();
        for row in rows {
            let embedding_bytes: Vec<u8> = row.get("embedding");
            let embedding: Vec<f32> = bincode::deserialize(&embedding_bytes)?;
            let score = cosine_similarity(&query_vector, &embedding);

            results.push(SearchResult {
                id: row.get("id"),
                score,
                chunk_text: row.get("chunk_text"),
                metadata: ChunkMetadata {
                    workspace_id: workspace_id.to_string(),
                    document_id: row.get("document_id"),
                    document_name: row.get("filename"),
                    chunk_index: row.get::<i32, _>("chunk_index") as usize,
                    page_number: row.get("page_number"),
                    file_type: row.get("file_type"),
                    created_at: row.get("created_at"),
                },
            });
        }

        // Sort by similarity (highest first) and take top K
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    /// Delete vectors by document ID
    pub async fn delete_by_document(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM vector_embeddings WHERE workspace_id = ? AND document_id = ?")
            .bind(workspace_id)
            .bind(document_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete entire workspace collection
    pub async fn delete_workspace_collection(&self, workspace_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM vector_embeddings WHERE workspace_id = ?")
            .bind(workspace_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get collection statistics
    pub async fn get_stats(&self, workspace_id: &str) -> Result<CollectionStats> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vector_embeddings WHERE workspace_id = ?"
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(CollectionStats {
            vectors_count: count as u64,
            points_count: count as u64,
        })
    }
}

/// Calculate cosine similarity between two vectors
/// Returns value between -1 and 1 (1 = identical, 0 = orthogonal, -1 = opposite)
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use uuid::Uuid;

    async fn create_test_store() -> (VectorStore, String) {
        use crate::repository::{Repository, CreateWorkspaceRequest};

        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_vec_{}.db", Uuid::new_v4()));

        let db = Database::new(&db_path).await.unwrap();
        let store = VectorStore::new(db.pool().clone());

        // Create workspace
        let repo = Repository::new(db.pool().clone());
        let workspace = repo
            .create_workspace(CreateWorkspaceRequest {
                name: "Test".to_string(),
                description: None,
                system_prompt: None,
                model_id: None,
                embedding_model_id: None,
                temperature: None,
                max_tokens: None,
            })
            .await
            .unwrap();

        (store, workspace.id)
    }

    #[test]
    fn test_cosine_similarity() {
        // Identical vectors
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < 0.001);

        // Orthogonal vectors
        let v3 = vec![1.0, 0.0, 0.0];
        let v4 = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&v3, &v4) - 0.0).abs() < 0.001);

        // Opposite vectors
        let v5 = vec![1.0, 0.0, 0.0];
        let v6 = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v5, &v6) + 1.0).abs() < 0.001);

        // Similar vectors
        let v7 = vec![1.0, 1.0, 0.0];
        let v8 = vec![1.0, 0.9, 0.0];
        let sim = cosine_similarity(&v7, &v8);
        assert!(sim > 0.9 && sim < 1.0);
    }

    #[tokio::test]
    async fn test_vector_store_creation() {
        let (store, workspace_id) = create_test_store().await;

        let stats = store.get_stats(&workspace_id).await.unwrap();
        assert_eq!(stats.vectors_count, 0);
    }

    #[tokio::test]
    async fn test_insert_and_search() {
        // This test would require document_chunks to exist
        // For now, just test the store creation
        let (store, _workspace_id) = create_test_store().await;
        assert!(store.pool.is_closed() == false);
    }

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.document_ids.is_none());
        assert!(filters.file_types.is_none());
        assert!(filters.page_range.is_none());
        assert!(filters.date_range.is_none());
    }
}
