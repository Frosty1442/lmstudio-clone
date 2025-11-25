use anyhow::{anyhow, Result};
use futures_util::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum AcceleratorType {
    CUDA,
    ROCm,
    Metal,
    Vulkan,
    CPU,
}

#[derive(Debug, Serialize, Deserialize)]
struct LlamaCompletionRequest {
    prompt: String,
    n_predict: i32,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_keep: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LlamaCompletionResponse {
    content: String,
    #[serde(default)]
    stop: bool,
}

/// Response from completion with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub tokens_used: Option<i32>,
}

pub struct ModelProcess {
    child: Child,
    port: u16,
    model_path: PathBuf,
    accelerator: AcceleratorType,
}

pub struct InferenceEngine {
    processes: Arc<RwLock<HashMap<String, Arc<Mutex<ModelProcess>>>>>,
    base_port: u16,
    llama_server_path: PathBuf,
    accelerator: AcceleratorType,
    http_client: reqwest::Client,
}

impl InferenceEngine {
    pub fn new() -> Self {
        info!("Initializing InferenceEngine with subprocess architecture");

        let accelerator = Self::detect_accelerator();
        info!("Detected accelerator: {:?}", accelerator);

        let llama_server_path = Self::find_or_download_llama_server(&accelerator);
        info!("Using llama-server at: {:?}", llama_server_path);

        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            base_port: 8080,
            llama_server_path,
            accelerator,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
        }
    }

    fn detect_accelerator() -> AcceleratorType {
        // Check for NVIDIA CUDA
        if std::process::Command::new("nvidia-smi")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            info!("NVIDIA GPU detected via nvidia-smi");
            return AcceleratorType::CUDA;
        }

        // Check for AMD ROCm
        if std::path::Path::new("/opt/rocm").exists() {
            info!("AMD ROCm detected");
            return AcceleratorType::ROCm;
        }

        // Check for Apple Metal (macOS)
        #[cfg(target_os = "macos")]
        {
            info!("macOS detected, using Metal");
            return AcceleratorType::Metal;
        }

        // Check for Vulkan
        if std::process::Command::new("vulkaninfo")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            info!("Vulkan detected");
            return AcceleratorType::Vulkan;
        }

        info!("No GPU acceleration detected, using CPU");
        AcceleratorType::CPU
    }

    fn find_or_download_llama_server(accelerator: &AcceleratorType) -> PathBuf {
        // Check if llama-server is in PATH
        if let Ok(output) = std::process::Command::new("which").arg("llama-server").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    info!("Found llama-server in PATH: {}", path);
                    return PathBuf::from(path);
                }
            }
        }

        // Check common installation locations
        let common_paths = vec![
            "/usr/local/bin/llama-server",
            "/usr/bin/llama-server",
            "./llama-server",
            "./bin/llama-server",
            "~/.local/bin/llama-server",
        ];

        for path in common_paths {
            let expanded = shellexpand::tilde(path).to_string();
            if std::path::Path::new(&expanded).exists() {
                info!("Found llama-server at: {}", expanded);
                return PathBuf::from(expanded);
            }
        }

        // For now, return a default path - in production, would download from GitHub releases
        warn!(
            "llama-server not found, using default path (may not exist): ./llama-server-{:?}",
            accelerator
        );
        PathBuf::from(format!("./llama-server-{:?}", accelerator).to_lowercase())
    }

    pub async fn load_model(
        &self,
        model_id: String,
        model_path: &Path,
        n_ctx: u32,
        n_gpu_layers: u32,
        n_threads: u32,
    ) -> Result<()> {
        info!(
            "Loading model {} from {} (ctx: {}, gpu_layers: {}, threads: {})",
            model_id,
            model_path.display(),
            n_ctx,
            n_gpu_layers,
            n_threads
        );

        // Check if already loaded
        if self.processes.read().await.contains_key(&model_id) {
            warn!("Model {} already loaded", model_id);
            return Ok(());
        }

        // Verify model file exists
        if !model_path.exists() {
            return Err(anyhow!("Model file not found: {}", model_path.display()));
        }

        // Assign a port for this model
        let port = self.base_port + (self.processes.read().await.len() as u16);

        // Build llama-server command
        let mut cmd = Command::new(&self.llama_server_path);
        cmd.arg("--model")
            .arg(model_path)
            .arg("--port")
            .arg(port.to_string())
            .arg("--ctx-size")
            .arg(n_ctx.to_string())
            .arg("--threads")
            .arg(n_threads.to_string());

        // Add GPU layers if applicable
        if self.accelerator != AcceleratorType::CPU && n_gpu_layers > 0 {
            cmd.arg("--n-gpu-layers").arg(n_gpu_layers.to_string());
        }

        // Configure stdio
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!(
            "Starting llama-server on port {} with accelerator {:?}",
            port, self.accelerator
        );

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| {
            error!("Failed to spawn llama-server: {}", e);
            anyhow!("Failed to spawn llama-server: {}. Is llama-server installed?", e)
        })?;

        // Capture stdout/stderr for logging
        if let Some(stdout) = child.stdout.take() {
            let model_id_clone = model_id.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    info!("[llama-server:{}] {}", model_id_clone, line);
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let model_id_clone = model_id.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    warn!("[llama-server:{}] {}", model_id_clone, line);
                }
            });
        }

        // Wait for server to be ready
        let health_url = format!("http://localhost:{}/health", port);
        info!("Waiting for llama-server to be ready at {}", health_url);

        for attempt in 1..=30 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            match self.http_client.get(&health_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("llama-server ready on port {} (attempt {})", port, attempt);
                    break;
                }
                Ok(resp) => {
                    warn!(
                        "llama-server returned status {} (attempt {})",
                        resp.status(),
                        attempt
                    );
                }
                Err(e) if attempt == 30 => {
                    child.kill().await.ok();
                    return Err(anyhow!(
                        "llama-server failed to become ready after 15s: {}",
                        e
                    ));
                }
                Err(_) => {
                    // Server not ready yet, continue waiting
                }
            }
        }

        let process = ModelProcess {
            child,
            port,
            model_path: model_path.to_path_buf(),
            accelerator: self.accelerator.clone(),
        };

        self.processes
            .write()
            .await
            .insert(model_id.clone(), Arc::new(Mutex::new(process)));

        info!("Model {} loaded successfully on port {}", model_id, port);
        Ok(())
    }

    pub async fn unload_model(&self, model_id: &str) -> Result<()> {
        info!("Unloading model {}", model_id);

        let mut processes = self.processes.write().await;
        if let Some(process) = processes.remove(model_id) {
            let mut proc = process.lock().await;
            proc.child.kill().await?;
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
        temperature: f32,
        max_tokens: usize,
    ) -> Result<String> {
        let response = self
            .generate_completion(model_id, prompt, temperature, max_tokens as i32)
            .await?;
        Ok(response.content)
    }

    pub async fn generate_completion(
        &self,
        model_id: &str,
        prompt: &str,
        temperature: f32,
        max_tokens: i32,
    ) -> Result<CompletionResponse> {
        info!("Generating response for model {}", model_id);

        let processes = self.processes.read().await;
        let process = processes
            .get(model_id)
            .ok_or_else(|| anyhow!("Model {} not loaded", model_id))?;
        let proc = process.lock().await;
        let port = proc.port;
        drop(proc);
        drop(processes);

        let url = format!("http://localhost:{}/completion", port);
        let request = LlamaCompletionRequest {
            prompt: prompt.to_string(),
            n_predict: max_tokens,
            temperature,
            stream: false,
            n_keep: None,
        };

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("llama-server returned error {}: {}", status, text));
        }

        let completion: LlamaCompletionResponse = response.json().await?;

        // Note: llama-server doesn't return token counts in the basic response
        // This could be enhanced by parsing the verbose output
        Ok(CompletionResponse {
            content: completion.content,
            tokens_used: None,
        })
    }

    pub async fn generate_stream(
        &self,
        model_id: &str,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> Result<impl Stream<Item = String>> {
        info!("Generating streaming response for model {}", model_id);

        let processes = self.processes.read().await;
        let process = processes
            .get(model_id)
            .ok_or_else(|| anyhow!("Model {} not loaded", model_id))?;
        let proc = process.lock().await;
        let port = proc.port;
        drop(proc);
        drop(processes);

        let url = format!("http://localhost:{}/completion", port);
        let request = LlamaCompletionRequest {
            prompt: prompt.to_string(),
            n_predict: max_tokens as i32,
            temperature,
            stream: true,
            n_keep: None,
        };

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("llama-server returned error {}: {}", status, text));
        }

        // Parse SSE stream from llama-server
        let stream = response.bytes_stream().map(|chunk_result| {
            chunk_result
                .ok()
                .and_then(|chunk| {
                    let text = String::from_utf8_lossy(&chunk).to_string();
                    // Parse SSE format: "data: {...}\n\n"
                    text.lines()
                        .filter(|line| line.starts_with("data: "))
                        .filter_map(|line| {
                            let json_str = line.strip_prefix("data: ")?;
                            serde_json::from_str::<LlamaCompletionResponse>(json_str).ok()
                        })
                        .map(|resp| resp.content)
                        .next()
                })
                .unwrap_or_default()
        });

        Ok(stream)
    }

    pub async fn is_loaded(&self, model_id: &str) -> bool {
        self.processes.read().await.contains_key(model_id)
    }

    pub async fn generate_embeddings(
        &self,
        model_id: &str,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        info!(
            "Generating embeddings for model {} ({} texts)",
            model_id,
            texts.len()
        );

        let processes = self.processes.read().await;
        let process = processes
            .get(model_id)
            .ok_or_else(|| anyhow!("Model {} not loaded", model_id))?;
        let proc = process.lock().await;
        let port = proc.port;
        drop(proc);
        drop(processes);

        let url = format!("http://localhost:{}/embedding", port);

        let mut all_embeddings = Vec::new();
        for text in texts {
            let request = serde_json::json!({
                "content": text,
            });

            let response = self
                .http_client
                .post(&url)
                .json(&request)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(anyhow!("llama-server returned error {}: {}", status, text));
            }

            let result: serde_json::Value = response.json().await?;
            if let Some(embedding) = result["embedding"].as_array() {
                let embedding_vec: Vec<f32> = embedding
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
                all_embeddings.push(embedding_vec);
            } else {
                return Err(anyhow!("Invalid embedding response format"));
            }
        }

        Ok(all_embeddings)
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
