// Model Types
export interface Model {
  id: string;
  name: string;
  path: string;
  format: 'gguf' | 'mlx';
  size: number;
  quantization?: string;
  contextLength: number;
  architecture: string;
  loaded: boolean;
  metadata?: ModelMetadata;
}

export interface ModelMetadata {
  author?: string;
  description?: string;
  license?: string;
  tags?: string[];
  promptTemplate?: string;
  stopTokens?: string[];
  systemPrompt?: string;
}

export interface ModelConfig {
  temperature: number;
  topP: number;
  topK: number;
  repeatPenalty: number;
  maxTokens: number;
  contextLength: number;
  gpuLayers: number;
  threads: number;
  batchSize: number;
  seed?: number;
}

// Chat Types
export interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  images?: string[];
  createdAt: number;
  tokens?: number;
  parentId?: string;
}

export interface Conversation {
  id: string;
  title: string;
  messages: Message[];
  modelId: string;
  systemPrompt?: string;
  folderId?: string;
  createdAt: number;
  updatedAt: number;
}

export interface Folder {
  id: string;
  name: string;
  parentId?: string;
  createdAt: number;
}

// API Types (OpenAI Compatible)
export interface ChatCompletionRequest {
  model: string;
  messages: Array<{
    role: 'user' | 'assistant' | 'system';
    content: string | Array<{ type: 'text' | 'image_url'; text?: string; image_url?: { url: string } }>;
  }>;
  stream?: boolean;
  temperature?: number;
  top_p?: number;
  max_tokens?: number;
  stop?: string | string[];
  presence_penalty?: number;
  frequency_penalty?: number;
  response_format?: { type: 'json_object' | 'text' };
  tools?: Array<{
    type: 'function';
    function: {
      name: string;
      description?: string;
      parameters: Record<string, unknown>;
    };
  }>;
}

export interface ChatCompletionResponse {
  id: string;
  object: 'chat.completion' | 'chat.completion.chunk';
  created: number;
  model: string;
  choices: Array<{
    index: number;
    message?: {
      role: 'assistant';
      content: string;
      tool_calls?: Array<{
        id: string;
        type: 'function';
        function: {
          name: string;
          arguments: string;
        };
      }>;
    };
    delta?: {
      role?: 'assistant';
      content?: string;
    };
    finish_reason: 'stop' | 'length' | 'tool_calls' | null;
  }>;
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

export interface CompletionRequest {
  model: string;
  prompt: string;
  stream?: boolean;
  temperature?: number;
  top_p?: number;
  max_tokens?: number;
  stop?: string | string[];
}

export interface EmbeddingRequest {
  model: string;
  input: string | string[];
}

export interface EmbeddingResponse {
  object: 'list';
  data: Array<{
    object: 'embedding';
    embedding: number[];
    index: number;
  }>;
  model: string;
  usage: {
    prompt_tokens: number;
    total_tokens: number;
  };
}

// Model Management Types
export interface ModelLoadRequest {
  modelId: string;
  config?: Partial<ModelConfig>;
}

export interface ModelLoadResponse {
  success: boolean;
  modelId: string;
  error?: string;
}

export interface ModelStats {
  modelId: string;
  loaded: boolean;
  memoryUsage: number;
  tokensPerSecond: number;
  timeToFirstToken: number;
  contextUsage: number;
  maxContext: number;
}

// Download Types
export interface DownloadProgress {
  modelId: string;
  downloaded: number;
  total: number;
  speed: number;
  status: 'downloading' | 'extracting' | 'completed' | 'error';
  error?: string;
}

export interface HuggingFaceModel {
  id: string;
  author: string;
  modelId: string;
  downloads: number;
  likes: number;
  tags: string[];
  lastModified: string;
  siblings: Array<{
    filename: string;
    size: number;
  }>;
}

// RAG Types
export interface Document {
  id: string;
  filename: string;
  content: string;
  chunks: DocumentChunk[];
  createdAt: number;
}

export interface DocumentChunk {
  id: string;
  content: string;
  embedding?: number[];
  metadata: {
    page?: number;
    position: number;
  };
}

// Config Types
export interface AppConfig {
  theme: 'dark' | 'light' | 'sepia' | 'system';
  language: string;
  apiPort: number;
  networkAccess: boolean;
  modelsPath: string;
  chatsPath: string;
  defaultModel?: string;
  autoLoadModel: boolean;
  developerMode: boolean;
}

export interface Preset {
  id: string;
  name: string;
  description?: string;
  config: ModelConfig;
  createdAt: number;
}

// IPC Event Types
export type IPCEvents = {
  // Model events
  'model:load': ModelLoadRequest;
  'model:unload': { modelId: string };
  'model:loaded': ModelLoadResponse;
  'model:stats': ModelStats;
  'model:list': void;
  'model:download': { url: string; modelId: string };
  'model:progress': DownloadProgress;

  // Chat events
  'chat:send': { conversationId: string; message: Message };
  'chat:token': { conversationId: string; token: string };
  'chat:complete': { conversationId: string; message: Message };
  'chat:error': { conversationId: string; error: string };

  // Config events
  'config:get': void;
  'config:update': Partial<AppConfig>;
  'config:updated': AppConfig;

  // App events
  'app:ready': void;
  'app:quit': void;
};

// Server Types
export interface ServerConfig {
  port: number;
  host: string;
  cors: boolean;
  apiKey?: string;
  maxConnections: number;
}

export interface ServerStatus {
  running: boolean;
  port: number;
  loadedModels: string[];
  uptime: number;
  requests: number;
}
