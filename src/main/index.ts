import { app, BrowserWindow, ipcMain, IpcMainEvent } from 'electron';
import path from 'path';
import Store from 'electron-store';
import { AppConfig, IPCEvents, ServerConfig } from '../shared/types';
import { logger } from '../shared/utils';
import InferenceEngine from '../core/inference-engine';
import ModelManager from '../core/model-manager';
import ModelDownloader from '../core/model-downloader';
import APIServer from '../server';

// Enable better error handling
process.on('uncaughtException', (error) => {
  logger.error('Uncaught exception:', error);
});

process.on('unhandledRejection', (error) => {
  logger.error('Unhandled rejection:', error);
});

class Application {
  private mainWindow: BrowserWindow | null = null;
  private store: Store<AppConfig>;
  private inferenceEngine: InferenceEngine;
  private modelManager: ModelManager;
  private modelDownloader: ModelDownloader;
  private apiServer: APIServer | null = null;
  private headless: boolean = false;

  constructor() {
    this.headless = process.argv.includes('--headless');
    this.store = new Store<AppConfig>({
      defaults: {
        theme: 'dark',
        language: 'en',
        apiPort: 1234,
        networkAccess: false,
        modelsPath: path.join(app.getPath('userData'), 'models'),
        chatsPath: path.join(app.getPath('userData'), 'chats'),
        autoLoadModel: true,
        developerMode: false,
      },
    });

    const config = this.store.store;
    this.inferenceEngine = new InferenceEngine();
    this.modelManager = new ModelManager(config.modelsPath, this.inferenceEngine);
    this.modelDownloader = new ModelDownloader();
  }

  async initialize(): Promise<void> {
    try {
      logger.info('Initializing application');

      // Initialize model manager
      await this.modelManager.initialize();

      // Start API server
      const config = this.store.store;
      const serverConfig: ServerConfig = {
        port: config.apiPort,
        host: config.networkAccess ? '0.0.0.0' : '127.0.0.1',
        cors: true,
        maxConnections: 100,
      };

      this.apiServer = new APIServer(serverConfig, this.inferenceEngine, this.modelManager);
      await this.apiServer.start();

      logger.info('Application initialized successfully');
    } catch (error) {
      logger.error('Error initializing application:', error);
      throw error;
    }
  }

  async createWindow(): Promise<void> {
    if (this.headless) {
      logger.info('Running in headless mode');
      return;
    }

    this.mainWindow = new BrowserWindow({
      width: 1200,
      height: 800,
      minWidth: 800,
      minHeight: 600,
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        preload: path.join(__dirname, 'preload.js'),
      },
      titleBarStyle: 'hidden',
      backgroundColor: '#1a1a1a',
    });

    // Load the app
    if (process.env.NODE_ENV === 'development') {
      await this.mainWindow.loadURL('http://localhost:5173');
      this.mainWindow.webContents.openDevTools();
    } else {
      await this.mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
    }

    this.mainWindow.on('closed', () => {
      this.mainWindow = null;
    });

    logger.info('Main window created');
  }

  setupIPC(): void {
    // Model management
    ipcMain.handle('model:load', async (event: IpcMainEvent, data: IPCEvents['model:load']) => {
      try {
        await this.modelManager.loadModel(data.modelId, data.config);
        return { success: true, modelId: data.modelId };
      } catch (error) {
        logger.error('Error in model:load:', error);
        return { success: false, error: error.message };
      }
    });

    ipcMain.handle('model:unload', async (event: IpcMainEvent, data: IPCEvents['model:unload']) => {
      try {
        await this.modelManager.unloadModel(data.modelId);
        return { success: true };
      } catch (error) {
        logger.error('Error in model:unload:', error);
        return { success: false, error: error.message };
      }
    });

    ipcMain.handle('model:list', async () => {
      try {
        const models = this.modelManager.getAllModels();
        return { success: true, models };
      } catch (error) {
        logger.error('Error in model:list:', error);
        return { success: false, error: error.message };
      }
    });

    ipcMain.handle('model:download', async (event: IpcMainEvent, data: IPCEvents['model:download']) => {
      try {
        const { url, modelId } = data;

        // Parse URL to get repo and file info
        // Simplified implementation
        const repo = url.includes('huggingface.co')
          ? url.split('huggingface.co/')[1].split('/')[0] + '/' + url.split('huggingface.co/')[1].split('/')[1]
          : url;

        const targetPath = path.join(this.store.store.modelsPath, modelId, 'model.gguf');

        // Set up progress listener
        this.modelDownloader.on('progress', (progress) => {
          if (this.mainWindow) {
            this.mainWindow.webContents.send('model:progress', progress);
          }
        });

        const downloadPath = await this.modelDownloader.downloadModel({
          modelId,
          repo,
          targetPath,
        });

        // Add model to manager
        await this.modelManager.addModel(downloadPath, modelId);

        return { success: true, modelId };
      } catch (error) {
        logger.error('Error in model:download:', error);
        return { success: false, error: error.message };
      }
    });

    // Chat management
    ipcMain.handle('chat:send', async (event: IpcMainEvent, data: IPCEvents['chat:send']) => {
      try {
        const { conversationId, message } = data;

        // Get conversation and build messages
        // This is simplified - implement full conversation management
        const messages = [{ role: message.role, content: message.content }];

        // Assuming a default model for now
        const loadedModels = this.modelManager.getLoadedModels();
        if (loadedModels.length === 0) {
          throw new Error('No model loaded');
        }

        const modelId = loadedModels[0].id;

        // Generate response
        const result = await this.inferenceEngine.chat(modelId, messages, { stream: false });
        const content = typeof result === 'string' ? result : '';

        return {
          success: true,
          message: {
            id: `msg-${Date.now()}`,
            role: 'assistant',
            content,
            createdAt: Date.now(),
          },
        };
      } catch (error) {
        logger.error('Error in chat:send:', error);
        return { success: false, error: error.message };
      }
    });

    // Config management
    ipcMain.handle('config:get', async () => {
      return { success: true, config: this.store.store };
    });

    ipcMain.handle('config:update', async (event: IpcMainEvent, data: IPCEvents['config:update']) => {
      try {
        this.store.set(data as any);
        return { success: true, config: this.store.store };
      } catch (error) {
        logger.error('Error in config:update:', error);
        return { success: false, error: error.message };
      }
    });

    // Server management
    ipcMain.handle('server:status', async () => {
      return {
        success: true,
        status: this.apiServer?.getStatus() || null,
      };
    });

    logger.info('IPC handlers set up');
  }

  async shutdown(): Promise<void> {
    logger.info('Shutting down application');

    try {
      // Stop API server
      if (this.apiServer) {
        await this.apiServer.stop();
      }

      // Dispose inference engine and model manager
      await this.inferenceEngine.dispose();
      await this.modelManager.dispose();

      logger.info('Application shut down successfully');
    } catch (error) {
      logger.error('Error during shutdown:', error);
    }
  }
}

// Application lifecycle
const application = new Application();

app.whenReady().then(async () => {
  try {
    await application.initialize();
    application.setupIPC();
    await application.createWindow();

    app.on('activate', async () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        await application.createWindow();
      }
    });
  } catch (error) {
    logger.error('Error during app ready:', error);
    app.quit();
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('before-quit', async (event) => {
  event.preventDefault();
  await application.shutdown();
  app.exit(0);
});
