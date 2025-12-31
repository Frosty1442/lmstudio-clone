// Unified repository for all database operations
// Consolidates workspace, chat session, and document operations into one place

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

// ============================================================================
// Domain Types
// ============================================================================

/// Workspace record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub embedding_model_id: Option<String>,
    pub temperature: f32,
    pub max_tokens: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Chat session record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatSession {
    pub id: String,
    pub workspace_id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Message in a chat session
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub model_id: Option<String>,
    pub citations: Option<String>,
    pub tokens_used: Option<i32>,
    pub created_at: String,
}

/// Document record
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

/// Workspace statistics
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceStats {
    pub documents: i32,
    pub chunks: i32,
    pub sessions: i32,
    pub messages: i32,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub embedding_model_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub embedding_model_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
    pub model_id: Option<String>,
    pub citations: Option<serde_json::Value>,
    pub tokens_used: Option<i32>,
}

// ============================================================================
// Repository Implementation
// ============================================================================

/// Unified repository for all database operations
#[derive(Clone)]
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ========================================================================
    // Workspace Operations
    // ========================================================================

    pub async fn create_workspace(&self, req: CreateWorkspaceRequest) -> Result<Workspace> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let temperature = req.temperature.unwrap_or(0.7);
        let max_tokens = req.max_tokens.unwrap_or(2048);

        sqlx::query(
            r#"
            INSERT INTO workspaces (id, name, description, system_prompt, model_id, embedding_model_id, temperature, max_tokens, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.system_prompt)
        .bind(&req.model_id)
        .bind(&req.embedding_model_id)
        .bind(temperature)
        .bind(max_tokens)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_workspace(&id)
            .await?
            .ok_or_else(|| anyhow!("Failed to fetch created workspace"))
    }

    pub async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>> {
        let workspace = sqlx::query_as::<_, Workspace>("SELECT * FROM workspaces WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(workspace)
    }

    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let workspaces = sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(workspaces)
    }

    pub async fn update_workspace(&self, id: &str, req: UpdateWorkspaceRequest) -> Result<Workspace> {
        let current = self
            .get_workspace(id)
            .await?
            .ok_or_else(|| anyhow!("Workspace not found: {}", id))?;

        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE workspaces
            SET name = ?, description = ?, system_prompt = ?, model_id = ?,
                embedding_model_id = ?, temperature = ?, max_tokens = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(req.name.unwrap_or(current.name))
        .bind(req.description.or(current.description))
        .bind(req.system_prompt.or(current.system_prompt))
        .bind(req.model_id.or(current.model_id))
        .bind(req.embedding_model_id.or(current.embedding_model_id))
        .bind(req.temperature.unwrap_or(current.temperature))
        .bind(req.max_tokens.unwrap_or(current.max_tokens))
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_workspace(id)
            .await?
            .ok_or_else(|| anyhow!("Failed to fetch updated workspace"))
    }

    pub async fn delete_workspace(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_workspace_stats(&self, id: &str) -> Result<WorkspaceStats> {
        let documents: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE workspace_id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let chunks: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM document_chunks WHERE document_id IN (SELECT id FROM documents WHERE workspace_id = ?)"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let sessions: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chat_sessions WHERE workspace_id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let messages: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE session_id IN (SELECT id FROM chat_sessions WHERE workspace_id = ?)"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(WorkspaceStats {
            documents,
            chunks,
            sessions,
            messages,
        })
    }

    // ========================================================================
    // Chat Session Operations
    // ========================================================================

    pub async fn create_session(&self, workspace_id: &str, req: CreateSessionRequest) -> Result<ChatSession> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, workspace_id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(workspace_id)
        .bind(&req.title)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_session(&session_id)
            .await?
            .ok_or_else(|| anyhow!("Failed to fetch created session"))
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<ChatSession>> {
        let session = sqlx::query_as::<_, ChatSession>("SELECT * FROM chat_sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(session)
    }

    pub async fn list_sessions(&self, workspace_id: &str) -> Result<Vec<ChatSession>> {
        let sessions = sqlx::query_as::<_, ChatSession>(
            "SELECT * FROM chat_sessions WHERE workspace_id = ? ORDER BY updated_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM chat_sessions WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_session_title(&self, session_id: &str, title: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE chat_sessions
            SET title = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(title)
        .bind(&now)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_message(&self, session_id: &str, req: AddMessageRequest) -> Result<Message> {
        let message_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let citations_json = req
            .citations
            .map(|c| serde_json::to_string(&c))
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO messages (id, session_id, role, content, model_id, citations, tokens_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&message_id)
        .bind(session_id)
        .bind(&req.role)
        .bind(&req.content)
        .bind(&req.model_id)
        .bind(&citations_json)
        .bind(req.tokens_used)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // Update session timestamp
        sqlx::query(
            r#"
            UPDATE chat_sessions
            SET updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        self.get_message(&message_id)
            .await?
            .ok_or_else(|| anyhow!("Failed to fetch created message"))
    }

    pub async fn get_message(&self, message_id: &str) -> Result<Option<Message>> {
        let message = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?")
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(message)
    }

    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let messages = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    pub async fn get_session_with_messages(&self, session_id: &str) -> Result<Option<(ChatSession, Vec<Message>)>> {
        if let Some(session) = self.get_session(session_id).await? {
            let messages = self.get_session_messages(session_id).await?;
            Ok(Some((session, messages)))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(message_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Document Operations
    // ========================================================================

    pub async fn create_document(
        &self,
        document_id: &str,
        workspace_id: &str,
        filename: &str,
        file_path: &str,
        file_type: &str,
        file_size: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO documents (id, workspace_id, filename, file_path, file_type, file_size, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 'processing', ?)
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .bind(filename)
        .bind(file_path)
        .bind(file_type)
        .bind(file_size)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_document_status(
        &self,
        document_id: &str,
        status: &str,
        error_message: Option<&str>,
        chunk_count: Option<i32>,
    ) -> Result<()> {
        if let Some(count) = chunk_count {
            sqlx::query(
                r#"
                UPDATE documents
                SET status = ?, error_message = ?, chunk_count = ?
                WHERE id = ?
                "#,
            )
            .bind(status)
            .bind(error_message)
            .bind(count)
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE documents
                SET status = ?, error_message = ?
                WHERE id = ?
                "#,
            )
            .bind(status)
            .bind(error_message)
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_document(&self, document_id: &str) -> Result<Option<Document>> {
        let doc = sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id = ?")
            .bind(document_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(doc)
    }

    pub async fn list_documents(&self, workspace_id: &str) -> Result<Vec<Document>> {
        let docs = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE workspace_id = ? ORDER BY created_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(docs)
    }

    pub async fn delete_document(&self, document_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(document_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // Parameters map directly to table columns
    pub async fn create_document_chunk(
        &self,
        chunk_id: &str,
        document_id: &str,
        chunk_index: i32,
        chunk_text: &str,
        char_start: i32,
        char_end: i32,
        vector_id: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO document_chunks
            (id, document_id, chunk_index, chunk_text, char_start, char_end, vector_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(chunk_id)
        .bind(document_id)
        .bind(chunk_index)
        .bind(chunk_text)
        .bind(char_start)
        .bind(char_end)
        .bind(vector_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::env::temp_dir;

    async fn create_test_repository() -> (Repository, String) {
        let db_path = temp_dir().join(format!("test_repo_{}.db", Uuid::new_v4()));
        let db = Database::new(&db_path).await.unwrap();
        let repository = Repository::new(db.pool().clone());

        // Create test workspace
        let workspace = repository
            .create_workspace(CreateWorkspaceRequest {
                name: "Test Workspace".to_string(),
                description: Some("Test".to_string()),
                system_prompt: None,
                model_id: None,
                embedding_model_id: None,
                temperature: None,
                max_tokens: None,
            })
            .await
            .unwrap();

        (repository, workspace.id)
    }

    #[tokio::test]
    async fn test_workspace_crud() {
        let (repo, workspace_id) = create_test_repository().await;

        // Get
        let workspace = repo.get_workspace(&workspace_id).await.unwrap().unwrap();
        assert_eq!(workspace.name, "Test Workspace");

        // Update
        let updated = repo
            .update_workspace(
                &workspace_id,
                UpdateWorkspaceRequest {
                    name: Some("Updated".to_string()),
                    description: None,
                    system_prompt: None,
                    model_id: None,
                    embedding_model_id: None,
                    temperature: None,
                    max_tokens: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Updated");

        // List
        let workspaces = repo.list_workspaces().await.unwrap();
        assert_eq!(workspaces.len(), 2); // default + test

        // Delete
        repo.delete_workspace(&workspace_id).await.unwrap();
        let result = repo.get_workspace(&workspace_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_chat_session_crud() {
        let (repo, workspace_id) = create_test_repository().await;

        // Create session
        let session = repo
            .create_session(
                &workspace_id,
                CreateSessionRequest {
                    title: Some("Test Chat".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(session.title, Some("Test Chat".to_string()));

        // Add message
        let message = repo
            .add_message(
                &session.id,
                AddMessageRequest {
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                    model_id: None,
                    citations: None,
                    tokens_used: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(message.content, "Hello!");

        // Get messages
        let messages = repo.get_session_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 1);

        // Delete session
        repo.delete_session(&session.id).await.unwrap();
        let result = repo.get_session(&session.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_workspace_stats() {
        let (repo, workspace_id) = create_test_repository().await;

        let stats = repo.get_workspace_stats(&workspace_id).await.unwrap();
        assert_eq!(stats.documents, 0);
        assert_eq!(stats.sessions, 0);
    }
}
