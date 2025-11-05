use crate::inference::InferenceEngine;
use crate::types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use walkdir::WalkDir;

pub struct ModelManager {
    models_dir: PathBuf,
    models: Arc<RwLock<HashMap<String, Model>>>,
    inference_engine: Arc<InferenceEngine>,
}

impl ModelManager {
    pub async fn new(models_dir: PathBuf, inference_engine: Arc<InferenceEngine>) -> Result<Self> {
        let manager = Self {
            models_dir: models_dir.clone(),
            models: Arc::new(RwLock::new(HashMap::new())),
            inference_engine,
        };

        // Scan for existing models
        manager.scan_models().await?;

        Ok(manager)
    }

    async fn scan_models(&self) -> Result<()> {
        info!("Scanning models directory: {}", self.models_dir.display());

        let mut models = self.models.write().await;
        let mut count = 0;

        for entry in WalkDir::new(&self.models_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();

                // Check if it's a GGUF or GGML file
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "gguf" || ext_str == "ggml" || ext_str == "bin" {
                        if let Ok(metadata) = entry.metadata() {
                            let file_name = path.file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown");

                            let model_id = format!("{}-{}",
                                file_name,
                                path.parent()
                                    .and_then(|p| p.file_name())
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                            );

                            let model = Model {
                                id: model_id.clone(),
                                name: file_name.to_string(),
                                path: path.to_string_lossy().to_string(),
                                size: metadata.len(),
                                quantization: extract_quantization(file_name),
                                loaded: false,
                                format: if ext_str == "gguf" {
                                    ModelFormat::GGUF
                                } else {
                                    ModelFormat::GGML
                                },
                            };

                            models.insert(model_id, model);
                            count += 1;
                        }
                    }
                }
            }
        }

        info!("Found {} models", count);
        Ok(())
    }

    pub async fn list_models(&self) -> Vec<Model> {
        self.models.read().await.values().cloned().collect()
    }

    pub async fn get_model(&self, id: &str) -> Option<Model> {
        self.models.read().await.get(id).cloned()
    }

    pub async fn load_model(&self, id: &str, config: ModelConfig) -> Result<()> {
        info!("Loading model: {}", id);

        let model = self
            .models
            .read()
            .await
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", id))?
            .clone();

        // Load the model with the inference engine
        self.inference_engine
            .load_model(
                id.to_string(),
                Path::new(&model.path),
                config.context_size as u32,
                config.gpu_layers,
                config.threads,
            )
            .await?;

        // Mark as loaded
        let mut models = self.models.write().await;
        if let Some(m) = models.get_mut(id) {
            m.loaded = true;
        }

        info!("Model loaded: {}", id);
        Ok(())
    }

    pub async fn unload_model(&self, id: &str) -> Result<()> {
        info!("Unloading model: {}", id);

        // Unload from inference engine
        self.inference_engine.unload_model(id).await?;

        // Mark as unloaded
        let mut models = self.models.write().await;
        if let Some(m) = models.get_mut(id) {
            m.loaded = false;
        }

        info!("Model unloaded: {}", id);
        Ok(())
    }

    pub async fn is_loaded(&self, id: &str) -> bool {
        self.inference_engine.is_loaded(id).await
    }

    pub async fn get_loaded_models(&self) -> Vec<String> {
        let models = self.models.read().await;
        models
            .values()
            .filter(|m| m.loaded)
            .map(|m| m.id.clone())
            .collect()
    }

    pub async fn download_model(&self, request: DownloadModelRequest) -> Result<()> {
        info!("Downloading model from {}", request.repo);

        // Determine filename
        let filename = if let Some(f) = request.filename {
            f
        } else {
            // Try to find a GGUF file in the repo
            let files = self.list_repo_files(&request.repo).await?;
            let gguf_files: Vec<_> = files.iter().filter(|f| f.ends_with(".gguf")).collect();

            if gguf_files.is_empty() {
                return Err(anyhow::anyhow!("No GGUF files found in repository"));
            }

            // If quantization specified, find matching file
            if let Some(quant) = &request.quantization {
                gguf_files
                    .iter()
                    .find(|f| f.to_lowercase().contains(&quant.to_lowercase()))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| gguf_files[0].to_string())
            } else {
                // Use first GGUF file
                gguf_files[0].to_string()
            }
        };

        info!("Downloading file: {}", filename);

        // Construct Hugging Face URL
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            request.repo, filename
        );

        // Create target directory
        let model_id = format!(
            "{}-{}",
            request.repo.replace("/", "-"),
            filename.replace(".gguf", "")
        );
        let target_dir = self.models_dir.join(&model_id);
        std::fs::create_dir_all(&target_dir)?;

        let target_path = target_dir.join(&filename);

        // Download the file
        info!("Downloading from {}", url);
        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download: HTTP {}",
                response.status()
            ));
        }

        let mut file = tokio::fs::File::create(&target_path).await?;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        info!("Download complete: {}", target_path.display());

        // Rescan models to pick up the new one
        self.scan_models().await?;

        Ok(())
    }

    async fn list_repo_files(&self, repo: &str) -> Result<Vec<String>> {
        // Use Hugging Face API to list files
        let url = format!("https://huggingface.co/api/models/{}", repo);
        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch repo info: HTTP {}",
                response.status()
            ));
        }

        let json: serde_json::Value = response.json().await?;

        // Extract file names from siblings array
        let files: Vec<String> = json["siblings"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|s| s["rfilename"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(files)
    }

    // Expose inference engine methods
    pub fn inference_engine(&self) -> &Arc<InferenceEngine> {
        &self.inference_engine
    }
}

