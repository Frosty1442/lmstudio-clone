# API Documentation

LMStudio Clone provides an OpenAI-compatible REST API for local LLM inference with RAG capabilities.

## Base URL

```
http://localhost:1234
```

## Authentication

Authentication is optional but recommended for production deployments.

### API Key Authentication

Set the `LMS_API_KEY` or `LMS_API_KEYS` environment variable to enable authentication.

```bash
# Using Authorization header (recommended)
curl -H "Authorization: Bearer your-api-key" http://localhost:1234/v1/models

# Using X-API-Key header
curl -H "X-API-Key: your-api-key" http://localhost:1234/v1/models
```

## Rate Limiting

When enabled, rate limit information is included in response headers:

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests per minute |
| `X-RateLimit-Remaining` | Remaining requests in window |
| `Retry-After` | Seconds until limit resets (when limited) |

## Endpoints

### Health & Status

#### GET /health

Check server health.

**Response:**
```json
{
  "status": "ok"
}
```

#### GET /v1/status

Get detailed server status.

**Response:**
```json
{
  "status": "running",
  "version": "1.0.0",
  "uptime": 3600,
  "loaded_models": ["llama-3.2-1b"],
  "total_models": 5,
  "workspaces": 3
}
```

#### GET /metrics

Prometheus-format metrics endpoint.

**Response:**
```
# HELP lms_uptime_seconds Server uptime in seconds
# TYPE lms_uptime_seconds gauge
lms_uptime_seconds 3600

# HELP lms_requests_total Total HTTP requests
# TYPE lms_requests_total counter
lms_requests_total{endpoint="/v1/chat/completions",status="200"} 42
...
```

---

### Models

#### GET /v1/models

List loaded models (OpenAI compatible).

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "llama-3.2-1b-Q4_K_M",
      "object": "model",
      "created": 1699999999,
      "owned_by": "local"
    }
  ]
}
```

#### GET /v1/models/list

List all available models with details.

**Response:**
```json
{
  "models": [
    {
      "id": "llama-3.2-1b-Q4_K_M",
      "name": "Llama 3.2 1B",
      "loaded": true,
      "size": 1073741824,
      "quantization": "Q4_K_M",
      "format": "gguf"
    }
  ]
}
```

#### POST /v1/models/load

Load a model into memory.

**Request:**
```json
{
  "model_id": "llama-3.2-1b-Q4_K_M"
}
```

**Response:**
```json
{
  "status": "loaded",
  "model_id": "llama-3.2-1b-Q4_K_M"
}
```

#### POST /v1/models/unload

Unload a model from memory.

**Request:**
```json
{
  "model_id": "llama-3.2-1b-Q4_K_M"
}
```

**Response:**
```json
{
  "status": "unloaded",
  "model_id": "llama-3.2-1b-Q4_K_M"
}
```

#### POST /v1/models/download

Download a model from Hugging Face.

**Request:**
```json
{
  "repo_id": "TheBloke/Llama-2-7B-GGUF",
  "filename": "llama-2-7b.Q4_K_M.gguf"
}
```

**Response:**
```json
{
  "status": "downloading",
  "model_id": "llama-2-7b.Q4_K_M"
}
```

#### GET /v1/models/:id/stats

Get model statistics.

**Response:**
```json
{
  "model_id": "llama-3.2-1b-Q4_K_M",
  "requests": 150,
  "tokens_generated": 45000,
  "average_latency_ms": 25.5
}
```

---

### Chat Completions

#### POST /v1/chat/completions

Create a chat completion (OpenAI compatible).

**Request:**
```json
{
  "model": "llama-3.2-1b-Q4_K_M",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "temperature": 0.7,
  "max_tokens": 256,
  "stream": false
}
```

**Response:**
```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1699999999,
  "model": "llama-3.2-1b-Q4_K_M",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 10,
    "total_tokens": 35
  }
}
```

**Streaming Response (stream: true):**
```
data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1699999999,"model":"llama-3.2-1b-Q4_K_M","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1699999999,"model":"llama-3.2-1b-Q4_K_M","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: [DONE]
```

---

### Completions

#### POST /v1/completions

Create a text completion (OpenAI compatible).

**Request:**
```json
{
  "model": "llama-3.2-1b-Q4_K_M",
  "prompt": "Once upon a time",
  "max_tokens": 100,
  "temperature": 0.8
}
```

**Response:**
```json
{
  "id": "cmpl-abc123",
  "object": "text_completion",
  "created": 1699999999,
  "model": "llama-3.2-1b-Q4_K_M",
  "choices": [
    {
      "text": " in a land far away, there lived a wise old wizard...",
      "index": 0,
      "finish_reason": "length"
    }
  ],
  "usage": {
    "prompt_tokens": 4,
    "completion_tokens": 100,
    "total_tokens": 104
  }
}
```

---

### Embeddings

#### POST /v1/embeddings

Create embeddings for text (OpenAI compatible).

**Request:**
```json
{
  "model": "llama-3.2-1b-Q4_K_M",
  "input": "The quick brown fox jumps over the lazy dog"
}
```

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "embedding": [0.001, -0.002, 0.003, ...],
      "index": 0
    }
  ],
  "model": "llama-3.2-1b-Q4_K_M",
  "usage": {
    "prompt_tokens": 9,
    "total_tokens": 9
  }
}
```

