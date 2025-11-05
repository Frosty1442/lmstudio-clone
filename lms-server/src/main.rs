use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod inference;
mod models;
mod types;

#[derive(Parser, Debug)]
#[command(name = "lms-server")]
#[command(about = "LMStudio Clone - Headless LLM Server", long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "1234")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Models directory
    #[arg(short, long)]
    models_dir: Option<PathBuf>,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Allow network access (bind to 0.0.0.0)
    #[arg(long)]
    network: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(match args.log_level.as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Get models directory
    let models_dir = args.models_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".lmstudio-clone")
            .join("models")
    });

    // Ensure models directory exists
    std::fs::create_dir_all(&models_dir)?;
    info!("Models directory: {}", models_dir.display());

    // Initialize model manager
    let model_manager = models::ModelManager::new(models_dir).await?;

    // Initialize inference engine
    let inference_engine = inference::InferenceEngine::new();

    // Create app state
    let state = api::AppState {
        model_manager,
        inference_engine,
    };

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(api::health))
        .route("/v1/models", get(api::list_models))
        // OpenAI compatible endpoints
        .route("/v1/chat/completions", post(api::chat_completions))
        .route("/v1/completions", post(api::completions))
        .route("/v1/embeddings", post(api::embeddings))
        // Model management endpoints
        .route("/v1/models/load", post(api::load_model))
        .route("/v1/models/unload", post(api::unload_model))
        .route("/v1/models/download", post(api::download_model))
        .route("/v1/models/list", get(api::list_all_models))
        .route("/v1/models/:id/stats", get(api::model_stats))
        // Status endpoint
        .route("/v1/status", get(api::server_status))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Determine bind address
    let host = if args.network {
        "0.0.0.0".to_string()
    } else {
        args.host
    };

    let addr: SocketAddr = format!("{}:{}", host, args.port).parse()?;

    info!("🚀 LMS Server starting on http://{}", addr);
    info!("📡 OpenAI API compatible endpoints available at /v1/*");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
