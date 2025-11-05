import { downloadFile, listFiles } from '@huggingface/hub';
import fs from 'fs/promises';
import path from 'path';
import { DownloadProgress, HuggingFaceModel } from '../shared/types';
import { logger, formatBytes, formatSpeed } from '../shared/utils';
import EventEmitter from 'events';

export interface DownloadOptions {
  modelId: string;
  repo: string;
  filename?: string;
  quantization?: string;
  targetPath: string;
}

export class ModelDownloader extends EventEmitter {
  private activeDownloads: Map<string, AbortController> = new Map();

  constructor() {
    super();
  }

  async searchModels(query: string, options?: {
    filter?: string;
    limit?: number;
  }): Promise<HuggingFaceModel[]> {
    try {
      // This is a simplified implementation
      // In production, use the full Hugging Face API
      logger.info(`Searching models: ${query}`);

      // For now, return empty array - implement full HF API search
      return [];
    } catch (error) {
      logger.error('Error searching models:', error);
      throw error;
    }
  }

  async getModelFiles(repo: string): Promise<Array<{ filename: string; size: number }>> {
    try {
      logger.info(`Fetching files for repo: ${repo}`);

      const files = [];
      for await (const file of listFiles({ repo })) {
        files.push({
          filename: file.path,
          size: file.size || 0,
        });
      }

      return files;
    } catch (error) {
      logger.error(`Error fetching files for repo ${repo}:`, error);
      throw error;
    }
  }

  async downloadModel(options: DownloadOptions): Promise<string> {
    const { modelId, repo, filename, targetPath } = options;

    try {
      logger.info(`Starting download for model: ${modelId} from ${repo}`);

      // Ensure target directory exists
      await fs.mkdir(path.dirname(targetPath), { recursive: true });

      // Create abort controller for this download
      const abortController = new AbortController();
      this.activeDownloads.set(modelId, abortController);

      let downloaded = 0;
      let total = 0;
      let lastUpdate = Date.now();
      let lastDownloaded = 0;

      // Determine which file to download
      let fileToDownload = filename;

      if (!fileToDownload) {
        // Auto-select file based on quantization or get the first .gguf file
        const files = await this.getModelFiles(repo);
        const ggufFiles = files.filter((f) => f.filename.endsWith('.gguf'));

        if (ggufFiles.length === 0) {
          throw new Error('No GGUF files found in repository');
        }

        if (options.quantization) {
          // Find file with matching quantization
          fileToDownload = ggufFiles.find((f) =>
            f.filename.toLowerCase().includes(options.quantization!.toLowerCase())
          )?.filename;
        }

        if (!fileToDownload) {
          // Use first file or one with q4 quantization (common default)
          fileToDownload =
            ggufFiles.find((f) => f.filename.includes('q4'))?.filename ||
            ggufFiles[0].filename;
        }
      }

      logger.info(`Downloading file: ${fileToDownload}`);

      // Emit initial progress
      this.emitProgress(modelId, downloaded, total, 0, 'downloading');

      // Download the file with progress tracking
      const downloadResult = await downloadFile({
        repo,
        path: fileToDownload,
        // Note: The actual API may differ slightly
      });

      // Read the download stream and track progress
      if (downloadResult) {
        const response = await fetch(downloadResult);

        if (!response.ok) {
          throw new Error(`Download failed: ${response.statusText}`);
        }

        total = parseInt(response.headers.get('content-length') || '0');

        const fileStream = await fs.open(targetPath, 'w');
        const writer = fileStream.createWriteStream();

        const reader = response.body?.getReader();
        if (!reader) {
          throw new Error('Failed to get response body reader');
        }

        while (true) {
          const { done, value } = await reader.read();

          if (done) break;

          downloaded += value.length;
          await writer.write(value);

          // Emit progress updates every 500ms
          const now = Date.now();
          if (now - lastUpdate > 500) {
            const speed = ((downloaded - lastDownloaded) / (now - lastUpdate)) * 1000;
            this.emitProgress(modelId, downloaded, total, speed, 'downloading');
            lastUpdate = now;
            lastDownloaded = downloaded;
          }
        }

        await fileStream.close();
      }

      // Download completed
      this.emitProgress(modelId, total, total, 0, 'completed');
      this.activeDownloads.delete(modelId);

      logger.info(`Download completed: ${modelId}`);
      return targetPath;
    } catch (error) {
      logger.error(`Error downloading model ${modelId}:`, error);
      this.emitProgress(modelId, 0, 0, 0, 'error', error.message);
      this.activeDownloads.delete(modelId);
      throw error;
    }
  }

  cancelDownload(modelId: string): void {
    const controller = this.activeDownloads.get(modelId);
    if (controller) {
      controller.abort();
      this.activeDownloads.delete(modelId);
      logger.info(`Download cancelled: ${modelId}`);
    }
  }

  private emitProgress(
    modelId: string,
    downloaded: number,
    total: number,
    speed: number,
    status: DownloadProgress['status'],
    error?: string
  ): void {
    const progress: DownloadProgress = {
      modelId,
      downloaded,
      total,
      speed,
      status,
      error,
    };

    this.emit('progress', progress);

    // Log progress
    if (status === 'downloading' && total > 0) {
      const percent = ((downloaded / total) * 100).toFixed(1);
      logger.debug(
        `Download progress [${modelId}]: ${percent}% (${formatBytes(downloaded)}/${formatBytes(total)}) @ ${formatSpeed(speed)}`
      );
    }
  }

  isDownloading(modelId: string): boolean {
    return this.activeDownloads.has(modelId);
  }

  getActiveDownloads(): string[] {
    return Array.from(this.activeDownloads.keys());
  }
}

export default ModelDownloader;
