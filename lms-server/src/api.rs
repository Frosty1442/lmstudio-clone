use crate::inference::InferenceEngine;
use crate::models::ModelManager;
use crate::types::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures_util::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;

#[derive(Clone)]
pub struct AppState {
    pub model_manager: ModelManager,
    pub inference_engine: InferenceEngine,
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
pub async fn chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, AppError> {
    // Check if model is loaded
    if !state.model_manager.is_loaded(&req.model).await {
        // Try to auto-load
        state
            .model_manager
            .load_model(&req.model, ModelConfig::default())
            .await
            .map_err(|e| AppError::ModelNotFound(e.to_string()))?;
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
        let stream = state
            .inference_engine
            .generate_stream(&req.model, &prompt, temperature, max_tokens)
            .await
            .map_err(|e| AppError::InferenceError(e.to_string()))?;

        let model = req.model.clone();
        let sse_stream = futures_util::stream::iter(stream)
            .await
            .map(move |token| {
                let chunk = ChatCompletionResponse {
                    id: uuid::Uuid::new_v4().to_string(),
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

        Ok(Sse::new(sse_stream).into_response())
    } else {
        // Non-streaming response
        let content = state
            .inference_engine
            .generate(&req.model, &prompt, temperature, max_tokens)
            .await
            .map_err(|e| AppError::InferenceError(e.to_string()))?;

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

        Ok(Json(response).into_response())
    }
}

// Text completions (OpenAI compatible)
pub async fn completions(
    State(state): State<AppState>,
    Json(req): Json<CompletionRequest>,
) -> Result<Response, AppError> {
    if !state.model_manager.is_loaded(&req.model).await {
        state
            .model_manager
            .load_model(&req.model, ModelConfig::default())
            .await
            .map_err(|e| AppError::ModelNotFound(e.to_string()))?;
    }

    let temperature = req.temperature.unwrap_or(0.7);
    let max_tokens = req.max_tokens.unwrap_or(2048);

    let content = state
        .inference_engine
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

// Embeddings (placeholder)
pub async fn embeddings(Json(_req): Json<EmbeddingRequest>) -> impl IntoResponse {
    Json(serde_json::json!({
        "object": "list",
        "data": [],
        "model": "placeholder",
        "usage": {
            "prompt_tokens": 0,
            "total_tokens": 0
        }
    }))
}

// Load model
pub async fn load_model(
    State(state): State<AppState>,
    Json(req): Json<LoadModelRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = req.config.unwrap_or_default();

    state
        .model_manager
        .load_model(&req.model_id, config)
        .await
        .map_err(|e| AppError::ModelLoadError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "model_id": req.model_id
    })))
}

// Unload model
pub async fn unload_model(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let model_id = req["model_id"]
        .as_str()
        .ok_or_else(|| AppError::InvalidRequest("Missing model_id".to_string()))?;

    state
        .model_manager
        .unload_model(model_id)
        .await
        .map_err(|e| AppError::ModelUnloadError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "model_id": model_id
    })))
}

// Download model
pub async fn download_model(
    State(state): State<AppState>,
    Json(req): Json<DownloadModelRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .model_manager
        .download_model(req)
        .await
        .map_err(|e| AppError::DownloadError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true
    })))
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
