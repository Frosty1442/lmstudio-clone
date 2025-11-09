use crate::inference::InferenceEngine;
use crate::vector_store::{SearchFilters, SearchResult, VectorStore};
use crate::workspace::WorkspaceManager;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Citation referencing a specific document chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Unique ID for this citation (e.g., "[1]")
    pub id: String,
    /// Document ID this chunk came from
    pub document_id: String,
    /// Document filename
    pub document_name: String,
    /// Page number (for PDFs)
    pub page_number: Option<u32>,
    /// Character position in original document
    pub char_range: (usize, usize),
    /// Relevance score from vector search
    pub score: f32,
    /// The actual chunk text
    pub text: String,
}

/// Complete RAG response with answer and citations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGResponse {
    /// The generated answer
    pub content: String,
    /// List of citations used
    pub citations: Vec<Citation>,
    /// Number of chunks retrieved
    pub chunks_retrieved: usize,
    /// Model used for generation
    pub model_id: String,
    /// Total tokens used
    pub tokens_used: Option<i32>,
}

/// Request for RAG query
#[derive(Debug, Clone, Deserialize)]
pub struct RAGRequest {
    /// The user's question
    pub query: String,
    /// Optional search filters
    pub filters: Option<SearchFilters>,
    /// Number of chunks to retrieve (default: 5)
    pub top_k: Option<usize>,
    /// Temperature for generation (overrides workspace default)
    pub temperature: Option<f32>,
    /// Max tokens for generation (overrides workspace default)
    pub max_tokens: Option<i32>,
}

/// RAG Engine - Handles retrieval augmented generation
pub struct RAGEngine {
    vector_store: Arc<VectorStore>,
    workspace_manager: Arc<WorkspaceManager>,
    inference_engine: Arc<InferenceEngine>,
}

impl RAGEngine {
    pub fn new(
        vector_store: Arc<VectorStore>,
        workspace_manager: Arc<WorkspaceManager>,
        inference_engine: Arc<InferenceEngine>,
    ) -> Self {
        Self {
            vector_store,
            workspace_manager,
            inference_engine,
        }
    }

    /// Generate embeddings for a query
    async fn embed_query(&self, query: &str, model_id: &str) -> Result<Vec<f32>> {
        let embeddings = self
            .inference_engine
            .generate_embeddings(model_id, vec![query.to_string()])
            .await?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }

    /// Retrieve relevant context chunks for a query
    pub async fn retrieve_context(
        &self,
        query: &str,
        workspace_id: &str,
        embedding_model_id: &str,
        top_k: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        // 1. Generate query embedding
        let query_vector = self.embed_query(query, embedding_model_id).await?;

        // 2. Search vector store
        let results = self
            .vector_store
            .search(workspace_id, query_vector, top_k, filters)
            .await?;

        Ok(results)
    }

    /// Build a prompt with retrieved context and citations
    pub fn build_prompt(
        &self,
        query: &str,
        context_chunks: &[SearchResult],
        system_prompt: Option<&str>,
    ) -> (String, Vec<Citation>) {
        let mut citations = Vec::new();
        let mut context_text = String::new();

        // Build context section with citations
        for (idx, result) in context_chunks.iter().enumerate() {
            let citation_id = format!("[{}]", idx + 1);

            // Add citation
            citations.push(Citation {
                id: citation_id.clone(),
                document_id: result.metadata.document_id.clone(),
                document_name: result.metadata.document_name.clone(),
                page_number: result.metadata.page_number,
                char_range: (0, 0), // TODO: Add char positions to ChunkMetadata if needed
                score: result.score,
                text: result.chunk_text.clone(),
            });

            // Add to context text
            context_text.push_str(&format!(
                "\n{} (Source: {}{}):\n{}\n",
                citation_id,
                result.metadata.document_name,
                result.metadata.page_number
                    .map(|p| format!(", page {}", p))
                    .unwrap_or_default(),
                result.chunk_text
            ));
        }

        // Build final prompt
        let system = system_prompt.unwrap_or(
            "You are a helpful AI assistant. Answer questions based on the provided context. \
             Always cite your sources using the citation IDs (e.g., [1], [2]) when referencing information. \
             If the context doesn't contain relevant information, say so clearly.",
        );

        let prompt = if context_chunks.is_empty() {
            format!(
                "{}\n\nUser: {}\n\nAssistant:",
                system, query
            )
        } else {
            format!(
                "{}\n\nContext:\n{}\n\nUser: {}\n\nAssistant:",
                system, context_text, query
            )
        };

        (prompt, citations)
    }

