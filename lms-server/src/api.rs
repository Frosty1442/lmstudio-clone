use crate::models::ModelManager;
use crate::types::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;

#[derive(Clone)]
pub struct AppState {
    pub model_manager: Arc<ModelManager>,
}

// Health check
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// List models (OpenAI compatible)
pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let models = state.model_manager.list_models().await;

    Json(serde_json::json!({
        "object": "list",
        "data": models.iter().map(|m| {
            serde_json::json!({
                "id": m.id,
                "object": "model",
                "created": 0,
                "owned_by": "local",
            })
        }).collect::<Vec<_>>()
    }))
}

// Chat completions (OpenAI compatible)
#[axum::debug_handler]
pub async fn chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    // Check if model is loaded
    if !state.model_manager.is_loaded(&req.model).await {
        // Try to auto-load
        if let Err(e) = state
            .model_manager
            .load_model(&req.model, ModelConfig::default())
            .await
        {
            return AppError::ModelNotFound(e.to_string()).into_response();
        }
    }

    // Build prompt from messages
    let prompt = req
        .messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let temperature = req.temperature.unwrap_or(0.7);
    let max_tokens = req.max_tokens.unwrap_or(2048);

    if req.stream {
        // Streaming response
        let stream = match state
            .model_manager
            .inference_engine()
            .generate_stream(&req.model, &prompt, temperature, max_tokens)
            .await
        {
            Ok(s) => s,
            Err(e) => return AppError::InferenceError(e.to_string()).into_response(),
        };

        let model = req.model.clone();
        let id = uuid::Uuid::new_v4().to_string();

        // Map the token stream to SSE events
        let sse_stream = stream.map(move |token| {
            let chunk = ChatCompletionResponse {
                id: id.clone(),
                object: "chat.completion.chunk".to_string(),
                created: current_timestamp(),
                model: model.clone(),
                choices: vec![ChatChoice {
                    index: 0,
                    message: None,
                    delta: Some(ChatMessage {
                        role: "assistant".to_string(),
                        content: token,
                    }),
                    finish_reason: None,
                }],
                usage: None,
            };
            Ok::<_, Infallible>(axum::response::sse::Event::default().json_data(chunk).unwrap())
        });

        Sse::new(sse_stream).into_response()
    } else {
        // Non-streaming response
        let content = match state
            .model_manager
            .inference_engine()
            .generate(&req.model, &prompt, temperature, max_tokens)
            .await
        {
            Ok(c) => c,
            Err(e) => return AppError::InferenceError(e.to_string()).into_response(),
        };

        let response = ChatCompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            object: "chat.completion".to_string(),
            created: current_timestamp(),
            model: req.model,
            choices: vec![ChatChoice {
                index: 0,
                message: Some(ChatMessage {
                    role: "assistant".to_string(),
                    content,
                }),
                delta: None,
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            }),
        };

        Json(response).into_response()
    }
}

