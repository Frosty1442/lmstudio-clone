// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerConfig {
    url: String,
}

struct AppState {
    server_url: Arc<Mutex<String>>,
    client: reqwest::Client,
}

#[tauri::command]
async fn get_models(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let url = state.server_url.lock().await;
    let response = state
        .client
        .get(format!("{}/v1/models/list", url))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn load_model(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<serde_json::Value, String> {
    let url = state.server_url.lock().await;
    let response = state
        .client
        .post(format!("{}/v1/models/load", url))
        .json(&serde_json::json!({ "model_id": model_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn unload_model(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<serde_json::Value, String> {
    let url = state.server_url.lock().await;
    let response = state
        .client
        .post(format!("{}/v1/models/unload", url))
        .json(&serde_json::json!({ "model_id": model_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_message(
    state: tauri::State<'_, AppState>,
    model: String,
    messages: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let url = state.server_url.lock().await;
    let response = state
        .client
        .post(format!("{}/v1/chat/completions", url))
        .json(&serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_server_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let url = state.server_url.lock().await;
    let response = state
        .client
        .get(format!("{}/v1/status", url))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    response.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_server_url(state: tauri::State<'_, AppState>, url: String) -> Result<(), String> {
    let mut server_url = state.server_url.lock().await;
    *server_url = url;
    Ok(())
}

fn main() {
    let app_state = AppState {
        server_url: Arc::new(Mutex::new("http://localhost:1234".to_string())),
        client: reqwest::Client::new(),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_models,
            load_model,
            unload_model,
            send_message,
            get_server_status,
            set_server_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
