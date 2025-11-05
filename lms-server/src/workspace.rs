use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Workspace model - isolated environment for documents and chats
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Create workspace request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub embedding_model_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

/// Update workspace request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub embedding_model_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

/// Workspace manager - handles all workspace operations
pub struct WorkspaceManager {
    pool: SqlitePool,
}

impl WorkspaceManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new workspace
    pub async fn create(&self, req: CreateWorkspaceRequest) -> Result<Workspace> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO workspaces (
                id, name, description, system_prompt, model_id,
                embedding_model_id, temperature, max_tokens, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.system_prompt)
        .bind(&req.model_id)
        .bind(&req.embedding_model_id)
        .bind(req.temperature.unwrap_or(0.7))
        .bind(req.max_tokens.unwrap_or(2048))
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("Failed to create workspace")?;

        // Fetch the created workspace
        self.get(&id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch created workspace"))
    }

    /// Get workspace by ID
    pub async fn get(&self, id: &str) -> Result<Option<Workspace>> {
        let workspace = sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get workspace")?;

        Ok(workspace)
    }

    /// List all workspaces
    pub async fn list(&self) -> Result<Vec<Workspace>> {
        let workspaces = sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list workspaces")?;

        Ok(workspaces)
    }

    /// Update workspace
    pub async fn update(&self, id: &str, req: UpdateWorkspaceRequest) -> Result<Workspace> {
        let now = chrono::Utc::now().to_rfc3339();

        // Get current workspace to check it exists
        let current = self.get(id).await?
            .ok_or_else(|| anyhow::anyhow!("Workspace not found: {}", id))?;

        // Apply updates
        let name = req.name.unwrap_or(current.name);
        let description = req.description.or(current.description);
        let system_prompt = req.system_prompt.or(current.system_prompt);
        let model_id = req.model_id.or(current.model_id);
        let embedding_model_id = req.embedding_model_id.or(current.embedding_model_id);
        let temperature = req.temperature.unwrap_or(current.temperature);
        let max_tokens = req.max_tokens.unwrap_or(current.max_tokens);

        // Execute update
        sqlx::query(
            r#"
            UPDATE workspaces
            SET name = ?, description = ?, system_prompt = ?, model_id = ?,
                embedding_model_id = ?, temperature = ?, max_tokens = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&name)
        .bind(&description)
        .bind(&system_prompt)
        .bind(&model_id)
        .bind(&embedding_model_id)
        .bind(temperature)
        .bind(max_tokens)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update workspace")?;

        // Fetch updated workspace
        self.get(id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch updated workspace"))
    }

    /// Delete workspace
    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM workspaces WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete workspace")?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Workspace not found: {}", id);
        }

        Ok(())
    }

    /// Check if workspace exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM workspaces WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check workspace existence")?;

        Ok(count > 0)
    }

    /// Get workspace statistics
    pub async fn get_stats(&self, id: &str) -> Result<WorkspaceStats> {
        let documents: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE workspace_id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let chunks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM document_chunks
             WHERE document_id IN (SELECT id FROM documents WHERE workspace_id = ?)"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let sessions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chat_sessions WHERE workspace_id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let messages: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages
             WHERE session_id IN (SELECT id FROM chat_sessions WHERE workspace_id = ?)"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(WorkspaceStats {
            documents: documents as usize,
            chunks: chunks as usize,
            sessions: sessions as usize,
            messages: messages as usize,
        })
    }
}

/// Workspace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    pub documents: usize,
    pub chunks: usize,
    pub sessions: usize,
    pub messages: usize,
}

