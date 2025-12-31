//! LMStudio Clone Server Library
//!
//! This library provides the core functionality for the LMStudio Clone server,
//! including API handlers, database operations, RAG engine, and more.
//!
//! # Features
//!
//! - **OpenAI Compatible API**: Drop-in replacement for OpenAI client libraries
//! - **RAG Support**: Upload documents and query with citations
//! - **Workspace Isolation**: Organize projects with separate document/chat contexts
//! - **Local First**: Complete privacy, offline operation
//!
//! # Example
//!
//! ```ignore
//! use lms_server::api::{AppState, Services};
//! use lms_server::db::Database;
//!
//! let database = Database::new(&db_path).await?;
//! let repository = Repository::new(database.pool().clone());
//! // ... initialize services
//! let state = AppState::new(services);
//! ```

pub mod api;
pub mod auth;
pub mod db;
pub mod document_manager;
pub mod documents;
pub mod inference;
pub mod metrics;
pub mod models;
pub mod rag;
pub mod rate_limit;
pub mod repository;
pub mod types;
pub mod vector_store;
