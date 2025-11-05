use anyhow::{anyhow, Result};
use futures_util::stream::{self, Stream};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

pub struct ModelSession {
    model: LlamaModel,
    n_ctx: u32,
    n_threads: u32,
}

pub struct InferenceEngine {
    backend: Arc<LlamaBackend>,
    sessions: Arc<RwLock<HashMap<String, Arc<Mutex<ModelSession>>>>>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        info!("Initializing LlamaBackend");
        let backend = LlamaBackend::init().expect("Failed to initialize llama backend");

        Self {
            backend: Arc::new(backend),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
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
            "Loading model {} from {} (ctx: {}, gpu_layers: {})",
            model_id,
            model_path.display(),
            n_ctx,
            n_gpu_layers
        );

        // Check if already loaded
        if self.sessions.read().await.contains_key(&model_id) {
            warn!("Model {} already loaded", model_id);
            return Ok(());
        }

        // Create model parameters
        let mut model_params = LlamaModelParams::default();
        model_params.set_n_gpu_layers(n_gpu_layers);

        // Load the model
        let model = LlamaModel::load_from_file(&self.backend, model_path, &model_params)
            .map_err(|e| anyhow!("Failed to load model: {}", e))?;

        info!("Model {} loaded successfully", model_id);

        // Create session
        let session = ModelSession {
            model,
            n_ctx,
            n_threads,
        };

        // Store session
        self.sessions
            .write()
            .await
            .insert(model_id.clone(), Arc::new(Mutex::new(session)));

