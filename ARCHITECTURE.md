# Architecture Document

## System Overview

This application is built as a multi-process Electron application with the following components:

```
┌─────────────────────────────────────────────────────────────┐
│                     Electron Main Process                    │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────┐ │
│  │ Window Manager │  │ IPC Handler  │  │ Config Manager  │ │
│  └────────────────┘  └──────────────┘  └─────────────────┘ │
└───────────────────────────┬─────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼────────┐  ┌───────▼────────┐  ┌──────▼───────┐
│   Renderer     │  │   API Server   │  │  CLI Tool    │
│   (React UI)   │  │   (Express)    │  │  (Node.js)   │
└───────┬────────┘  └───────┬────────┘  └──────┬───────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
                    ┌───────▼────────┐
                    │  Inference     │
                    │  Engine Core   │
                    │  (llama.cpp)   │
                    └────────────────┘
```

## Core Components

### 1. Main Process (Electron)

**Responsibilities:**
- Application lifecycle management
- Window creation and management
- IPC communication between renderer and backend
- System tray integration
- Auto-updater
- Configuration persistence

**Key Modules:**
- `WindowManager`: Manages application windows
- `IPCHandler`: Routes IPC messages
- `ConfigManager`: Handles app configuration
- `TrayManager`: System tray icon and menu

### 2. Renderer Process (React Frontend)

**Responsibilities:**
- User interface rendering
- User interaction handling
- State management
- Communication with main process via IPC

**Key Components:**
- **Chat Interface**: Conversation UI with message history
- **Model Browser**: Search and download models
- **Settings Panel**: Configuration UI
- **Model Manager**: View and manage loaded models
- **Theme Manager**: Handle UI themes

**Tech Stack:**
- React 18
- TypeScript
- Tailwind CSS
- Zustand (state management)
- React Router
- shadcn/ui components

### 3. API Server (Express)

**Responsibilities:**
- OpenAI-compatible REST API
- WebSocket connections for streaming
- Request queuing and rate limiting
- Model lifecycle management
- Authentication (optional)

**Endpoints:**
- `GET /v1/models` - List models
- `POST /v1/chat/completions` - Chat completions
- `POST /v1/completions` - Text completions
- `POST /v1/embeddings` - Generate embeddings
- `GET /health` - Health check
- `POST /v1/models/load` - Load model
- `POST /v1/models/unload` - Unload model

### 4. Inference Engine Core

**Responsibilities:**
- Model loading and unloading
- Inference execution
- GPU/CPU offload management
- Memory management
- Model format support (GGUF, MLX)

**Key Modules:**
- `ModelLoader`: Load/unload models
- `InferenceEngine`: Execute inference
- `TokenizerManager`: Handle tokenization
- `MemoryManager`: Manage VRAM/RAM allocation
- `GPUManager`: Handle GPU offloading

### 5. CLI Tool

**Responsibilities:**
- Command-line interface for automation
- Server control
- Model management
- Scripting support

**Commands:**
- `status` - Show server status
- `start` - Start server
- `stop` - Stop server
- `load <model>` - Load model
- `unload <model>` - Unload model
- `ls` - List models
- `ps` - List loaded models
- `get <url>` - Download model

## Data Flow

### Chat Message Flow

```
User Input → Renderer → IPC → Main Process → API Server
                                                  ↓
                                           Inference Engine
                                                  ↓
                                             llama.cpp
                                                  ↓
Renderer ← IPC ← Main Process ← API Server ← Token Stream
```

### Model Download Flow

```
User Search → Renderer → Model Browser API
                              ↓
                      Hugging Face API
                              ↓
                     Download Manager
                              ↓
                      Local Storage
                              ↓
                      Model Index Update
```

## Storage Architecture

### File Structure

```
~/.lmstudio-clone/
├── models/                    # Downloaded models
│   ├── <model-id>/
│   │   ├── model.gguf
│   │   └── metadata.json
├── chats/                     # Chat histories
│   ├── conversations.db       # SQLite database
├── config/
│   ├── app.json              # App configuration
│   ├── models.json           # Model configurations
│   └── presets.json          # User presets
├── embeddings/               # Embedding cache
└── logs/                     # Application logs
```

