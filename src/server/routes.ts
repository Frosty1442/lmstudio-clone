import { Express, Request, Response } from 'express';
import {
  ChatCompletionRequest,
  ChatCompletionResponse,
  CompletionRequest,
  EmbeddingRequest,
  EmbeddingResponse,
  ModelLoadRequest,
} from '../shared/types';
import { logger, generateId } from '../shared/utils';
import InferenceEngine from '../core/inference-engine';
import ModelManager from '../core/model-manager';

export function setupRoutes(
  app: Express,
  inferenceEngine: InferenceEngine,
  modelManager: ModelManager
): void {
  // OpenAI-compatible endpoints

  // List models
  app.get('/v1/models', async (req: Request, res: Response) => {
    try {
      const models = modelManager.getAllModels();

      res.json({
        object: 'list',
        data: models.map((model) => ({
          id: model.id,
          object: 'model',
          created: Math.floor(Date.now() / 1000),
          owned_by: 'local',
          permission: [],
          root: model.id,
          parent: null,
        })),
      });
    } catch (error) {
      logger.error('Error listing models:', error);
      res.status(500).json({ error: { message: error.message } });
    }
  });

  // Chat completions
  app.post('/v1/chat/completions', async (req: Request, res: Response) => {
    try {
      const request: ChatCompletionRequest = req.body;

      // Validate request
      if (!request.model || !request.messages) {
        return res.status(400).json({
          error: { message: 'Missing required fields: model and messages' },
        });
      }

      // Check if model is loaded
      if (!inferenceEngine.isModelLoaded(request.model)) {
        // Try to auto-load the model
        try {
          await modelManager.loadModel(request.model);
        } catch (error) {
          return res.status(404).json({
            error: { message: `Model ${request.model} not found or failed to load` },
          });
        }
      }

      const messages = request.messages.map((msg) => ({
        role: msg.role,
        content: typeof msg.content === 'string' ? msg.content : JSON.stringify(msg.content),
      }));

      if (request.stream) {
        // Streaming response
        res.setHeader('Content-Type', 'text/event-stream');
        res.setHeader('Cache-Control', 'no-cache');
        res.setHeader('Connection', 'keep-alive');

        const result = await inferenceEngine.chat(request.model, messages, {
          temperature: request.temperature,
          topP: request.top_p,
          maxTokens: request.max_tokens,
          stop: Array.isArray(request.stop) ? request.stop : request.stop ? [request.stop] : undefined,
          stream: true,
        });

        if (typeof result === 'string') {
          // Non-streaming fallback
          const response: ChatCompletionResponse = {
            id: `chatcmpl-${generateId()}`,
            object: 'chat.completion.chunk',
            created: Math.floor(Date.now() / 1000),
            model: request.model,
            choices: [
              {
                index: 0,
                delta: { content: result },
                finish_reason: 'stop',
              },
            ],
          };
          res.write(`data: ${JSON.stringify(response)}\n\n`);
          res.write('data: [DONE]\n\n');
          res.end();
        } else {
          // Streaming
          const id = `chatcmpl-${generateId()}`;
          for await (const token of result) {
            const chunk: ChatCompletionResponse = {
              id,
              object: 'chat.completion.chunk',
              created: Math.floor(Date.now() / 1000),
              model: request.model,
              choices: [
                {
                  index: 0,
                  delta: { content: token },
                  finish_reason: null,
                },
              ],
            };
            res.write(`data: ${JSON.stringify(chunk)}\n\n`);
          }

          // Send final chunk
          const finalChunk: ChatCompletionResponse = {
            id,
            object: 'chat.completion.chunk',
            created: Math.floor(Date.now() / 1000),
            model: request.model,
            choices: [
              {
                index: 0,
                delta: {},
                finish_reason: 'stop',
              },
            ],
          };
          res.write(`data: ${JSON.stringify(finalChunk)}\n\n`);
          res.write('data: [DONE]\n\n');
          res.end();
        }
      } else {
        // Non-streaming response
        const result = await inferenceEngine.chat(request.model, messages, {
          temperature: request.temperature,
          topP: request.top_p,
          maxTokens: request.max_tokens,
          stop: Array.isArray(request.stop) ? request.stop : request.stop ? [request.stop] : undefined,
          stream: false,
        });

        const content = typeof result === 'string' ? result : '';

        const response: ChatCompletionResponse = {
          id: `chatcmpl-${generateId()}`,
          object: 'chat.completion',
          created: Math.floor(Date.now() / 1000),
          model: request.model,
          choices: [
            {
              index: 0,
              message: {
                role: 'assistant',
                content: content,
              },
              finish_reason: 'stop',
            },
          ],
          usage: {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
          },
        };

        res.json(response);
      }
    } catch (error) {
      logger.error('Error in chat completion:', error);
      res.status(500).json({ error: { message: error.message } });
    }
  });

  // Text completions
  app.post('/v1/completions', async (req: Request, res: Response) => {
    try {
      const request: CompletionRequest = req.body;

      if (!request.model || !request.prompt) {
        return res.status(400).json({
          error: { message: 'Missing required fields: model and prompt' },
        });
      }

      // Check if model is loaded
      if (!inferenceEngine.isModelLoaded(request.model)) {
        await modelManager.loadModel(request.model);
      }

      const result = await inferenceEngine.generate(request.model, request.prompt, {
        temperature: request.temperature,
        topP: request.top_p,
        maxTokens: request.max_tokens,
        stop: Array.isArray(request.stop) ? request.stop : request.stop ? [request.stop] : undefined,
        stream: request.stream,
      });

      if (request.stream) {
        res.setHeader('Content-Type', 'text/event-stream');
        res.setHeader('Cache-Control', 'no-cache');
        res.setHeader('Connection', 'keep-alive');

        if (typeof result !== 'string') {
          const id = `cmpl-${generateId()}`;
          for await (const token of result) {
            res.write(
              `data: ${JSON.stringify({
                id,
                object: 'text_completion',
                created: Math.floor(Date.now() / 1000),
                model: request.model,
                choices: [{ text: token, index: 0, finish_reason: null }],
              })}\n\n`
            );
          }
        }
        res.write('data: [DONE]\n\n');
        res.end();
      } else {
        const text = typeof result === 'string' ? result : '';
        res.json({
          id: `cmpl-${generateId()}`,
          object: 'text_completion',
          created: Math.floor(Date.now() / 1000),
          model: request.model,
          choices: [
            {
              text,
              index: 0,
              finish_reason: 'stop',
            },
          ],
        });
      }
    } catch (error) {
      logger.error('Error in completion:', error);
      res.status(500).json({ error: { message: error.message } });
    }
  });

  // Embeddings (placeholder - requires embedding model support)
  app.post('/v1/embeddings', async (req: Request, res: Response) => {
    try {
      const request: EmbeddingRequest = req.body;

      if (!request.model || !request.input) {
        return res.status(400).json({
          error: { message: 'Missing required fields: model and input' },
        });
      }

      // TODO: Implement actual embedding generation
      const response: EmbeddingResponse = {
        object: 'list',
        data: [],
        model: request.model,
        usage: {
          prompt_tokens: 0,
          total_tokens: 0,
        },
      };

      res.json(response);
    } catch (error) {
      logger.error('Error generating embeddings:', error);
      res.status(500).json({ error: { message: error.message } });
    }
  });

  // LM Studio specific endpoints

  // Load model
  app.post('/v1/models/load', async (req: Request, res: Response) => {
    try {
      const request: ModelLoadRequest = req.body;

      if (!request.modelId) {
        return res.status(400).json({ error: { message: 'Missing modelId' } });
      }

      await modelManager.loadModel(request.modelId, request.config);

      res.json({
        success: true,
        modelId: request.modelId,
      });
    } catch (error) {
      logger.error('Error loading model:', error);
      res.status(500).json({
        success: false,
        error: error.message,
      });
    }
  });

  // Unload model
  app.post('/v1/models/unload', async (req: Request, res: Response) => {
    try {
      const { modelId } = req.body;

      if (!modelId) {
        return res.status(400).json({ error: { message: 'Missing modelId' } });
      }

      await modelManager.unloadModel(modelId);

      res.json({
        success: true,
        modelId,
      });
    } catch (error) {
      logger.error('Error unloading model:', error);
      res.status(500).json({
        success: false,
        error: error.message,
      });
    }
  });

  // Get model stats
  app.get('/v1/models/:modelId/stats', async (req: Request, res: Response) => {
    try {
      const { modelId } = req.params;
      const stats = inferenceEngine.getModelStats(modelId);

      if (!stats) {
        return res.status(404).json({ error: { message: 'Model not found' } });
      }

      res.json(stats);
    } catch (error) {
      logger.error('Error getting model stats:', error);
      res.status(500).json({ error: { message: error.message } });
    }
  });

  // List loaded models
  app.get('/v1/models/loaded', async (req: Request, res: Response) => {
    try {
      const models = modelManager.getLoadedModels();
      res.json({
        models: models.map((m) => ({
          id: m.id,
          name: m.name,
          loaded: m.loaded,
          size: m.size,
          quantization: m.quantization,
        })),
      });
    } catch (error) {
      logger.error('Error listing loaded models:', error);
      res.status(500).json({ error: { message: error.message } });
    }
  });
}
