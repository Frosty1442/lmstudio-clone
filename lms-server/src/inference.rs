use anyhow::{anyhow, Result};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
struct LlamaServerRequest {
    prompt: String,
    n_predict: i32,
    temperature: f32,
    stream: bool,
}

pub struct ModelProcess {
    child: Child,
    port: u16,
    model_path: PathBuf,
}

pub struct InferenceEngine {
    processes: Arc<RwLock<HashMap<String, Arc<Mutex<ModelProcess>>>>>,
    base_port: u16,
}

impl InferenceEngine {
    pub fn new() -> Self {
        info!("Initializing InferenceEngine with subprocess architecture");
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            base_port: 8080,
        }
    }

    pub async fn load_model(
        &self,
        model_id: String,
        model_path: &Path,
        n_ctx: u32,
        n_gpu_layers: u32,
        _n_threads: u32,
    ) -> Result<()> {
        info!(
            "Loading model {} from {} (ctx: {}, gpu_layers: {})",
            model_id,
            model_path.display(),
            n_ctx,
            n_gpu_layers
        );

        // Check if already loaded
        if self.processes.read().await.contains_key(&model_id) {
            warn!("Model {} already loaded", model_id);
            return Ok(());
        }

        // For now, we'll use a mock implementation that doesn't actually start llama.cpp
        // In production, this would start: llama-server -m model.gguf --port PORT
        info!("Model {} marked as loaded (subprocess not started in mock mode)", model_id);

        // Store a placeholder process
        let port = self.base_port + (self.processes.read().await.len() as u16);
        let mock_child = Command::new("sleep")
            .arg("infinity")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let process = ModelProcess {
            child: mock_child,
            port,
            model_path: model_path.to_path_buf(),
        };

        self.processes
            .write()
            .await
            .insert(model_id.clone(), Arc::new(Mutex::new(process)));

        Ok(())
    }

    pub async fn unload_model(&self, model_id: &str) -> Result<()> {
        info!("Unloading model {}", model_id);

        let mut processes = self.processes.write().await;
        if let Some(process) = processes.remove(model_id) {
            let mut proc = process.lock().await;
            proc.child.kill().await.ok();
            info!("Model {} unloaded successfully", model_id);
            Ok(())
        } else {
            Err(anyhow!("Model {} not found", model_id))
        }
    }

    pub async fn generate(
        &self,
        model_id: &str,
        prompt: &str,
        _temperature: f32,
        _max_tokens: usize,
    ) -> Result<String> {
        info!("Generating response for model {}", model_id);

        let processes = self.processes.read().await;
        if !processes.contains_key(model_id) {
            return Err(anyhow!("Model {} not loaded", model_id));
        }

        // Mock implementation - in production would make HTTP request to llama-server
        Ok(format!("Generated response for prompt: {}", prompt))
    }

    pub async fn generate_stream(
        &self,
        model_id: &str,
        prompt: &str,
        _temperature: f32,
        _max_tokens: usize,
    ) -> Result<impl Stream<Item = String>> {
        info!("Generating streaming response for model {}", model_id);

        let processes = self.processes.read().await;
        if !processes.contains_key(model_id) {
            return Err(anyhow!("Model {} not loaded", model_id));
        }
        drop(processes);

        // Mock implementation - return simple token stream
        let tokens = vec![
            "Generated ".to_string(),
            "response ".to_string(),
            format!("for: {}", prompt),
        ];

        Ok(stream::iter(tokens))
    }

    pub async fn is_loaded(&self, model_id: &str) -> bool {
        self.processes.read().await.contains_key(model_id)
    }

    pub async fn generate_embeddings(
        &self,
        model_id: &str,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        info!("Generating embeddings for model {} ({} texts)", model_id, texts.len());

        let processes = self.processes.read().await;
        if !processes.contains_key(model_id) {
            return Err(anyhow!("Model {} not loaded", model_id));
        }

        // Mock implementation - return placeholder 384-dimensional embeddings
        Ok(texts.iter().map(|_| vec![0.5f32; 384]).collect())
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InferenceEngine {
    fn drop(&mut self) {
        // Kill all child processes
        if let Ok(processes) = self.processes.try_read() {
            for (_id, process) in processes.iter() {
                if let Ok(mut proc) = process.try_lock() {
                    let _ = proc.child.start_kill();
                }
            }
        }
    }
}