---

### Workspaces

Workspaces provide isolated environments for documents and chat sessions.

#### POST /v1/workspaces

Create a new workspace.

**Request:**
```json
{
  "name": "My Research Project",
  "description": "Research papers and notes"
}
```

**Response:**
```json
{
  "id": "ws_abc123",
  "name": "My Research Project",
  "description": "Research papers and notes",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### GET /v1/workspaces

List all workspaces.

**Response:**
```json
{
  "workspaces": [
    {
      "id": "ws_abc123",
      "name": "My Research Project",
      "description": "Research papers and notes",
      "created_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

#### GET /v1/workspaces/:id

Get workspace details.

**Response:**
```json
{
  "id": "ws_abc123",
  "name": "My Research Project",
  "description": "Research papers and notes",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### PATCH /v1/workspaces/:id

Update a workspace.

**Request:**
```json
{
  "name": "Updated Project Name"
}
```

#### DELETE /v1/workspaces/:id

Delete a workspace and all its documents/sessions.

#### GET /v1/workspaces/:id/stats

Get workspace statistics.

**Response:**
```json
{
  "document_count": 15,
  "total_chunks": 450,
  "chat_sessions": 5,
  "total_messages": 120
}
```

---

### Documents

#### POST /v1/workspaces/:id/documents

Upload a document to a workspace.

**Request:** `multipart/form-data`
- `file`: The document file (PDF, DOCX, TXT, MD)

**Response:**
```json
{
  "id": "doc_xyz789",
  "filename": "research-paper.pdf",
  "file_type": "pdf",
  "size": 1048576,
  "page_count": 25,
  "chunk_count": 150,
  "created_at": "2024-01-15T10:35:00Z"
}
```

#### GET /v1/workspaces/:id/documents

List documents in a workspace.

**Query Parameters:**
- `page` (default: 1)
- `per_page` (default: 20)

**Response:**
```json
{
  "documents": [
    {
      "id": "doc_xyz789",
      "filename": "research-paper.pdf",
      "file_type": "pdf",
      "size": 1048576,
      "page_count": 25,
      "created_at": "2024-01-15T10:35:00Z"
    }
  ],
  "total": 15,
  "page": 1,
  "per_page": 20
}
```

#### GET /v1/workspaces/:workspace_id/documents/:document_id

Get document details.

**Response:**
```json
{
  "id": "doc_xyz789",
  "filename": "research-paper.pdf",
  "file_type": "pdf",
  "size": 1048576,
  "page_count": 25,
  "chunk_count": 150,
  "metadata": {
    "title": "Research Paper Title",
    "author": "Author Name"
  },
  "created_at": "2024-01-15T10:35:00Z"
}
```

#### DELETE /v1/workspaces/:workspace_id/documents/:document_id

Delete a document and its embeddings.

---

### RAG Chat

#### POST /v1/workspaces/:id/chat

Chat with RAG (Retrieval Augmented Generation).

**Request:**
```json
{
  "model": "llama-3.2-1b-Q4_K_M",
  "messages": [
    {"role": "user", "content": "What does the paper say about machine learning?"}
  ],
  "max_context_chunks": 5,
  "temperature": 0.7
}
```

**Response:**
```json
{
  "id": "chatcmpl-rag123",
  "object": "chat.completion",
  "model": "llama-3.2-1b-Q4_K_M",
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "According to the research paper, machine learning is described as..."
      },
      "finish_reason": "stop"
    }
  ],
  "citations": [
    {
      "document_id": "doc_xyz789",
      "filename": "research-paper.pdf",
      "page": 5,
      "text": "Machine learning is a subset of artificial intelligence...",
      "relevance_score": 0.92
    },
    {
      "document_id": "doc_xyz789",
      "filename": "research-paper.pdf",
      "page": 12,
      "text": "The applications of machine learning include...",
      "relevance_score": 0.87
    }
  ],
  "usage": {
    "prompt_tokens": 450,
    "completion_tokens": 120,
    "total_tokens": 570
  }
}
```

---

### Chat Sessions

#### POST /v1/workspaces/:id/sessions

Create a new chat session.

**Request:**
```json
{
  "title": "Research Discussion",
  "model": "llama-3.2-1b-Q4_K_M"
}
```

**Response:**
```json
{
  "id": "sess_abc123",
  "workspace_id": "ws_abc123",
  "title": "Research Discussion",
  "model": "llama-3.2-1b-Q4_K_M",
  "created_at": "2024-01-15T11:00:00Z"
}
```

#### GET /v1/workspaces/:id/sessions

List chat sessions in a workspace.

**Response:**
```json
{
  "sessions": [
    {
      "id": "sess_abc123",
      "title": "Research Discussion",
      "model": "llama-3.2-1b-Q4_K_M",
      "message_count": 10,
      "created_at": "2024-01-15T11:00:00Z",
      "updated_at": "2024-01-15T12:30:00Z"
    }
  ]
}
```

#### GET /v1/workspaces/:workspace_id/sessions/:session_id

Get chat session details.

#### GET /v1/workspaces/:workspace_id/sessions/:session_id/messages

Get chat session with all messages.

**Response:**
```json
{
  "session": {
    "id": "sess_abc123",
    "title": "Research Discussion"
  },
  "messages": [
    {
      "id": "msg_001",
      "role": "user",
      "content": "What is machine learning?",
      "created_at": "2024-01-15T11:00:00Z"
    },
    {
      "id": "msg_002",
      "role": "assistant",
      "content": "Machine learning is...",
      "created_at": "2024-01-15T11:00:05Z"
    }
  ]
}
```

#### POST /v1/workspaces/:workspace_id/sessions/:session_id/messages

Add a message to a chat session.

**Request:**
```json
{
  "role": "user",
  "content": "Tell me more about neural networks"
}
```

#### DELETE /v1/workspaces/:workspace_id/sessions/:session_id

Delete a chat session and all its messages.

---

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "message": "Human-readable error message",
    "type": "error_type",
    "code": 400
  }
}
```