        Ok(())
    }

    pub async fn unload_model(&self, model_id: &str) -> Result<()> {
        info!("Unloading model {}", model_id);

        let mut sessions = self.sessions.write().await;
        if sessions.remove(model_id).is_some() {
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
        info!("Generating response for model {}", model_id);

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(model_id)
            .ok_or_else(|| anyhow!("Model {} not loaded", model_id))?
            .clone();

        // Drop the read lock before the blocking operation
        drop(sessions);

        // Spawn blocking task for inference
        let result = tokio::task::spawn_blocking(move || {
            let session = session.blocking_lock();
            Self::run_inference(&session, prompt, temperature, max_tokens)
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))??;

        Ok(result)
    }

    fn run_inference(
        session: &ModelSession,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> Result<String> {
        // Create context parameters
        let mut ctx_params = LlamaContextParams::default();
        ctx_params
            .set_n_ctx(Some(session.n_ctx))
            .set_n_threads(session.n_threads);

        // Create context
        let mut ctx = session
            .model
            .new_context(&session.model, ctx_params)
            .map_err(|e| anyhow!("Failed to create context: {}", e))?;

        // Tokenize prompt
        let tokens = session
            .model
            .str_to_token(prompt, llama_cpp_2::token::LlamaTokenAttr::AddBos)
            .map_err(|e| anyhow!("Failed to tokenize: {}", e))?;

        info!("Prompt tokenized to {} tokens", tokens.len());

        // Create batch
        let mut batch = LlamaBatch::new(session.n_ctx as usize, 1);

        // Add tokens to batch
        for (i, token) in tokens.iter().enumerate() {
            batch
                .add(*token, i as i32, &[0], false)
                .map_err(|e| anyhow!("Failed to add token to batch: {}", e))?;
        }

        // Mark last token for logits
        if !tokens.is_empty() {
            batch.set_last_logits_to_true();
        }

        // Decode
        ctx.decode(&mut batch)
            .map_err(|e| anyhow!("Failed to decode: {}", e))?;

        // Generate tokens
        let mut result = String::new();
        let mut n_cur = tokens.len();

        for _ in 0..max_tokens {
            // Get logits
            let candidates = ctx.candidates_ith(batch.n_tokens() - 1);

            let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);

            // Apply temperature
            ctx.sample_temp(&mut candidates_p, temperature);

            // Sample token
            let new_token_id = ctx.sample_token(&mut candidates_p);

            // Check for EOS
            if session.model.is_eog_token(new_token_id) {
                break;
            }

            // Convert token to string
            let piece = session
                .model
                .token_to_str(new_token_id)
                .map_err(|e| anyhow!("Failed to convert token: {}", e))?;

            result.push_str(&piece);

            // Prepare for next iteration
            batch.clear();
            batch
                .add(new_token_id, n_cur as i32, &[0], true)
                .map_err(|e| anyhow!("Failed to add new token: {}", e))?;

            ctx.decode(&mut batch)
                .map_err(|e| anyhow!("Failed to decode: {}", e))?;

            n_cur += 1;
        }

        Ok(result)
    }

    pub async fn generate_stream(
        &self,
        model_id: &str,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> Result<impl Stream<Item = String>> {
        info!("Generating streaming response for model {}", model_id);

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(model_id)
            .ok_or_else(|| anyhow!("Model {} not loaded", model_id))?
            .clone();
        drop(sessions);

        let prompt = prompt.to_string();

        // Create a channel for streaming tokens
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(32);

        // Spawn inference task
        tokio::task::spawn_blocking(move || {
            let session = session.blocking_lock();

            // Similar to run_inference but send tokens as they're generated
            match Self::run_inference_streaming(&session, &prompt, temperature, max_tokens, tx) {
                Ok(_) => info!("Streaming inference completed"),
                Err(e) => error!("Streaming inference failed: {}", e),
            }
        });

        // Convert mpsc receiver to stream
        Ok(stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|token| (token, rx))
        }))
    }

    fn run_inference_streaming(
        session: &ModelSession,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
        tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<()> {
        // Similar to run_inference but sends tokens through channel
        let mut ctx_params = LlamaContextParams::default();
        ctx_params
            .set_n_ctx(Some(session.n_ctx))
            .set_n_threads(session.n_threads);

        let mut ctx = session
            .model
            .new_context(&session.model, ctx_params)
            .map_err(|e| anyhow!("Failed to create context: {}", e))?;

        let tokens = session
            .model
            .str_to_token(prompt, llama_cpp_2::token::LlamaTokenAttr::AddBos)
            .map_err(|e| anyhow!("Failed to tokenize: {}", e))?;

        let mut batch = LlamaBatch::new(session.n_ctx as usize, 1);

        for (i, token) in tokens.iter().enumerate() {
            batch
                .add(*token, i as i32, &[0], false)
                .map_err(|e| anyhow!("Failed to add token: {}", e))?;
        }

        if !tokens.is_empty() {
            batch.set_last_logits_to_true();
        }

        ctx.decode(&mut batch)
            .map_err(|e| anyhow!("Failed to decode: {}", e))?;

        let mut n_cur = tokens.len();

        for _ in 0..max_tokens {
            let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
            let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);

            ctx.sample_temp(&mut candidates_p, temperature);
            let new_token_id = ctx.sample_token(&mut candidates_p);

            if session.model.is_eog_token(new_token_id) {
                break;
            }

            let piece = session
                .model
                .token_to_str(new_token_id)
                .map_err(|e| anyhow!("Failed to convert token: {}", e))?;

            // Send token through channel
            if tx.blocking_send(piece).is_err() {
                // Channel closed, stop generating
                break;
            }

            batch.clear();
            batch
                .add(new_token_id, n_cur as i32, &[0], true)
                .map_err(|e| anyhow!("Failed to add token: {}", e))?;

            ctx.decode(&mut batch)
                .map_err(|e| anyhow!("Failed to decode: {}", e))?;

            n_cur += 1;
        }

        Ok(())
    }

    pub async fn is_loaded(&self, model_id: &str) -> bool {
        self.sessions.read().await.contains_key(model_id)
    }

    pub async fn generate_embeddings(
        &self,
        model_id: &str,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        info!("Generating embeddings for model {} ({} texts)", model_id, texts.len());

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(model_id)
            .ok_or_else(|| anyhow!("Model {} not loaded", model_id))?
            .clone();
        drop(sessions);

        // Spawn blocking task for embeddings generation
        let result = tokio::task::spawn_blocking(move || {
            let session = session.blocking_lock();
            Self::run_embeddings(&session, texts)
        })
        .await
        .map_err(|e| anyhow!("Task join error: {}", e))??;

        Ok(result)
    }

    fn run_embeddings(
        session: &ModelSession,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        let mut all_embeddings = Vec::new();

        for text in texts {
            // Create context parameters with pooling for embeddings
            let mut ctx_params = LlamaContextParams::default();
            ctx_params
                .set_n_ctx(Some(session.n_ctx))
                .set_n_threads(session.n_threads)
                .set_embeddings(true);

            // Create context
            let mut ctx = session
                .model
                .new_context(&session.model, ctx_params)
                .map_err(|e| anyhow!("Failed to create context: {}", e))?;

            // Tokenize text
            let tokens = session
                .model
                .str_to_token(&text, llama_cpp_2::token::LlamaTokenAttr::AddBos)
                .map_err(|e| anyhow!("Failed to tokenize: {}", e))?;

            // Create batch
            let mut batch = LlamaBatch::new(session.n_ctx as usize, 1);

            // Add tokens to batch
            for (i, token) in tokens.iter().enumerate() {
                batch
                    .add(*token, i as i32, &[0], false)
                    .map_err(|e| anyhow!("Failed to add token to batch: {}", e))?;
            }

            // Decode to get embeddings
            ctx.decode(&mut batch)
                .map_err(|e| anyhow!("Failed to decode: {}", e))?;

            // Get embeddings from context
            let embeddings = ctx.embeddings_ith(0);

            if embeddings.is_empty() {
                return Err(anyhow!("Failed to generate embeddings: empty result"));
            }

            all_embeddings.push(embeddings);
        }

        Ok(all_embeddings)
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}