// Implement sqlx FromRow for Workspace manually since we need custom mapping
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Workspace {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            system_prompt: row.try_get("system_prompt")?,
            model_id: row.try_get("model_id")?,
            embedding_model_id: row.try_get("embedding_model_id")?,
            temperature: row.try_get("temperature")?,
            max_tokens: row.try_get("max_tokens")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::TempDir;

    async fn setup_test_db() -> (WorkspaceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).await.unwrap();
        let manager = WorkspaceManager::new(db.pool().clone());
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_create_workspace() {
        let (manager, _temp) = setup_test_db().await;

        let req = CreateWorkspaceRequest {
            name: "Test Workspace".to_string(),
            description: Some("A test workspace".to_string()),
            system_prompt: Some("You are helpful".to_string()),
            model_id: None,
            embedding_model_id: None,
            temperature: Some(0.8),
            max_tokens: Some(1024),
        };

        let workspace = manager.create(req).await.unwrap();

        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.description, Some("A test workspace".to_string()));
        assert_eq!(workspace.temperature, 0.8);
        assert_eq!(workspace.max_tokens, 1024);
    }

    #[tokio::test]
    async fn test_get_workspace() {
        let (manager, _temp) = setup_test_db().await;

        // Create workspace
        let req = CreateWorkspaceRequest {
            name: "Test".to_string(),
            description: None,
            system_prompt: None,
            model_id: None,
            embedding_model_id: None,
            temperature: None,
            max_tokens: None,
        };
        let created = manager.create(req).await.unwrap();

        // Get workspace
        let fetched = manager.get(&created.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, created.id);

        // Get non-existent
        let not_found = manager.get("non-existent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_workspaces() {
        let (manager, _temp) = setup_test_db().await;

        // Should have default workspace
        let initial = manager.list().await.unwrap();
        assert_eq!(initial.len(), 1);

        // Create two more
        for i in 1..=2 {
            let req = CreateWorkspaceRequest {
                name: format!("Workspace {}", i),
                description: None,
                system_prompt: None,
                model_id: None,
                embedding_model_id: None,
                temperature: None,
                max_tokens: None,
            };
            manager.create(req).await.unwrap();
        }

        let all = manager.list().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_update_workspace() {
        let (manager, _temp) = setup_test_db().await;

        // Create workspace
        let req = CreateWorkspaceRequest {
            name: "Original".to_string(),
            description: Some("Original description".to_string()),
            system_prompt: None,
            model_id: None,
            embedding_model_id: None,
            temperature: Some(0.7),
            max_tokens: None,
        };
        let created = manager.create(req).await.unwrap();

        // Update
        let update = UpdateWorkspaceRequest {
            name: Some("Updated".to_string()),
            description: None,
            system_prompt: Some("New prompt".to_string()),
            model_id: None,
            embedding_model_id: None,
            temperature: Some(0.9),
            max_tokens: None,
        };
        let updated = manager.update(&created.id, update).await.unwrap();

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.system_prompt, Some("New prompt".to_string()));
        assert_eq!(updated.temperature, 0.9);
        // Description should remain unchanged
        assert_eq!(updated.description, Some("Original description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_workspace() {
        let (manager, _temp) = setup_test_db().await;

        // Create workspace
        let req = CreateWorkspaceRequest {
            name: "To Delete".to_string(),
            description: None,
            system_prompt: None,
            model_id: None,
            embedding_model_id: None,
            temperature: None,
            max_tokens: None,
        };
        let created = manager.create(req).await.unwrap();

        // Verify exists
        assert!(manager.exists(&created.id).await.unwrap());

        // Delete
        manager.delete(&created.id).await.unwrap();

        // Verify deleted
        assert!(!manager.exists(&created.id).await.unwrap());

        // Try to delete again - should fail
        let result = manager.delete(&created.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workspace_stats() {
        let (manager, _temp) = setup_test_db().await;

        // Create workspace
        let req = CreateWorkspaceRequest {
            name: "Stats Test".to_string(),
            description: None,
            system_prompt: None,
            model_id: None,
            embedding_model_id: None,
            temperature: None,
            max_tokens: None,
        };
        let workspace = manager.create(req).await.unwrap();

        // Get stats - should be all zeros
        let stats = manager.get_stats(&workspace.id).await.unwrap();
        assert_eq!(stats.documents, 0);
        assert_eq!(stats.chunks, 0);
        assert_eq!(stats.sessions, 0);
        assert_eq!(stats.messages, 0);
    }
}
