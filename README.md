# LMStudio Clone - Open Source Local LLM Platform

> **Ollama meets LMStudio**: A headless Rust server with OpenAI-compatible API + lightweight Tauri desktop UI

An open-source alternative to LMStudio, built the right way:
- 🦀 **Pure Rust** headless server (no CGO nonsense)
- ⚡ **Single binary** deployment (~10-20MB)
- 🎨 **Tauri UI** - not Electron bloat (~5MB vs 200MB)
- 🔌 **Truly headless** - server works standalone
- 🌐 **OpenAI compatible** - drop-in replacement
- 💪 **Better than Ollama** - full model control like LMStudio

## Architecture

```
┌─────────────────┐         ┌──────────────┐
│   lms-server    │◄────────┤   lms-ui     │
│  (Rust binary)  │  HTTP   │ (Tauri app)  │
│                 │         │              │
│ • Axum server   │         │ • React UI   │
│ • llama.cpp-rs  │         │ • Webview    │
│ • OpenAI API    │         │ • ~5MB size  │
│ • ~10-20MB size │         │              │
└─────────────────┘         └──────────────┘
```

**Key Design Principles:**
- Server and UI are completely separate
- UI connects via HTTP (can be local or remote)
- No shared state, no IPC complexity
- Each compiles to a single static binary

## Quick Start

### Option 1: Run the Server (Headless)

```bash
# Build and run the server
cargo run --bin lms-server

# Or with options
cargo run --bin lms-server -- --port 1234 --network
```

The server will start at `http://localhost:1234` with OpenAI-compatible endpoints.

### Option 2: Run the Desktop UI

```bash
# In the lms-ui directory
cd lms-ui
npm install
npm run tauri dev
```

The UI will connect to your local server automatically.

### Option 3: Use the API Directly

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:1234/v1",
    api_key="not-needed"
)

response = client.chat.completions.create(
    model="your-model-id",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

## Project Structure

```
lmstudio-clone/
├── lms-server/          # Rust headless server
│   ├── src/
│   │   ├── main.rs     # Server entry + CLI
│   │   ├── api.rs      # OpenAI API endpoints
│   │   ├── models.rs   # Model management
│   │   ├── inference.rs# Inference engine
│   │   └── types.rs    # Shared types
│   └── Cargo.toml
│
├── lms-ui/              # Tauri desktop app
│   ├── src/            # React frontend
│   ├── src-tauri/      # Rust backend
│   └── package.json
│
└── Cargo.toml          # Workspace root
```

## Features

### Server (`lms-server`)
- ✅ OpenAI-compatible API (`/v1/*`)
- ✅ Model loading/unloading on demand
- ✅ List models (local + eventually HuggingFace)
- ✅ Chat completions (streaming + non-streaming)
- ✅ Text completions
- ✅ Server status and health checks
- 🚧 Embeddings generation
- 🚧 Model downloading
- 🚧 Actual llama.cpp-rs integration (currently placeholder)

### UI (`lms-ui`)
- ✅ Modern dark-themed interface
- ✅ Chat with loaded models
- ✅ Model browser and management
- ✅ Load/unload models with one click
- ✅ Server status monitoring
- ✅ Settings panel
- ✅ Connect to local or remote servers

## Building from Source

### Prerequisites

- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs))
- **Node.js** 18+ (for UI only)
- **(Optional)** CUDA Toolkit for NVIDIA GPU support

### Build the Server

```bash
# Development
cargo run --bin lms-server

# Release build (optimized)
cargo build --release --bin lms-server

# Binary will be at: target/release/lms-server
```

### Build the UI

```bash
cd lms-ui

# Development
npm install
npm run tauri dev

# Release build
npm run tauri build

# Binaries will be in: src-tauri/target/release/
```

## Configuration

### Server Options

```bash
lms-server --help

Options:
  -p, --port <PORT>              Port to listen on [default: 1234]
      --host <HOST>              Host to bind to [default: 127.0.0.1]
  -m, --models-dir <PATH>        Models directory [default: ~/.lmstudio-clone/models]
      --log-level <LEVEL>        Log level [default: info]
      --network                  Allow network access (bind to 0.0.0.0)
```

### Model Storage

Place your GGUF models in:
```
~/.lmstudio-clone/models/
```

The server will automatically discover them on startup.

## API Endpoints

### OpenAI Compatible

- `GET  /v1/models` - List available models
- `POST /v1/chat/completions` - Chat with streaming support
- `POST /v1/completions` - Text completion
- `POST /v1/embeddings` - Generate embeddings

### Model Management

- `POST /v1/models/load` - Load a model
- `POST /v1/models/unload` - Unload a model
- `GET  /v1/models/list` - List all models with details
- `GET  /v1/models/:id/stats` - Get model statistics
- `POST /v1/models/download` - Download from HuggingFace

### Server

- `GET /health` - Health check
- `GET /v1/status` - Server status

## Development

### Tech Stack

**Server:**
- Rust + Axum (async HTTP)
- llama-cpp-2 (LLM inference)
- Tokio (async runtime)
- Reqwest (HTTP client)

**UI:**
- Tauri 1.5 (desktop framework)
- React + TypeScript
- Tailwind CSS
- Vite (build tool)

### Why This Stack?

| Instead of... | We use... | Because... |
|--------------|-----------|------------|
| Electron | Tauri | 5MB vs 200MB |
| Node.js | Rust | Real performance |
| CGO | Pure Rust | No C dependency hell |
| Bundled everything | Separate binaries | True modularity |

## License

MIT License - see LICENSE file for details

## Acknowledgments

- [LMStudio](https://lmstudio.ai/) - Original inspiration
- [Ollama](https://ollama.ai/) - Headless server concept
- [llama.cpp](https://github.com/ggerganov/llama.cpp) - LLM inference
- [Tauri](https://tauri.app/) - Better than Electron

## Roadmap

See [FEATURES.md](FEATURES.md) for planned features.

**Current Status:** 🏗️ Alpha - Core functionality working, llama.cpp integration in progress
