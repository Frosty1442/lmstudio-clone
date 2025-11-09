use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

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
    pub role: String, // "user", "assistant", "system"
    pub content: String,
    pub model_id: Option<String>,
    pub citations: Option<String>, // JSON string
    pub tokens_used: Option<i32>,
    pub created_at: String,
}

/// Request to create a chat session
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

/// Request to add a message to a session
#[derive(Debug, Clone, Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
    pub model_id: Option<String>,
    pub citations: Option<serde_json::Value>,
    pub tokens_used: Option<i32>,
}

/// Manages chat sessions and message history
pub struct ChatSessionManager {
    pool: SqlitePool,
}

impl ChatSessionManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new chat session
    pub async fn create_session(
        &self,
        workspace_id: &str,
        req: CreateSessionRequest,
    ) -> Result<ChatSession> {
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
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch created session"))
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<ChatSession>> {
        let session = sqlx::query_as::<_, ChatSession>("SELECT * FROM chat_sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(session)
    }

    /// List sessions in workspace
    pub async fn list_sessions(&self, workspace_id: &str) -> Result<Vec<ChatSession>> {
        let sessions = sqlx::query_as::<_, ChatSession>(
            "SELECT * FROM chat_sessions WHERE workspace_id = ? ORDER BY updated_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// Delete session and all its messages
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM chat_sessions WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update session title and updated_at
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

    /// Add message to session
    pub async fn add_message(
        &self,
        session_id: &str,
        req: AddMessageRequest,
    ) -> Result<Message> {
        let message_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Serialize citations to JSON string if provided
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
        .bind(&req.tokens_used)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // Update session's updated_at timestamp
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
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch created message"))
    }

    /// Get message by ID
    pub async fn get_message(&self, message_id: &str) -> Result<Option<Message>> {
        let message = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?")
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(message)
    }

    /// Get all messages in a session
    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let messages = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    /// Get session with its messages
    pub async fn get_session_with_messages(
        &self,
        session_id: &str,
    ) -> Result<Option<(ChatSession, Vec<Message>)>> {
        if let Some(session) = self.get_session(session_id).await? {
            let messages = self.get_session_messages(session_id).await?;
            Ok(Some((session, messages)))
        } else {
            Ok(None)
        }
    }

    /// Delete a specific message
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(message_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    async fn create_test_manager() -> (ChatSessionManager, String) {
        use crate::workspace::{WorkspaceManager, CreateWorkspaceRequest};

        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_chat_{}.db", Uuid::new_v4()));

        let db = Database::new(&db_path).await.unwrap();

        // Create a test workspace (required for foreign key)
        let workspace_manager = WorkspaceManager::new(db.pool().clone());
        let workspace = workspace_manager
            .create(CreateWorkspaceRequest {
                name: "Test Workspace".to_string(),
                description: Some("Test workspace for chat sessions".to_string()),
                system_prompt: None,
                model_id: None,
                embedding_model_id: None,
                temperature: None,
                max_tokens: None,
            })
            .await
            .unwrap();

        let manager = ChatSessionManager::new(db.pool().clone());

        (manager, workspace.id)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (manager, workspace_id) = create_test_manager().await;

        let req = CreateSessionRequest {
            title: Some("Test Chat".to_string()),
        };

        let session = manager.create_session(&workspace_id, req).await.unwrap();

        assert_eq!(session.workspace_id, workspace_id);
        assert_eq!(session.title, Some("Test Chat".to_string()));
        assert!(!session.id.is_empty());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let (manager, workspace_id) = create_test_manager().await;

        // Create multiple sessions
        manager
            .create_session(
                &workspace_id,
                CreateSessionRequest {
                    title: Some("Chat 1".to_string()),
                },
            )
            .await
            .unwrap();

        manager
            .create_session(
                &workspace_id,
                CreateSessionRequest {
                    title: Some("Chat 2".to_string()),
                },
            )
            .await
            .unwrap();

        let sessions = manager.list_sessions(&workspace_id).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_add_message() {
        let (manager, workspace_id) = create_test_manager().await;

        let session = manager
            .create_session(
                &workspace_id,
                CreateSessionRequest {
                    title: Some("Test Chat".to_string()),
                },
            )
            .await
            .unwrap();

        let msg_req = AddMessageRequest {
            role: "user".to_string(),
            content: "Hello!".to_string(),
            model_id: None,
            citations: None,
            tokens_used: None,
        };

        let message = manager.add_message(&session.id, msg_req).await.unwrap();

        assert_eq!(message.session_id, session.id);
        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Hello!");
    }

    #[tokio::test]
    async fn test_get_session_messages() {
        let (manager, workspace_id) = create_test_manager().await;

        let session = manager
            .create_session(
                &workspace_id,
                CreateSessionRequest {
                    title: Some("Test Chat".to_string()),
                },
            )
            .await
            .unwrap();

        // Add messages
        manager
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

        manager
            .add_message(
                &session.id,
                AddMessageRequest {
                    role: "assistant".to_string(),
                    content: "Hi there!".to_string(),
                    model_id: Some("test-model".to_string()),
                    citations: None,
                    tokens_used: Some(10),
                },
            )
            .await
            .unwrap();

        let messages = manager.get_session_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[tokio::test]
    async fn test_delete_session() {
        let (manager, workspace_id) = create_test_manager().await;

        let session = manager
            .create_session(
                &workspace_id,
                CreateSessionRequest {
                    title: Some("Test Chat".to_string()),
                },
            )
            .await
            .unwrap();

        manager.delete_session(&session.id).await.unwrap();

        let result = manager.get_session(&session.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_message_with_citations() {
        let (manager, workspace_id) = create_test_manager().await;

        let session = manager
            .create_session(
                &workspace_id,
                CreateSessionRequest {
                    title: Some("Test Chat".to_string()),
                },
            )
            .await
            .unwrap();

        let citations = serde_json::json!([
            {
                "id": "[1]",
                "document_name": "test.pdf",
                "page_number": 5
            }
        ]);

        let msg_req = AddMessageRequest {
            role: "assistant".to_string(),
            content: "Based on the document [1], ...".to_string(),
            model_id: Some("test-model".to_string()),
            citations: Some(citations),
            tokens_used: Some(50),
        };

        let message = manager.add_message(&session.id, msg_req).await.unwrap();

        assert_eq!(message.role, "assistant");
        assert!(message.citations.is_some());
        assert_eq!(message.tokens_used, Some(50));
    }
}