### Database Schema

**Conversations Table:**
```sql
CREATE TABLE conversations (
  id TEXT PRIMARY KEY,
  title TEXT,
  created_at INTEGER,
  updated_at INTEGER,
  folder_id TEXT,
  model_id TEXT,
  system_prompt TEXT
);
```

**Messages Table:**
```sql
CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT,
  role TEXT,
  content TEXT,
  created_at INTEGER,
  tokens INTEGER,
  parent_id TEXT,
  FOREIGN KEY(conversation_id) REFERENCES conversations(id)
);
```

**Folders Table:**
```sql
CREATE TABLE folders (
  id TEXT PRIMARY KEY,
  name TEXT,
  parent_id TEXT,
  created_at INTEGER
);
```

## Technology Stack

### Backend
- **Runtime**: Node.js 18+
- **Language**: TypeScript
- **Framework**: Electron
- **API Server**: Express.js
- **Database**: SQLite (better-sqlite3)
- **LLM Engine**: llama.cpp (via node-llama-cpp)

### Frontend
- **Framework**: React 18
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **Components**: shadcn/ui
- **State**: Zustand
- **Routing**: React Router

### Build Tools
- **Bundler**: Vite
- **Electron Builder**: electron-builder
- **Compiler**: TypeScript, SWC

## Module Dependencies

```
@electron/main
  ├── express (API Server)
  ├── better-sqlite3 (Database)
  └── node-llama-cpp (Inference)

@renderer/ui
  ├── react
  ├── react-router-dom
  ├── zustand
  └── tailwindcss

@shared/types
  └── TypeScript definitions
```

## Communication Protocols

### IPC (Inter-Process Communication)

**Main → Renderer:**
- `model:loaded` - Model loaded successfully
- `model:error` - Model loading error
- `chat:token` - New token generated
- `download:progress` - Download progress update

**Renderer → Main:**
- `model:load` - Request model load
- `chat:send` - Send chat message
- `model:download` - Download model
- `config:update` - Update configuration

### API Protocol (OpenAI Compatible)

**Request Format:**
```json
{
  "model": "model-name",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "stream": true,
  "temperature": 0.7,
  "max_tokens": 2048
}
```

**Response Format (Streaming):**
```
data: {"id":"123","object":"chat.completion.chunk","choices":[{"delta":{"content":"Hello"}}]}

data: [DONE]
```

## Security Considerations

1. **Input Validation**: All API inputs validated
2. **Path Traversal Prevention**: Sanitize file paths
3. **Resource Limits**: Limit memory and CPU usage
4. **Sandboxing**: Renderer process sandboxed
5. **CSP**: Content Security Policy enforced
6. **No Telemetry**: Completely offline operation

## Performance Optimization

1. **Lazy Loading**: Load components on demand
2. **Memoization**: Cache expensive computations
3. **Worker Threads**: Offload heavy processing
4. **Streaming**: Stream responses for better UX
5. **Connection Pooling**: Reuse connections
6. **Model Caching**: Keep frequently used models in memory

## Scaling Strategy

1. **Multiple Models**: Support loading multiple models
2. **Queue Management**: Queue inference requests
3. **Load Balancing**: Distribute requests across models
4. **Resource Management**: Monitor and limit resource usage
5. **Graceful Degradation**: Fall back to CPU if GPU unavailable

## Error Handling

1. **Model Loading Errors**: Retry with fallback
2. **Inference Errors**: Graceful error messages
3. **Network Errors**: Retry with exponential backoff
4. **Memory Errors**: Auto-unload least used models
5. **Validation Errors**: Clear user feedback

## Testing Strategy

1. **Unit Tests**: Jest for individual modules
2. **Integration Tests**: Test component interactions
3. **E2E Tests**: Playwright for full workflows
4. **Performance Tests**: Benchmark inference speed
5. **Load Tests**: Test under high load

## Deployment

1. **Desktop Installers**: electron-builder
2. **Auto-Updates**: Electron auto-updater
3. **Crash Reporting**: Optional crash reports
4. **Versioning**: Semantic versioning
5. **Release Channels**: Stable, beta, nightly
