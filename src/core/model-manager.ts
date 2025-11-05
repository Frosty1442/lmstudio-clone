import fs from 'fs/promises';
import path from 'path';
import { Model, ModelConfig, ModelMetadata } from '../shared/types';
import { logger, generateId } from '../shared/utils';
import InferenceEngine from './inference-engine';

export class ModelManager {
  private modelsPath: string;
  private models: Map<string, Model> = new Map();
  private inferenceEngine: InferenceEngine;
  private modelConfigs: Map<string, ModelConfig> = new Map();

  constructor(modelsPath: string, inferenceEngine: InferenceEngine) {
    this.modelsPath = modelsPath;
    this.inferenceEngine = inferenceEngine;
  }

  async initialize(): Promise<void> {
    try {
      // Ensure models directory exists
      await fs.mkdir(this.modelsPath, { recursive: true });

      // Scan for existing models
      await this.scanModels();

      logger.info(`Model manager initialized with ${this.models.size} models`);
    } catch (error) {
      logger.error('Error initializing model manager:', error);
      throw error;
    }
  }

  private async scanModels(): Promise<void> {
    try {
      const entries = await fs.readdir(this.modelsPath, { withFileTypes: true });

      for (const entry of entries) {
        if (entry.isDirectory()) {
          const modelPath = path.join(this.modelsPath, entry.name);
          await this.loadModelMetadata(modelPath, entry.name);
        }
      }
    } catch (error) {
      logger.error('Error scanning models:', error);
    }
  }

  private async loadModelMetadata(modelPath: string, dirName: string): Promise<void> {
    try {
      const metadataPath = path.join(modelPath, 'metadata.json');
      const files = await fs.readdir(modelPath);

      // Find the .gguf or .mlx file
      const modelFile = files.find(
        (f) => f.endsWith('.gguf') || f.endsWith('.bin') || f.endsWith('.mlx')
      );

      if (!modelFile) {
        logger.warn(`No model file found in ${modelPath}`);
        return;
      }

      const modelFilePath = path.join(modelPath, modelFile);
      const stats = await fs.stat(modelFilePath);

      // Try to load metadata
      let metadata: ModelMetadata = {};
      try {
        const metadataContent = await fs.readFile(metadataPath, 'utf-8');
        metadata = JSON.parse(metadataContent);
      } catch {
        logger.debug(`No metadata file found for ${dirName}`);
      }

      const model: Model = {
        id: dirName,
        name: metadata.description || dirName,
        path: modelFilePath,
        format: modelFile.endsWith('.gguf') ? 'gguf' : 'mlx',
        size: stats.size,
        quantization: this.extractQuantization(modelFile),
        contextLength: 4096, // Default, should be detected from model
        architecture: metadata.author || 'unknown',
        loaded: false,
        metadata,
      };

      this.models.set(model.id, model);
      logger.info(`Loaded model metadata: ${model.id}`);
    } catch (error) {
      logger.error(`Error loading model metadata from ${modelPath}:`, error);
    }
  }

  private extractQuantization(filename: string): string | undefined {
    // Extract quantization from filename (e.g., q4_k_m, q5_k_s, etc.)
    const match = filename.match(/[._-](q\d+_[a-z0-9_]+)/i);
    return match ? match[1] : undefined;
  }

  async addModel(modelPath: string, modelId?: string): Promise<Model> {
    try {
      const id = modelId || generateId();
      const targetDir = path.join(this.modelsPath, id);

      // Create directory for the model
      await fs.mkdir(targetDir, { recursive: true });

      // Copy model file
      const filename = path.basename(modelPath);
      const targetPath = path.join(targetDir, filename);
      await fs.copyFile(modelPath, targetPath);

      const stats = await fs.stat(targetPath);

      const model: Model = {
        id,
        name: filename,
        path: targetPath,
        format: filename.endsWith('.gguf') ? 'gguf' : 'mlx',
        size: stats.size,
        quantization: this.extractQuantization(filename),
        contextLength: 4096,
        architecture: 'unknown',
        loaded: false,
      };

      this.models.set(id, model);

      // Save metadata
      await this.saveModelMetadata(model);

      logger.info(`Added model: ${id}`);
      return model;
    } catch (error) {
      logger.error('Error adding model:', error);
      throw error;
    }
  }

