use crate::types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use walkdir::WalkDir;

pub struct ModelManager {
    models_dir: PathBuf,
    models: Arc<RwLock<HashMap<String, Model>>>,
    loaded_models: Arc<RwLock<HashMap<String, ()>>>, // Placeholder for actual model instances
}

impl ModelManager {
    pub async fn new(models_dir: PathBuf) -> Result<Self> {
        let manager = Self {
            models_dir: models_dir.clone(),
            models: Arc::new(RwLock::new(HashMap::new())),
            loaded_models: Arc::new(RwLock::new(HashMap::new())),
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

    pub async fn load_model(&self, id: &str, _config: ModelConfig) -> Result<()> {
        info!("Loading model: {}", id);

        let model = self
            .models
            .read()
            .await
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", id))?
            .clone();

        // TODO: Actually load the model with llama-cpp-2
        // For now, just mark it as loaded

        let mut loaded = self.loaded_models.write().await;
        loaded.insert(id.to_string(), ());

        let mut models = self.models.write().await;
        if let Some(m) = models.get_mut(id) {
            m.loaded = true;
        }

        info!("Model loaded: {}", id);
        Ok(())
    }

    pub async fn unload_model(&self, id: &str) -> Result<()> {
        info!("Unloading model: {}", id);

        let mut loaded = self.loaded_models.write().await;
        loaded.remove(id);

        let mut models = self.models.write().await;
        if let Some(m) = models.get_mut(id) {
            m.loaded = false;
        }

        info!("Model unloaded: {}", id);
        Ok(())
    }

    pub async fn is_loaded(&self, id: &str) -> bool {
        self.loaded_models.read().await.contains_key(id)
    }

    pub async fn get_loaded_models(&self) -> Vec<String> {
        self.loaded_models.read().await.keys().cloned().collect()
    }

    pub async fn download_model(&self, _request: DownloadModelRequest) -> Result<()> {
        // TODO: Implement downloading from Hugging Face
        warn!("Model downloading not yet implemented");
        Ok(())
    }
}

fn extract_quantization(filename: &str) -> Option<String> {
    // Extract quantization from filename (e.g., q4_k_m, q5_k_s)
    let lower = filename.to_lowercase();

    if let Some(start) = lower.find("q") {
        let rest = &lower[start..];
        if let Some(end) = rest.find(|c: char| !c.is_alphanumeric() && c != '_') {
            return Some(rest[..end].to_uppercase());
        }
    }

    None
}