### Error Types

| Type | Status | Description |
|------|--------|-------------|
| `bad_request` | 400 | Invalid request parameters |
| `unauthorized` | 401 | Missing or invalid API key |
| `not_found` | 404 | Resource not found |
| `rate_limit_exceeded` | 429 | Too many requests |
| `internal_error` | 500 | Server error |
| `model_not_loaded` | 503 | Model not loaded |

---

## OpenAI SDK Compatibility

LMStudio Clone is compatible with OpenAI client libraries:

### Python

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:1234/v1",
    api_key="your-api-key"  # or "not-needed" if auth disabled
)

response = client.chat.completions.create(
    model="llama-3.2-1b-Q4_K_M",
    messages=[
        {"role": "user", "content": "Hello!"}
    ]
)
print(response.choices[0].message.content)
```

### JavaScript/TypeScript

```typescript
import OpenAI from 'openai';

const openai = new OpenAI({
    baseURL: 'http://localhost:1234/v1',
    apiKey: 'your-api-key'
});

const response = await openai.chat.completions.create({
    model: 'llama-3.2-1b-Q4_K_M',
    messages: [
        { role: 'user', content: 'Hello!' }
    ]
});
console.log(response.choices[0].message.content);
```

### cURL

```bash
curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "llama-3.2-1b-Q4_K_M",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```