  async removeModel(modelId: string): Promise<void> {
    try {
      const model = this.models.get(modelId);
      if (!model) {
        throw new Error(`Model ${modelId} not found`);
      }

      // Unload if loaded
      if (model.loaded) {
        await this.unloadModel(modelId);
      }

      // Remove directory
      const modelDir = path.dirname(model.path);
      await fs.rm(modelDir, { recursive: true, force: true });

      this.models.delete(modelId);
      this.modelConfigs.delete(modelId);

      logger.info(`Removed model: ${modelId}`);
    } catch (error) {
      logger.error(`Error removing model ${modelId}:`, error);
      throw error;
    }
  }

  async loadModel(modelId: string, config?: Partial<ModelConfig>): Promise<void> {
    try {
      const model = this.models.get(modelId);
      if (!model) {
        throw new Error(`Model ${modelId} not found`);
      }

      if (model.loaded) {
        logger.warn(`Model ${modelId} is already loaded`);
        return;
      }

      // Get or create config
      const fullConfig = this.getModelConfig(modelId, config);

      // Load model using inference engine
      await this.inferenceEngine.loadModel(modelId, model.path, fullConfig);

      // Update model status
      model.loaded = true;
      this.models.set(modelId, model);

      logger.info(`Model ${modelId} loaded`);
    } catch (error) {
      logger.error(`Error loading model ${modelId}:`, error);
      throw error;
    }
  }

  async unloadModel(modelId: string): Promise<void> {
    try {
      const model = this.models.get(modelId);
      if (!model) {
        throw new Error(`Model ${modelId} not found`);
      }

      if (!model.loaded) {
        logger.warn(`Model ${modelId} is not loaded`);
        return;
      }

      await this.inferenceEngine.unloadModel(modelId);

      model.loaded = false;
      this.models.set(modelId, model);

      logger.info(`Model ${modelId} unloaded`);
    } catch (error) {
      logger.error(`Error unloading model ${modelId}:`, error);
      throw error;
    }
  }

  getModel(modelId: string): Model | undefined {
    return this.models.get(modelId);
  }

  getAllModels(): Model[] {
    return Array.from(this.models.values());
  }

  getLoadedModels(): Model[] {
    return Array.from(this.models.values()).filter((m) => m.loaded);
  }

  private getModelConfig(modelId: string, override?: Partial<ModelConfig>): ModelConfig {
    const existing = this.modelConfigs.get(modelId);

    const defaultConfig: ModelConfig = {
      temperature: 0.7,
      topP: 0.9,
      topK: 40,
      repeatPenalty: 1.1,
      maxTokens: 2048,
      contextLength: 4096,
      gpuLayers: 32, // Default GPU layers
      threads: 4,
      batchSize: 512,
    };

    const config = {
      ...defaultConfig,
      ...existing,
      ...override,
    };

    this.modelConfigs.set(modelId, config);
    return config;
  }

  async updateModelConfig(modelId: string, config: Partial<ModelConfig>): Promise<void> {
    const currentConfig = this.getModelConfig(modelId);
    const newConfig = { ...currentConfig, ...config };
    this.modelConfigs.set(modelId, newConfig);

    // Save to file
    await this.saveModelConfig(modelId, newConfig);

    logger.info(`Updated config for model ${modelId}`);
  }

  private async saveModelMetadata(model: Model): Promise<void> {
    try {
      const modelDir = path.dirname(model.path);
      const metadataPath = path.join(modelDir, 'metadata.json');

      const metadata = {
        id: model.id,
        name: model.name,
        format: model.format,
        quantization: model.quantization,
        contextLength: model.contextLength,
        architecture: model.architecture,
        ...model.metadata,
      };

      await fs.writeFile(metadataPath, JSON.stringify(metadata, null, 2));
    } catch (error) {
      logger.error(`Error saving metadata for model ${model.id}:`, error);
    }
  }

  private async saveModelConfig(modelId: string, config: ModelConfig): Promise<void> {
    try {
      const model = this.models.get(modelId);
      if (!model) return;

      const modelDir = path.dirname(model.path);
      const configPath = path.join(modelDir, 'config.json');

      await fs.writeFile(configPath, JSON.stringify(config, null, 2));
    } catch (error) {
      logger.error(`Error saving config for model ${modelId}:`, error);
    }
  }

  async dispose(): Promise<void> {
    // Unload all loaded models
    const loadedModels = this.getLoadedModels();
    for (const model of loadedModels) {
      await this.unloadModel(model.id);
    }

    this.models.clear();
    this.modelConfigs.clear();

    logger.info('Model manager disposed');
  }
}

export default ModelManager;
