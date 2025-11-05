import express, { Express, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import { Server } from 'http';
import { ServerConfig, ServerStatus } from '../shared/types';
import { logger } from '../shared/utils';
import InferenceEngine from '../core/inference-engine';
import ModelManager from '../core/model-manager';
import { setupRoutes } from './routes';

export class APIServer {
  private app: Express;
  private server: Server | null = null;
  private config: ServerConfig;
  private inferenceEngine: InferenceEngine;
  private modelManager: ModelManager;
  private startTime: number = 0;
  private requestCount: number = 0;

  constructor(
    config: ServerConfig,
    inferenceEngine: InferenceEngine,
    modelManager: ModelManager
  ) {
    this.config = config;
    this.inferenceEngine = inferenceEngine;
    this.modelManager = modelManager;
    this.app = express();
    this.setupMiddleware();
    this.setupRoutes();
    this.setupErrorHandling();
  }

  private setupMiddleware(): void {
    // CORS
    if (this.config.cors) {
      this.app.use(cors());
    }

    // JSON parsing
    this.app.use(express.json({ limit: '50mb' }));

    // Request logging
    this.app.use((req: Request, res: Response, next: NextFunction) => {
      this.requestCount++;
      logger.debug(`${req.method} ${req.path}`);
      next();
    });

    // API key authentication (if configured)
    if (this.config.apiKey) {
      this.app.use((req: Request, res: Response, next: NextFunction) => {
        // Skip auth for health check
        if (req.path === '/health') {
          return next();
        }

        const authHeader = req.headers.authorization;
        if (!authHeader || authHeader !== `Bearer ${this.config.apiKey}`) {
          return res.status(401).json({ error: 'Unauthorized' });
        }
        next();
      });
    }
  }

  private setupRoutes(): void {
    setupRoutes(this.app, this.inferenceEngine, this.modelManager);

    // Health check
    this.app.get('/health', (req: Request, res: Response) => {
      res.json({
        status: 'ok',
        uptime: Date.now() - this.startTime,
        requests: this.requestCount,
        loadedModels: this.modelManager.getLoadedModels().length,
      });
    });

    // Server status
    this.app.get('/status', (req: Request, res: Response) => {
      const status: ServerStatus = {
        running: true,
        port: this.config.port,
        loadedModels: this.modelManager.getLoadedModels().map((m) => m.id),
        uptime: Date.now() - this.startTime,
        requests: this.requestCount,
      };
      res.json(status);
    });
  }

  private setupErrorHandling(): void {
    // 404 handler
    this.app.use((req: Request, res: Response) => {
      res.status(404).json({
        error: {
          message: `Route ${req.method} ${req.path} not found`,
          type: 'not_found',
          code: 404,
        },
      });
    });

    // Global error handler
    this.app.use((err: Error, req: Request, res: Response, next: NextFunction) => {
      logger.error('Server error:', err);

      res.status(500).json({
        error: {
          message: err.message || 'Internal server error',
          type: 'server_error',
          code: 500,
        },
      });
    });
  }

  async start(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.server = this.app.listen(this.config.port, this.config.host, () => {
          this.startTime = Date.now();
          logger.info(`API server listening on ${this.config.host}:${this.config.port}`);
          resolve();
        });

        this.server.on('error', (error) => {
          logger.error('Server error:', error);
          reject(error);
        });
      } catch (error) {
        logger.error('Failed to start server:', error);
        reject(error);
      }
    });
  }

  async stop(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.server) {
        resolve();
        return;
      }

      this.server.close((err) => {
        if (err) {
          logger.error('Error stopping server:', err);
          reject(err);
        } else {
          logger.info('API server stopped');
          this.server = null;
          resolve();
        }
      });
    });
  }

  isRunning(): boolean {
    return this.server !== null;
  }

  getStatus(): ServerStatus {
    return {
      running: this.isRunning(),
      port: this.config.port,
      loadedModels: this.modelManager.getLoadedModels().map((m) => m.id),
      uptime: this.startTime ? Date.now() - this.startTime : 0,
      requests: this.requestCount,
    };
  }
}

export default APIServer;
