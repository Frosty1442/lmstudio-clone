use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// OpenAI API Types
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize)]
pub struct ChatChoice {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ChatMessage>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: StringOrArray,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrArray {
    String(String),
    Array(Vec<String>),
}

// Model Management Types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub quantization: Option<String>,
    pub loaded: bool,
    pub format: ModelFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    GGUF,
    GGML,
    SafeTensors,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadModelRequest {
    pub model_id: String,
    #[serde(default)]
    pub config: Option<ModelConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    #[serde(default = "default_context_size")]
    pub context_size: usize,
    #[serde(default = "default_gpu_layers")]
    pub gpu_layers: u32,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_context_size() -> usize {
    4096
}
fn default_gpu_layers() -> u32 {
    32
}
fn default_threads() -> u32 {
    4
}
fn default_batch_size() -> usize {
    512
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            context_size: default_context_size(),
            gpu_layers: default_gpu_layers(),
            threads: default_threads(),
            batch_size: default_batch_size(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadModelRequest {
    pub repo: String,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub quantization: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ModelStats {
    pub model_id: String,
    pub loaded: bool,
    pub memory_usage: u64,
    pub tokens_per_second: f32,
    pub time_to_first_token: u64,
    pub context_usage: usize,
    pub max_context: usize,
}

#[derive(Debug, Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub loaded_models: Vec<String>,
    pub uptime: u64,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub message: String,
    pub r#type: String,
    pub code: u16,
}

impl ApiError {
    pub fn new(message: impl Into<String>, error_type: impl Into<String>, code: u16) -> Self {
        Self {
            error: ErrorDetail {
                message: message.into(),
                r#type: error_type.into(),
                code,
            },
        }
    }
}
