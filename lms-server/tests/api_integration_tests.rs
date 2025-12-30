//! Integration tests for the LMS Server API
//!
//! These tests verify the API endpoints work correctly end-to-end.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt;

// Import from the server crate
use lms_server::api::{self, AppState, Services};
use lms_server::db::Database;
use lms_server::document_manager::DocumentManager;
use lms_server::inference::InferenceEngine;
use lms_server::models::ModelManager;
use lms_server::rag::RAGEngine;
use lms_server::repository::Repository;
use lms_server::vector_store::VectorStore;

/// Helper to create a test application with all routes
async fn create_test_app() -> (Router, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let models_path = temp_dir.path().join("models");
    let db_path = temp_dir.path().join("test.db");
    let docs_path = temp_dir.path().join("documents");

    std::fs::create_dir_all(&models_path).unwrap();
    std::fs::create_dir_all(&docs_path).unwrap();

    // Create a dummy model file
    std::fs::write(models_path.join("test-model.gguf"), b"test data").unwrap();

    // Create database and repository
    let database = Arc::new(Database::new(&db_path).await.unwrap());
    let repository = Repository::new(database.pool().clone());

    // Create inference engine
    let inference_engine = Arc::new(InferenceEngine::new());

    // Create model manager
    let model_manager = ModelManager::new(models_path, inference_engine.clone())
        .await
        .unwrap();

    // Create vector store (SQLite-based)
    let vector_store = Arc::new(VectorStore::new(database.pool().clone()));

    // Create document manager
    let document_manager = DocumentManager::new(
        database.pool().clone(),
        &repository,
        vector_store.clone(),
        inference_engine.clone(),
        docs_path,
    );

    // Create RAG engine
    let rag_engine = RAGEngine::new(vector_store, &repository, inference_engine);

    let state = AppState::new(Services {
        repository,
        model_manager,
        document_manager,
        rag_engine,
        start_time: Instant::now(),
    });

    let app = Router::new()
        .route("/health", axum::routing::get(api::health))
        .route("/v1/status", axum::routing::get(api::server_status))
        .route("/v1/models", axum::routing::get(api::list_models))
        .route("/v1/models/list", axum::routing::get(api::list_all_models))
        .route("/v1/workspaces", axum::routing::post(api::create_workspace))
        .route("/v1/workspaces", axum::routing::get(api::list_workspaces))
        .route("/v1/workspaces/:id", axum::routing::get(api::get_workspace))
        .route(
            "/v1/workspaces/:id",
            axum::routing::delete(api::delete_workspace),
        )
        .with_state(state);

    (app, temp_dir)
}

/// Helper to make a request and get JSON response
async fn make_request(app: Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let body = match body {
        Some(v) => Body::from(serde_json::to_vec(&v).unwrap()),
        None => Body::empty(),
    };

    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap_or(json!({}));

    (status, json)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (app, _temp) = create_test_app().await;

    let (status, json) = make_request(app, "GET", "/health", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_server_status_endpoint() {
    let (app, _temp) = create_test_app().await;

    let (status, json) = make_request(app, "GET", "/v1/status", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["running"], true);
    assert!(json["loaded_models"].is_array());
    assert!(json["uptime"].is_number());
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_list_models_openai_format() {
    let (app, _temp) = create_test_app().await;

    let (status, json) = make_request(app, "GET", "/v1/models", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["object"], "list");
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_list_all_models_with_details() {
    let (app, _temp) = create_test_app().await;

    let (status, json) = make_request(app, "GET", "/v1/models/list", None).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["models"].is_array());

    // Should have at least one model (test-model.gguf)
    let models = json["models"].as_array().unwrap();
    assert!(!models.is_empty());

    // Check model structure
    let model = &models[0];
    assert!(model["id"].is_string());
    assert!(model["name"].is_string());
}

#[tokio::test]
async fn test_workspace_crud_operations() {
    let (app, _temp) = create_test_app().await;

    // Create workspace
    let create_body = json!({
        "name": "Test Workspace",
        "description": "A test workspace for integration tests"
    });

    let (status, json) = make_request(app.clone(), "POST", "/v1/workspaces", Some(create_body)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert!(json["workspace"]["id"].is_string());

    let workspace_id = json["workspace"]["id"].as_str().unwrap().to_string();

    // List workspaces
    let (status, json) = make_request(app.clone(), "GET", "/v1/workspaces", None).await;
    assert_eq!(status, StatusCode::OK);
    let workspaces = json["workspaces"].as_array().unwrap();
    assert!(workspaces.iter().any(|w| w["id"] == workspace_id));

    // Get specific workspace
    let (status, json) = make_request(
        app.clone(),
        "GET",
        &format!("/v1/workspaces/{}", workspace_id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["workspace"]["name"], "Test Workspace");

    // Delete workspace
    let (status, json) = make_request(
        app.clone(),
        "DELETE",
        &format!("/v1/workspaces/{}", workspace_id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);

    // Verify deleted
    let (status, json) = make_request(app, "GET", "/v1/workspaces", None).await;
    assert_eq!(status, StatusCode::OK);
    let workspaces = json["workspaces"].as_array().unwrap();
    assert!(!workspaces.iter().any(|w| w["id"] == workspace_id));
}

#[tokio::test]
async fn test_uptime_tracking() {
    let (app, _temp) = create_test_app().await;

    // Wait a bit to ensure uptime > 0
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let (status, json) = make_request(app, "GET", "/v1/status", None).await;

    assert_eq!(status, StatusCode::OK);
    // Uptime should be at least 0 (might be 0 if checked immediately)
    assert!(json["uptime"].as_u64().is_some());
}

#[tokio::test]
async fn test_create_multiple_workspaces() {
    let (app, _temp) = create_test_app().await;

    // Create first workspace
    let (status, _) = make_request(
        app.clone(),
        "POST",
        "/v1/workspaces",
        Some(json!({"name": "Workspace 1"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Create second workspace
    let (status, _) = make_request(
        app.clone(),
        "POST",
        "/v1/workspaces",
        Some(json!({"name": "Workspace 2"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // List should show both (plus default)
    let (status, json) = make_request(app, "GET", "/v1/workspaces", None).await;
    assert_eq!(status, StatusCode::OK);

    let workspaces = json["workspaces"].as_array().unwrap();
    // Should have at least 2 workspaces (the ones we created)
    assert!(workspaces.len() >= 2);
}
