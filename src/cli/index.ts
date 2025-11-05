#!/usr/bin/env node

import { program } from 'commander';
import path from 'path';
import fs from 'fs/promises';
import { logger } from '../shared/utils';
import InferenceEngine from '../core/inference-engine';
import ModelManager from '../core/model-manager';
import ModelDownloader from '../core/model-downloader';
import APIServer from '../server';
import { ServerConfig } from '../shared/types';

const DEFAULT_MODELS_PATH = path.join(process.env.HOME || process.env.USERPROFILE || '', '.lmstudio-clone', 'models');

// Initialize core components
let inferenceEngine: InferenceEngine;
let modelManager: ModelManager;
let modelDownloader: ModelDownloader;
let apiServer: APIServer | null = null;

async function initializeComponents() {
  if (!inferenceEngine) {
    inferenceEngine = new InferenceEngine();
    modelManager = new ModelManager(DEFAULT_MODELS_PATH, inferenceEngine);
    modelDownloader = new ModelDownloader();
    await modelManager.initialize();
  }
}

program
  .name('lmstudio-clone')
  .description('CLI tool for LMStudio Clone')
  .version('0.1.0');

// Status command
program
  .command('status')
  .description('Show server and model status')
  .action(async () => {
    try {
      await initializeComponents();

      console.log('\nLMStudio Clone Status\n');
      console.log('Server:', apiServer?.isRunning() ? 'Running' : 'Stopped');

      if (apiServer?.isRunning()) {
        const status = apiServer.getStatus();
        console.log(`Port: ${status.port}`);
        console.log(`Uptime: ${Math.floor(status.uptime / 1000)}s`);
        console.log(`Requests: ${status.requests}`);
      }

      const loadedModels = modelManager.getLoadedModels();
      console.log(`\nLoaded Models: ${loadedModels.length}`);
      loadedModels.forEach((model) => {
        console.log(`  - ${model.name} (${model.id})`);
      });

      const allModels = modelManager.getAllModels();
      console.log(`\nTotal Models: ${allModels.length}\n`);
    } catch (error) {
      console.error('Error:', error.message);
      process.exit(1);
    }
  });

// List models command
program
  .command('ls')
  .description('List all downloaded models')
  .action(async () => {
    try {
      await initializeComponents();

      const models = modelManager.getAllModels();

      if (models.length === 0) {
        console.log('No models found.');
        return;
      }

      console.log('\nAvailable Models:\n');
      models.forEach((model) => {
        const status = model.loaded ? '[LOADED]' : '[NOT LOADED]';
        const size = (model.size / (1024 * 1024 * 1024)).toFixed(2);
        console.log(`${status} ${model.name}`);
        console.log(`  ID: ${model.id}`);
        console.log(`  Size: ${size} GB`);
        console.log(`  Quantization: ${model.quantization || 'N/A'}`);
        console.log('');
      });
    } catch (error) {
      console.error('Error:', error.message);
      process.exit(1);
    }
  });

// List loaded models command
program
  .command('ps')
  .description('List loaded models')
  .action(async () => {
    try {
      await initializeComponents();

      const models = modelManager.getLoadedModels();

      if (models.length === 0) {
        console.log('No models loaded.');
        return;
      }

      console.log('\nLoaded Models:\n');
      models.forEach((model) => {
        const stats = inferenceEngine.getModelStats(model.id);
        console.log(`${model.name} (${model.id})`);
        if (stats) {
          console.log(`  Tokens/sec: ${stats.tokensPerSecond.toFixed(2)}`);
          console.log(`  Context: ${stats.contextUsage}/${stats.maxContext}`);
        }
        console.log('');
      });
    } catch (error) {
      console.error('Error:', error.message);
      process.exit(1);
    }
  });

// Load model command
program
  .command('load <model-id>')
  .description('Load a model')
  .option('-g, --gpu-layers <layers>', 'Number of GPU layers', '32')
  .action(async (modelId, options) => {
    try {
      await initializeComponents();

      console.log(`Loading model: ${modelId}...`);

      await modelManager.loadModel(modelId, {
        gpuLayers: parseInt(options.gpuLayers),
      });

      console.log(`Model ${modelId} loaded successfully.`);
    } catch (error) {
      console.error('Error:', error.message);
      process.exit(1);
    }
  });

// Unload model command
program
  .command('unload <model-id>')
  .description('Unload a model')
  .action(async (modelId) => {
    try {
      await initializeComponents();

      console.log(`Unloading model: ${modelId}...`);
      await modelManager.unloadModel(modelId);
      console.log(`Model ${modelId} unloaded successfully.`);
    } catch (error) {
      console.error('Error:', error.message);
      process.exit(1);
    }
  });

// Download model command
program
  .command('get <url>')
  .description('Download a model from Hugging Face')
  .option('-i, --id <id>', 'Model ID (default: auto-generated)')
  .option('-q, --quantization <quant>', 'Quantization (e.g., q4_k_m)')
  .action(async (url, options) => {
    try {
      await initializeComponents();

      const modelId = options.id || `model-${Date.now()}`;
      const repo = url.includes('huggingface.co')
        ? url.split('huggingface.co/')[1].split('/').slice(0, 2).join('/')
        : url;

      console.log(`Downloading model from ${repo}...`);

      // Set up progress listener
      modelDownloader.on('progress', (progress) => {
        if (progress.status === 'downloading') {
          const percent = ((progress.downloaded / progress.total) * 100).toFixed(1);
          const downloaded = (progress.downloaded / (1024 * 1024)).toFixed(1);
          const total = (progress.total / (1024 * 1024)).toFixed(1);
          process.stdout.write(`\rProgress: ${percent}% (${downloaded}/${total} MB)`);
        }
      });

      const targetPath = path.join(DEFAULT_MODELS_PATH, modelId, 'model.gguf');

      await modelDownloader.downloadModel({
        modelId,
        repo,
        quantization: options.quantization,
        targetPath,
      });

      console.log(`\n\nModel downloaded successfully: ${modelId}`);

      // Add to model manager
      await modelManager.addModel(targetPath, modelId);
      console.log('Model added to library.');
    } catch (error) {
      console.error('\nError:', error.message);
      process.exit(1);
    }
  });

// Start server command
program
  .command('server')
  .description('Start the API server')
  .option('-p, --port <port>', 'Server port', '1234')
  .option('-h, --host <host>', 'Server host', '127.0.0.1')
  .action(async (options) => {
    try {
      await initializeComponents();

      const serverConfig: ServerConfig = {
        port: parseInt(options.port),
        host: options.host,
        cors: true,
        maxConnections: 100,
      };

      apiServer = new APIServer(serverConfig, inferenceEngine, modelManager);
      await apiServer.start();

      console.log(`\nAPI Server running at http://${options.host}:${options.port}`);
      console.log('Press Ctrl+C to stop\n');

      // Keep process alive
      process.on('SIGINT', async () => {
        console.log('\nShutting down...');
        if (apiServer) {
          await apiServer.stop();
        }
        await inferenceEngine.dispose();
        await modelManager.dispose();
        process.exit(0);
      });
    } catch (error) {
      console.error('Error:', error.message);
      process.exit(1);
    }
  });

program.parse();
