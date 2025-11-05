import React, { useState } from 'react';
import { Model } from '../../shared/types';
import { formatBytes } from '../../shared/utils';

interface ModelsProps {
  models: Model[];
  onLoadModel: (modelId: string) => Promise<void>;
  onUnloadModel: (modelId: string) => Promise<void>;
  onRefresh: () => Promise<void>;
}

const Models: React.FC<ModelsProps> = ({ models, onLoadModel, onUnloadModel, onRefresh }) => {
  const [loading, setLoading] = useState<string | null>(null);
  const [showDownload, setShowDownload] = useState(false);
  const [downloadUrl, setDownloadUrl] = useState('');

  const handleLoad = async (modelId: string) => {
    setLoading(modelId);
    try {
      await onLoadModel(modelId);
    } finally {
      setLoading(null);
    }
  };

  const handleUnload = async (modelId: string) => {
    setLoading(modelId);
    try {
      await onUnloadModel(modelId);
    } finally {
      setLoading(null);
    }
  };

  const handleDownload = async () => {
    if (!downloadUrl.trim()) return;

    try {
      const modelId = `model-${Date.now()}`;
      await window.electronAPI.downloadModel({
        url: downloadUrl,
        modelId,
      });
      setDownloadUrl('');
      setShowDownload(false);
      await onRefresh();
    } catch (error) {
      console.error('Error downloading model:', error);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border p-4">
        <div className="flex justify-between items-center">
          <div>
            <h2 className="text-lg font-semibold">Models</h2>
            <p className="text-sm text-muted-foreground mt-1">
              {models.length} model{models.length !== 1 ? 's' : ''} available
            </p>
          </div>
          <div className="flex space-x-2">
            <button
              onClick={onRefresh}
              className="px-4 py-2 bg-secondary text-secondary-foreground rounded-lg hover:opacity-90"
            >
              Refresh
            </button>
            <button
              onClick={() => setShowDownload(!showDownload)}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90"
            >
              Download Model
            </button>
          </div>
        </div>

        {/* Download form */}
        {showDownload && (
          <div className="mt-4 p-4 bg-muted rounded-lg">
            <h3 className="text-sm font-semibold mb-2">Download from Hugging Face</h3>
            <div className="flex space-x-2">
              <input
                type="text"
                value={downloadUrl}
                onChange={(e) => setDownloadUrl(e.target.value)}
                placeholder="Enter model URL or repo (e.g., username/model-name)"
                className="flex-1 bg-background border border-border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-ring"
              />
              <button
                onClick={handleDownload}
                disabled={!downloadUrl.trim()}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90 disabled:opacity-50"
              >
                Download
              </button>
            </div>
            <p className="text-xs text-muted-foreground mt-2">
              Example: lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF
            </p>
          </div>
        )}
      </div>

      {/* Models list */}
      <div className="flex-1 overflow-y-auto p-4">
        {models.length === 0 ? (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            <div className="text-center">
              <p className="text-xl mb-2">No models found</p>
              <p className="text-sm">Download a model to get started</p>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4">
            {models.map((model) => (
              <div
                key={model.id}
                className="bg-card border border-border rounded-lg p-4 hover:border-primary transition-colors"
              >
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <h3 className="font-semibold">{model.name}</h3>
                    <div className="mt-2 space-y-1 text-sm text-muted-foreground">
                      <div className="flex items-center space-x-4">
                        <span>Size: {formatBytes(model.size)}</span>
                        {model.quantization && <span>Quant: {model.quantization}</span>}
                        <span>Format: {model.format.toUpperCase()}</span>
                      </div>
                      <div>Context: {model.contextLength} tokens</div>
                      <div className="flex items-center space-x-2">
                        <span>Status:</span>
                        <span
                          className={
                            model.loaded
                              ? 'text-green-500 font-semibold'
                              : 'text-muted-foreground'
                          }
                        >
                          {model.loaded ? 'Loaded' : 'Not loaded'}
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="ml-4">
                    {model.loaded ? (
                      <button
                        onClick={() => handleUnload(model.id)}
                        disabled={loading === model.id}
                        className="px-4 py-2 bg-destructive text-destructive-foreground rounded-lg hover:opacity-90 disabled:opacity-50"
                      >
                        {loading === model.id ? 'Unloading...' : 'Unload'}
                      </button>
                    ) : (
                      <button
                        onClick={() => handleLoad(model.id)}
                        disabled={loading === model.id}
                        className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90 disabled:opacity-50"
                      >
                        {loading === model.id ? 'Loading...' : 'Load'}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default Models;