fn extract_quantization(filename: &str) -> Option<String> {
    // Extract quantization from filename (e.g., q4_k_m, q5_k_s)
    // Pattern: q followed by a digit, then optionally followed by alphanumeric or underscore
    let lower = filename.to_lowercase();

    // Find 'q' followed by a digit
    for (i, _) in lower.char_indices() {
        if let Some(rest) = lower.get(i..) {
            if rest.starts_with('q') && rest.len() > 1 {
                if let Some(next_char) = rest.chars().nth(1) {
                    if next_char.is_numeric() {
                        // Found valid quantization pattern
                        let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
                            .unwrap_or(rest.len());
                        return Some(rest[..end].to_uppercase());
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_quantization() {
        assert_eq!(extract_quantization("model-q4_k_m.gguf"), Some("Q4_K_M".to_string()));
        assert_eq!(extract_quantization("llama-2-7b-q5_k_s.gguf"), Some("Q5_K_S".to_string()));
        assert_eq!(extract_quantization("model-Q8_0.gguf"), Some("Q8_0".to_string()));
        assert_eq!(extract_quantization("model.gguf"), None);
        assert_eq!(extract_quantization("no-quant-here"), None);
    }

    #[tokio::test]
    async fn test_model_manager_scan_models() {
        let temp_dir = TempDir::new().unwrap();
        let models_path = temp_dir.path().to_path_buf();

        // Create a fake GGUF file
        let model_dir = models_path.join("test-model");
        std::fs::create_dir_all(&model_dir).unwrap();
        let model_file = model_dir.join("model-q4_k_m.gguf");
        std::fs::write(&model_file, b"fake model data").unwrap();

        let inference_engine = Arc::new(InferenceEngine::new());
        let manager = ModelManager::new(models_path, inference_engine).await.unwrap();

        let models = manager.list_models().await;
        assert_eq!(models.len(), 1);
        assert!(models[0].id.contains("model-q4_k_m"));
        assert_eq!(models[0].quantization, Some("Q4_K_M".to_string()));
        assert_eq!(models[0].format, ModelFormat::GGUF);
        assert!(!models[0].loaded);
    }

    #[tokio::test]
    async fn test_model_manager_get_model() {
        let temp_dir = TempDir::new().unwrap();
        let models_path = temp_dir.path().to_path_buf();

        // Create a fake GGUF file
        let model_file = models_path.join("test.gguf");
        std::fs::create_dir_all(&models_path).unwrap();
        std::fs::write(&model_file, b"fake model").unwrap();

        let inference_engine = Arc::new(InferenceEngine::new());
        let manager = ModelManager::new(models_path, inference_engine).await.unwrap();

        let models = manager.list_models().await;
        assert!(!models.is_empty());

        let model_id = &models[0].id;
        let retrieved = manager.get_model(model_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, *model_id);

        let not_found = manager.get_model("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_model_manager_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let models_path = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(&models_path).unwrap();

        let inference_engine = Arc::new(InferenceEngine::new());
        let manager = ModelManager::new(models_path, inference_engine).await.unwrap();

        let models = manager.list_models().await;
        assert_eq!(models.len(), 0);
    }

    #[tokio::test]
    async fn test_model_manager_multiple_formats() {
        let temp_dir = TempDir::new().unwrap();
        let models_path = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(&models_path).unwrap();

        // Create files with different formats
        std::fs::write(models_path.join("model1.gguf"), b"data").unwrap();
        std::fs::write(models_path.join("model2.ggml"), b"data").unwrap();
        std::fs::write(models_path.join("model3.bin"), b"data").unwrap();
        std::fs::write(models_path.join("readme.txt"), b"ignore").unwrap();

        let inference_engine = Arc::new(InferenceEngine::new());
        let manager = ModelManager::new(models_path, inference_engine).await.unwrap();

        let models = manager.list_models().await;
        assert_eq!(models.len(), 3); // Only gguf, ggml, and bin files
    }

    #[tokio::test]
    async fn test_get_loaded_models() {
        let temp_dir = TempDir::new().unwrap();
        let models_path = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(&models_path).unwrap();

        let inference_engine = Arc::new(InferenceEngine::new());
        let manager = ModelManager::new(models_path, inference_engine).await.unwrap();

        let loaded = manager.get_loaded_models().await;
        assert_eq!(loaded.len(), 0);
    }
}
