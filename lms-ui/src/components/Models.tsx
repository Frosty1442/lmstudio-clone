import { useState } from "react";

interface ModelsProps {
  models: any[];
  onLoadModel: (id: string) => Promise<void>;
  onUnloadModel: (id: string) => Promise<void>;
  onRefresh: () => Promise<void>;
}

export default function Models({
  models,
  onLoadModel,
  onUnloadModel,
  onRefresh,
}: ModelsProps) {
  const [loading, setLoading] = useState<string | null>(null);

  const handleLoad = async (id: string) => {
    setLoading(id);
    try {
      await onLoadModel(id);
    } finally {
      setLoading(null);
    }
  };

  const handleUnload = async (id: string) => {
    setLoading(id);
    try {
      await onUnloadModel(id);
    } finally {
      setLoading(null);
    }
  };

  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(2)} GB`;
  };

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-gray-800 p-4">
        <div className="flex justify-between items-center">
          <div>
            <h2 className="text-lg font-semibold">Models</h2>
            <p className="text-sm text-gray-400 mt-1">
              {models.length} model{models.length !== 1 ? "s" : ""} available
            </p>
          </div>
          <button
            onClick={onRefresh}
            className="px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-lg transition-colors"
          >
            Refresh
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {models.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-400">
            <div className="text-center">
              <p className="text-xl mb-2">No models found</p>
              <p className="text-sm">
                Place GGUF models in ~/.lmstudio-clone/models/
              </p>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4">
            {models.map((model) => (
              <div
                key={model.id}
                className="bg-gray-800 border border-gray-700 rounded-lg p-4 hover:border-blue-600 transition-colors"
              >
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <h3 className="font-semibold text-lg">{model.name}</h3>
                    <div className="mt-2 space-y-1 text-sm text-gray-400">
                      <div className="flex items-center space-x-4">
                        <span>Size: {formatBytes(model.size)}</span>
                        {model.quantization && (
                          <span>Quant: {model.quantization}</span>
                        )}
                        <span>Format: {model.format || "GGUF"}</span>
                      </div>
                      <div className="flex items-center space-x-2">
                        <span>Status:</span>
                        <span
                          className={
                            model.loaded
                              ? "text-green-500 font-semibold"
                              : "text-gray-400"
                          }
                        >
                          {model.loaded ? "Loaded" : "Not loaded"}
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="ml-4">
                    {model.loaded ? (
                      <button
                        onClick={() => handleUnload(model.id)}
                        disabled={loading === model.id}
                        className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg disabled:opacity-50 transition-colors"
                      >
                        {loading === model.id ? "Unloading..." : "Unload"}
                      </button>
                    ) : (
                      <button
                        onClick={() => handleLoad(model.id)}
                        disabled={loading === model.id}
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50 transition-colors"
                      >
                        {loading === model.id ? "Loading..." : "Load"}
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
}
