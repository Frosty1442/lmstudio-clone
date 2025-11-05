import { LlamaModel, LlamaContext, LlamaChatSession, LlamaLogLevel } from 'node-llama-cpp';
import { ModelConfig, ModelStats } from '../shared/types';
import { logger } from '../shared/utils';
import path from 'path';
import EventEmitter from 'events';

export interface InferenceOptions {
  temperature?: number;
  topP?: number;
  topK?: number;
  maxTokens?: number;
  stop?: string[];
  stream?: boolean;
}

export class InferenceEngine extends EventEmitter {
  private models: Map<string, LlamaModel> = new Map();
  private contexts: Map<string, LlamaContext> = new Map();
  private sessions: Map<string, LlamaChatSession> = new Map();
  private modelStats: Map<string, ModelStats> = new Map();

  constructor() {
    super();
  }

  async loadModel(
    modelId: string,
    modelPath: string,
    config: ModelConfig
  ): Promise<void> {
    try {
      logger.info(`Loading model: ${modelId} from ${modelPath}`);

      // Check if model is already loaded
      if (this.models.has(modelId)) {
        logger.warn(`Model ${modelId} is already loaded`);
        return;
      }

      // Load the model
      const model = new LlamaModel({
        modelPath,
        gpuLayers: config.gpuLayers,
        logLevel: LlamaLogLevel.error,
      });

      // Create context
      const context = new LlamaContext({
        model,
        contextSize: config.contextLength,
        threads: config.threads,
        batchSize: config.batchSize,
      });

      // Create chat session
      const session = new LlamaChatSession({
        context,
      });

      this.models.set(modelId, model);
      this.contexts.set(modelId, context);
      this.sessions.set(modelId, session);

      // Initialize stats
      this.modelStats.set(modelId, {
        modelId,
        loaded: true,
        memoryUsage: 0,
        tokensPerSecond: 0,
        timeToFirstToken: 0,
        contextUsage: 0,
        maxContext: config.contextLength,
      });

      logger.info(`Model ${modelId} loaded successfully`);
      this.emit('model:loaded', modelId);
    } catch (error) {
      logger.error(`Error loading model ${modelId}:`, error);
      throw new Error(`Failed to load model: ${error}`);
    }
  }

  async unloadModel(modelId: string): Promise<void> {
    try {
      logger.info(`Unloading model: ${modelId}`);

      const session = this.sessions.get(modelId);
      const context = this.contexts.get(modelId);
      const model = this.models.get(modelId);

      if (!model) {
        logger.warn(`Model ${modelId} is not loaded`);
        return;
      }

      // Clean up in reverse order
      if (session) {
        // Sessions don't need explicit cleanup
        this.sessions.delete(modelId);
      }

      if (context) {
        // Contexts will be garbage collected
        this.contexts.delete(modelId);
      }

      if (model) {
        // Models will be garbage collected
        this.models.delete(modelId);
      }

      this.modelStats.delete(modelId);

      logger.info(`Model ${modelId} unloaded successfully`);
      this.emit('model:unloaded', modelId);
    } catch (error) {
      logger.error(`Error unloading model ${modelId}:`, error);
      throw error;
    }
  }

  async generate(
    modelId: string,
    prompt: string,
    options: InferenceOptions = {}
  ): Promise<string | AsyncIterable<string>> {
    const session = this.sessions.get(modelId);
    if (!session) {
      throw new Error(`Model ${modelId} is not loaded`);
    }

    const startTime = Date.now();
    let firstTokenTime: number | null = null;
    let tokenCount = 0;

    try {
      if (options.stream) {
        // Return async generator for streaming
        return this.generateStream(session, prompt, options, {
          onFirstToken: () => {
            firstTokenTime = Date.now();
          },
          onToken: () => {
            tokenCount++;
          },
          modelId,
        });
      } else {
        // Non-streaming generation
        const response = await session.prompt(prompt, {
          temperature: options.temperature,
          topP: options.topP,
          topK: options.topK,
          maxTokens: options.maxTokens,
          stopSequence: options.stop,
        });

        // Update stats
        const endTime = Date.now();
        const duration = (endTime - startTime) / 1000; // in seconds
        const stats = this.modelStats.get(modelId);
        if (stats) {
          stats.tokensPerSecond = tokenCount / duration;
          stats.timeToFirstToken = firstTokenTime ? firstTokenTime - startTime : 0;
        }

        return response;
      }
    } catch (error) {
      logger.error(`Error generating with model ${modelId}:`, error);
      throw error;
    }
  }

  private async *generateStream(
    session: LlamaChatSession,
    prompt: string,
    options: InferenceOptions,
    callbacks: {
      onFirstToken: () => void;
      onToken: () => void;
      modelId: string;
    }
  ): AsyncIterable<string> {
    let isFirstToken = true;
    const startTime = Date.now();
    let tokenCount = 0;

    try {
      // Note: node-llama-cpp API may vary, this is a conceptual implementation
      for await (const token of session.promptStream(prompt, {
        temperature: options.temperature,
        topP: options.topP,
        topK: options.topK,
        maxTokens: options.maxTokens,
        stopSequence: options.stop,
      })) {
        if (isFirstToken) {
          callbacks.onFirstToken();
          isFirstToken = false;
        }

        callbacks.onToken();
        tokenCount++;
        yield token;
      }

      // Update stats after streaming completes
      const endTime = Date.now();
      const duration = (endTime - startTime) / 1000;
      const stats = this.modelStats.get(callbacks.modelId);
      if (stats) {
        stats.tokensPerSecond = tokenCount / duration;
      }
    } catch (error) {
      logger.error('Error during streaming generation:', error);
      throw error;
    }
  }

  async chat(
    modelId: string,
    messages: Array<{ role: string; content: string }>,
    options: InferenceOptions = {}
  ): Promise<string | AsyncIterable<string>> {
    const session = this.sessions.get(modelId);
    if (!session) {
      throw new Error(`Model ${modelId} is not loaded`);
    }

    // Convert chat messages to a single prompt
    // This is simplified; in production, use proper prompt templates
    const prompt = messages
      .map((msg) => {
        if (msg.role === 'system') return `System: ${msg.content}`;
        if (msg.role === 'user') return `User: ${msg.content}`;
        if (msg.role === 'assistant') return `Assistant: ${msg.content}`;
        return msg.content;
      })
      .join('\n\n');

    return this.generate(modelId, prompt + '\n\nAssistant:', options);
  }

  getModelStats(modelId: string): ModelStats | undefined {
    return this.modelStats.get(modelId);
  }

  getAllStats(): ModelStats[] {
    return Array.from(this.modelStats.values());
  }

  isModelLoaded(modelId: string): boolean {
    return this.models.has(modelId);
  }

  getLoadedModels(): string[] {
    return Array.from(this.models.keys());
  }

  async dispose(): Promise<void> {
    logger.info('Disposing inference engine');

    // Unload all models
    const modelIds = Array.from(this.models.keys());
    for (const modelId of modelIds) {
      await this.unloadModel(modelId);
    }

    this.removeAllListeners();
  }
}

export default InferenceEngine;