    /// Generate a response with citations (full RAG pipeline)
    pub async fn generate_with_citations(
        &self,
        workspace_id: &str,
        request: RAGRequest,
    ) -> Result<RAGResponse> {
        // 1. Get workspace configuration
        let workspace = self
            .workspace_manager
            .get(workspace_id)
            .await?
            .ok_or_else(|| anyhow!("Workspace not found: {}", workspace_id))?;

        // Determine which models to use
        let embedding_model = workspace
            .embedding_model_id
            .as_deref()
            .ok_or_else(|| anyhow!("No embedding model configured for workspace"))?;
        let chat_model = workspace
            .model_id
            .as_deref()
            .ok_or_else(|| anyhow!("No chat model configured for workspace"))?;

        let top_k = request.top_k.unwrap_or(5);

        // 2. Retrieve context chunks
        let context_chunks = self
            .retrieve_context(
                &request.query,
                workspace_id,
                embedding_model,
                top_k,
                request.filters,
            )
            .await?;

        // 3. Build prompt with citations
        let (prompt, citations) = self.build_prompt(
            &request.query,
            &context_chunks,
            workspace.system_prompt.as_deref(),
        );

        // 4. Generate response using LLM
        let temperature = request.temperature.unwrap_or(workspace.temperature);
        let max_tokens = request.max_tokens.unwrap_or(workspace.max_tokens);

        let response = self
            .inference_engine
            .generate_completion(chat_model, &prompt, temperature, max_tokens)
            .await?;

        // 5. Return complete RAG response
        Ok(RAGResponse {
            content: response.content,
            citations,
            chunks_retrieved: context_chunks.len(),
            model_id: chat_model.to_string(),
            tokens_used: response.tokens_used,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::workspace::{CreateWorkspaceRequest, WorkspaceManager};
    use std::path::PathBuf;

    async fn create_test_rag_engine() -> (RAGEngine, String) {
        // Create temporary database
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_rag_{}.db", uuid::Uuid::new_v4()));
        let db = Database::new(&db_path).await.unwrap();

        // Create workspace manager
        let workspace_manager = Arc::new(WorkspaceManager::new(db.pool().clone()));

        // Create test workspace
        let workspace = workspace_manager
            .create(CreateWorkspaceRequest {
                name: "Test Workspace".to_string(),
                description: Some("Test workspace for RAG".to_string()),
                system_prompt: Some("You are a test assistant.".to_string()),
                model_id: Some("test-model".to_string()),
                embedding_model_id: Some("test-embed-model".to_string()),
                temperature: Some(0.7),
                max_tokens: Some(2048),
            })
            .await
            .unwrap();

        // Create vector store and inference engine
        let vector_store = Arc::new(VectorStore::new("http://localhost:6333"));
        let inference_engine = Arc::new(InferenceEngine::new());

        let rag_engine = RAGEngine::new(vector_store, workspace_manager, inference_engine);

        // Cleanup
        std::fs::remove_file(&db_path).ok();

        (rag_engine, workspace.id)
    }

    #[tokio::test]
    async fn test_build_prompt_with_context() {
        let (rag_engine, _) = create_test_rag_engine().await;

        use crate::vector_store::ChunkMetadata;

        let context_chunks = vec![
            SearchResult {
                id: "chunk-1".to_string(),
                score: 0.95,
                chunk_text: "The capital of France is Paris.".to_string(),
                metadata: ChunkMetadata {
                    workspace_id: "ws-1".to_string(),
                    document_id: "doc1".to_string(),
                    document_name: "geography.txt".to_string(),
                    chunk_index: 0,
                    page_number: Some(1),
                    file_type: "txt".to_string(),
                    created_at: "2024-11-05T00:00:00Z".to_string(),
                },
            },
            SearchResult {
                id: "chunk-2".to_string(),
                score: 0.85,
                chunk_text: "Paris is known for the Eiffel Tower.".to_string(),
                metadata: ChunkMetadata {
                    workspace_id: "ws-1".to_string(),
                    document_id: "doc1".to_string(),
                    document_name: "geography.txt".to_string(),
                    chunk_index: 1,
                    page_number: Some(1),
                    file_type: "txt".to_string(),
                    created_at: "2024-11-05T00:00:00Z".to_string(),
                },
            },
        ];

        let (prompt, citations) = rag_engine.build_prompt(
            "What is the capital of France?",
            &context_chunks,
            Some("You are a helpful assistant."),
        );

        // Check prompt contains context
        assert!(prompt.contains("The capital of France is Paris"));
        assert!(prompt.contains("Paris is known for the Eiffel Tower"));
        assert!(prompt.contains("[1]"));
        assert!(prompt.contains("[2]"));
        assert!(prompt.contains("What is the capital of France?"));

        // Check citations
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].id, "[1]");
        assert_eq!(citations[0].document_name, "geography.txt");
        assert_eq!(citations[0].page_number, Some(1));
        assert_eq!(citations[0].score, 0.95);
        assert_eq!(citations[1].id, "[2]");
        assert_eq!(citations[1].score, 0.85);
    }

    #[tokio::test]
    async fn test_build_prompt_without_context() {
        let (rag_engine, _) = create_test_rag_engine().await;

        let (prompt, citations) = rag_engine.build_prompt(
            "What is the meaning of life?",
            &[],
            Some("You are a helpful assistant."),
        );

        // Check prompt without context
        assert!(prompt.contains("What is the meaning of life?"));
        assert!(!prompt.contains("Context:"));
        assert!(citations.is_empty());
    }

    #[tokio::test]
    async fn test_citation_metadata() {
        let (rag_engine, _) = create_test_rag_engine().await;

        use crate::vector_store::ChunkMetadata;

        let context_chunks = vec![SearchResult {
            id: "chunk-1".to_string(),
            score: 0.99,
            chunk_text: "Test content".to_string(),
            metadata: ChunkMetadata {
                workspace_id: "ws-1".to_string(),
                document_id: "test-doc".to_string(),
                document_name: "test.pdf".to_string(),
                chunk_index: 0,
                page_number: Some(42),
                file_type: "pdf".to_string(),
                created_at: "2024-11-05T00:00:00Z".to_string(),
            },
        }];

        let (_, citations) = rag_engine.build_prompt("test query", &context_chunks, None);

        assert_eq!(citations.len(), 1);
        let citation = &citations[0];
        assert_eq!(citation.document_id, "test-doc");
        assert_eq!(citation.document_name, "test.pdf");
        assert_eq!(citation.page_number, Some(42));
        assert_eq!(citation.score, 0.99);
        assert_eq!(citation.text, "Test content");
    }
}
