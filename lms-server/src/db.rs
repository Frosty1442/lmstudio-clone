use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::ConnectOptions;
use std::path::Path;
use std::str::FromStr;
use tracing::{info, warn};

/// Database connection pool and management
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Initialize database with the given path
    /// Creates the database file if it doesn't exist
    pub async fn new(db_path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        info!("Opening database at: {}", db_path.display());

        // Configure connection options
        let options = SqliteConnectOptions::from_str(&format!(
            "sqlite:{}",
            db_path.to_string_lossy()
        ))?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .disable_statement_logging();

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("Failed to connect to database")?;

        let db = Self { pool };

        // Run migrations
        db.run_migrations().await?;

        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // Read and execute the migration file
        let migration_sql = include_str!("../migrations/20241105000001_init_schema.sql");

        // Execute each statement
        sqlx::raw_sql(migration_sql)
            .execute(&self.pool)
            .await
            .context("Failed to run migrations")?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Check if database is healthy
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let workspaces: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
                .fetch_one(&self.pool)
                .await?;

        let documents: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
            .fetch_one(&self.pool)
            .await?;

        let chunks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM document_chunks")
                .fetch_one(&self.pool)
                .await?;

        let sessions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_sessions")
                .fetch_one(&self.pool)
                .await?;

        let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await?;

        Ok(DatabaseStats {
            workspaces: workspaces as usize,
            documents: documents as usize,
            chunks: chunks as usize,
            sessions: sessions as usize,
            messages: messages as usize,
        })
    }

    /// Close the database connection pool
    pub async fn close(self) {
        info!("Closing database connection pool");
        self.pool.close().await;
    }
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub workspaces: usize,
    pub documents: usize,
    pub chunks: usize,
    pub sessions: usize,
    pub messages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();

        // Check health
        db.health_check().await.unwrap();

        // Check stats
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.workspaces, 1); // Default workspace
        assert_eq!(stats.documents, 0);
        assert_eq!(stats.chunks, 0);
        assert_eq!(stats.sessions, 0);
        assert_eq!(stats.messages, 0);
    }

    #[tokio::test]
    async fn test_database_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();

        // Verify tables were created
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();

        let table_names: Vec<String> = tables.into_iter().map(|(name,)| name).collect();

        assert!(table_names.contains(&"workspaces".to_string()));
        assert!(table_names.contains(&"documents".to_string()));
        assert!(table_names.contains(&"document_chunks".to_string()));
        assert!(table_names.contains(&"chat_sessions".to_string()));
        assert!(table_names.contains(&"messages".to_string()));
        assert!(table_names.contains(&"settings".to_string()));
    }

    #[tokio::test]
    async fn test_default_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();

        // Check that default workspace exists
        let workspace: (String, String) = sqlx::query_as(
            "SELECT id, name FROM workspaces WHERE id = 'default'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert_eq!(workspace.0, "default");
        assert_eq!(workspace.1, "Default Workspace");
    }
}
