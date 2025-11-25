use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_completion_request_serialization() {
        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
            ],
            stream: false,
            temperature: Some(0.7),
            top_p: None,
            max_tokens: Some(100),
            stop: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-model"));
        assert!(json.contains("Hello"));
        assert!(json.contains("0.7"));
    }

    #[test]
    fn test_chat_completion_request_deserialization() {
        let json = r#"{
            "model": "test-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "temperature": 0.7,
            "max_tokens": 100
        }"#;

        let request: ChatCompletionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.model, "test-model");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.messages[0].content, "Hello");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert!(!request.stream);
    }

    #[test]
    fn test_chat_completion_request_defaults() {
        let json = r#"{
            "model": "test-model",
            "messages": [{"role": "user", "content": "Hello"}]
        }"#;

        let request: ChatCompletionRequest = serde_json::from_str(json).unwrap();
        assert!(!request.stream);
        assert_eq!(request.temperature, None);
        assert_eq!(request.max_tokens, None);
    }

    #[test]
    fn test_string_or_array_string() {
        let json = r#"{"model": "test", "input": "single text"}"#;
        let request: EmbeddingRequest = serde_json::from_str(json).unwrap();

        match request.input {
            StringOrArray::String(s) => assert_eq!(s, "single text"),
            StringOrArray::Array(_) => panic!("Expected String variant"),
        }
    }

    #[test]
    fn test_string_or_array_array() {
        let json = r#"{"model": "test", "input": ["text1", "text2"]}"#;
        let request: EmbeddingRequest = serde_json::from_str(json).unwrap();

        match request.input {
            StringOrArray::String(_) => panic!("Expected Array variant"),
            StringOrArray::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0], "text1");
                assert_eq!(arr[1], "text2");
            }
        }
    }

    #[test]
    fn test_model_config_defaults() {
        let config = ModelConfig::default();
        assert_eq!(config.context_size, 4096);
        assert_eq!(config.gpu_layers, 32);
        assert_eq!(config.threads, 4);
        assert_eq!(config.batch_size, 512);
    }

    #[test]
    fn test_model_config_partial_json() {
        let json = r#"{"context_size": 8192}"#;
        let config: ModelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.context_size, 8192);
        assert_eq!(config.gpu_layers, 32); // default
        assert_eq!(config.threads, 4); // default
    }

    #[test]
    fn test_model_format_serialization() {
        let model = Model {
            id: "test".to_string(),
            name: "Test Model".to_string(),
            path: "/path/to/model".to_string(),
            size: 1024,
            quantization: Some("Q4_K_M".to_string()),
            loaded: false,
            format: ModelFormat::GGUF,
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("\"format\":\"gguf\""));
    }

    #[test]
    fn test_api_error_creation() {
        let error = ApiError::new("Test error", "validation_error", 400);
        assert_eq!(error.error.message, "Test error");
        assert_eq!(error.error.r#type, "validation_error");
        assert_eq!(error.error.code, 400);
    }

    #[test]
    fn test_chat_completion_response_serialization() {
        let response = ChatCompletionResponse {
            id: "test-id".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: Some(ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hello!".to_string(),
                }),
                delta: None,
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("Hello!"));
        assert!(json.contains("\"total_tokens\":15"));
    }

    #[test]
    fn test_download_model_request() {
        let json = r#"{
            "repo": "TheBloke/Llama-2-7B-GGUF",
            "quantization": "Q4_K_M"
        }"#;

        let request: DownloadModelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.repo, "TheBloke/Llama-2-7B-GGUF");
        assert_eq!(request.quantization, Some("Q4_K_M".to_string()));
        assert_eq!(request.filename, None);
    }
}