// Text completions (OpenAI compatible)
pub async fn completions(
    State(state): State<AppState>,
    Json(req): Json<CompletionRequest>,
) -> impl IntoResponse {
    async fn inner(
        state: Arc<ModelManager>,
        req: CompletionRequest,
    ) -> Result<Response, AppError> {
        if !state.is_loaded(&req.model).await {
            state
                .load_model(&req.model, ModelConfig::default())
                .await
                .map_err(|e| AppError::ModelNotFound(e.to_string()))?;
        }

        let temperature = req.temperature.unwrap_or(0.7);
        let max_tokens = req.max_tokens.unwrap_or(2048);

        let content = state
            .inference_engine()
            .generate(&req.model, &req.prompt, temperature, max_tokens)
            .await
            .map_err(|e| AppError::InferenceError(e.to_string()))?;

        Ok(Json(serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "object": "text_completion",
            "created": current_timestamp(),
            "model": req.model,
            "choices": [{
                "text": content,
                "index": 0,
                "finish_reason": "stop"
            }]
        }))
        .into_response())
    }

    match inner(state.model_manager, req).await {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

// Embeddings
pub async fn embeddings(
    State(state): State<AppState>,
    Json(req): Json<EmbeddingRequest>,
) -> impl IntoResponse {
    async fn inner(
        state: Arc<ModelManager>,
        req: EmbeddingRequest,
    ) -> Result<Response, AppError> {
        if !state.is_loaded(&req.model).await {
            state
                .load_model(&req.model, ModelConfig::default())
                .await
                .map_err(|e| AppError::ModelNotFound(e.to_string()))?;
        }

        let texts = match req.input {
            StringOrArray::String(s) => vec![s],
            StringOrArray::Array(arr) => arr,
        };

        let embeddings = state
            .inference_engine()
            .generate_embeddings(&req.model, texts.clone())
            .await
            .map_err(|e| AppError::InferenceError(e.to_string()))?;

        let data: Vec<_> = embeddings
            .into_iter()
            .enumerate()
            .map(|(i, embedding)| {
                serde_json::json!({
                    "object": "embedding",
                    "embedding": embedding,
                    "index": i
                })
            })
            .collect();

        let total_tokens: usize = texts.iter().map(|t| t.split_whitespace().count()).sum();

        Ok(Json(serde_json::json!({
            "object": "list",
            "data": data,
            "model": req.model,
            "usage": {
                "prompt_tokens": total_tokens,
                "total_tokens": total_tokens
            }
        }))
        .into_response())
    }

    match inner(state.model_manager, req).await {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

// Load model
pub async fn load_model(
    State(state): State<AppState>,
    Json(req): Json<LoadModelRequest>,
) -> impl IntoResponse {
    async fn inner(
        state: Arc<ModelManager>,
        req: LoadModelRequest,
    ) -> Result<Response, AppError> {
        let config = req.config.unwrap_or_default();

        state
            .load_model(&req.model_id, config)
            .await
            .map_err(|e| AppError::ModelLoadError(e.to_string()))?;

        Ok(Json(serde_json::json!({
            "success": true,
            "model_id": req.model_id
        }))
        .into_response())
    }

    match inner(state.model_manager, req).await {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

// Unload model
pub async fn unload_model(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Response {
    let model_id = match req["model_id"].as_str() {
        Some(id) => id,
        None => return AppError::InvalidRequest("Missing model_id".to_string()).into_response(),
    };

    if let Err(e) = state.model_manager.unload_model(model_id).await {
        return AppError::ModelUnloadError(e.to_string()).into_response();
    }

    Json(serde_json::json!({
        "success": true,
        "model_id": model_id
    }))
    .into_response()
}

// Download model
pub async fn download_model(
    State(state): State<AppState>,
    Json(req): Json<DownloadModelRequest>,
) -> Response {
    if let Err(e) = state.model_manager.download_model(req).await {
        return AppError::DownloadError(e.to_string()).into_response();
    }

    Json(serde_json::json!({
        "success": true
    }))
    .into_response()
}

// List all models (with details)
pub async fn list_all_models(State(state): State<AppState>) -> impl IntoResponse {
    let models = state.model_manager.list_models().await;
    Json(serde_json::json!({ "models": models }))
}

// Get model stats
pub async fn model_stats(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Placeholder
    Json(ModelStats {
        model_id: id,
        loaded: false,
        memory_usage: 0,
        tokens_per_second: 0.0,
        time_to_first_token: 0,
        context_usage: 0,
        max_context: 4096,
    })
}

// Server status
pub async fn server_status(State(state): State<AppState>) -> impl IntoResponse {
    let loaded_models = state.model_manager.get_loaded_models().await;

    Json(ServerStatus {
        running: true,
        loaded_models,
        uptime: 0, // TODO: Track actual uptime
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// Helper functions
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// Error handling
#[derive(Debug)]
pub enum AppError {
    ModelNotFound(String),
    ModelLoadError(String),
    ModelUnloadError(String),
    InferenceError(String),
    DownloadError(String),
    InvalidRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, error_type) = match self {
            AppError::ModelNotFound(msg) => (StatusCode::NOT_FOUND, msg, "model_not_found"),
            AppError::ModelLoadError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg, "load_error"),
            AppError::ModelUnloadError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg, "unload_error")
            }
            AppError::InferenceError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg, "inference_error")
            }
            AppError::DownloadError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg, "download_error")
            }
            AppError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg, "invalid_request"),
        };

        error!("API Error: {} - {}", error_type, message);

        let error = ApiError::new(message, error_type, status.as_u16());
        (status, Json(error)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::InferenceEngine;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
        routing::{get, post},
    };
    use tower::ServiceExt;
    use tempfile::TempDir;
    use http_body_util::BodyExt;

    fn create_test_app(model_manager: Arc<ModelManager>) -> Router {
        let state = AppState { model_manager };

        Router::new()
            .route("/health", get(health))
            .route("/v1/models", get(list_models))
            .route("/v1/models/list", get(list_all_models))
            .route("/status", get(server_status))
            .with_state(state)
    }

    async fn create_test_model_manager() -> Arc<ModelManager> {
        let temp_dir = TempDir::new().unwrap();
        let models_path = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(&models_path).unwrap();

        // Create a dummy model file
        std::fs::write(models_path.join("test.gguf"), b"test data").unwrap();

        let inference_engine = Arc::new(InferenceEngine::new());
        let manager = ModelManager::new(models_path, inference_engine).await.unwrap();

        // Leak the temp_dir so it doesn't get deleted
        std::mem::forget(temp_dir);

        Arc::new(manager)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let manager = create_test_model_manager().await;
        let app = create_test_app(manager);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_list_models_endpoint() {
        let manager = create_test_model_manager().await;
        let app = create_test_app(manager);

        let response = app
            .oneshot(Request::builder().uri("/v1/models").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["object"], "list");
        assert!(json["data"].is_array());
    }

    #[tokio::test]
    async fn test_server_status_endpoint() {
        let manager = create_test_model_manager().await;
        let app = create_test_app(manager);

        let response = app
            .oneshot(Request::builder().uri("/status").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["running"], true);
        assert!(json["loaded_models"].is_array());
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_list_all_models_endpoint() {
        let manager = create_test_model_manager().await;
        let app = create_test_app(manager);

        let response = app
            .oneshot(Request::builder().uri("/v1/models/list").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json["models"].is_array());
        // Should have at least one model (test.gguf)
        assert!(json["models"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_app_error_into_response() {
        let error = AppError::ModelNotFound("test-model".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let error = AppError::InvalidRequest("bad request".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let error = AppError::InferenceError("inference failed".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);

        // Should be a reasonable timestamp (after 2020)
        assert!(ts > 1577836800); // 2020-01-01
    }
}
