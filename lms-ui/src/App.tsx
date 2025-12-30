import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import Chat from "./components/Chat";
import Models from "./components/Models";
import Settings from "./components/Settings";
import Sidebar from "./components/Sidebar";
import LoadingSpinner from "./components/LoadingSpinner";
import { ToastContainer, useToast } from "./components/Toast";

interface Model {
  id: string;
  name: string;
  loaded: boolean;
  size: number;
  quantization?: string;
}

interface ServerStatus {
  running: boolean;
  loaded_models: string[];
  uptime: number;
  version: string;
}

function App() {
  const [currentPage, setCurrentPage] = useState("chat");
  const [models, setModels] = useState<Model[]>([]);
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const toast = useToast();

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 5000); // Refresh every 5s
    return () => clearInterval(interval);
  }, []);

  const loadData = async () => {
    try {
      const [modelsData, statusData] = await Promise.all([
        invoke<{ models: Model[] }>("get_models"),
        invoke<ServerStatus>("get_server_status"),
      ]);

      setModels(modelsData.models);
      setServerStatus(statusData);
    } catch (error) {
      console.error("Error loading data:", error);
      toast.error("Connection Error", "Failed to connect to server");
    } finally {
      setLoading(false);
    }
  };

  const handleLoadModel = async (modelId: string) => {
    setActionLoading(true);
    try {
      await invoke("load_model", { modelId });
      await loadData();
      toast.success("Model Loaded", `${modelId} is now ready to use`);
    } catch (error) {
      console.error("Error loading model:", error);
      toast.error("Failed to Load Model", String(error));
    } finally {
      setActionLoading(false);
    }
  };

  const handleUnloadModel = async (modelId: string) => {
    setActionLoading(true);
    try {
      await invoke("unload_model", { modelId });
      await loadData();
      toast.success("Model Unloaded", `${modelId} has been unloaded`);
    } catch (error) {
      console.error("Error unloading model:", error);
      toast.error("Failed to Unload Model", String(error));
    } finally {
      setActionLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-4">
        <LoadingSpinner size="lg" />
        <div className="text-xl text-gray-400">Connecting to server...</div>
      </div>
    );
  }

  return (
    <div className="flex h-screen">
      <Sidebar
        currentPage={currentPage}
        onPageChange={setCurrentPage}
        serverStatus={serverStatus}
        loadedModelsCount={models.filter((m) => m.loaded).length}
      />
      <main className="flex-1 overflow-hidden relative">
        {actionLoading && (
          <div className="absolute inset-0 bg-black/30 flex items-center justify-center z-40">
            <div className="bg-gray-800 rounded-lg p-4 flex items-center gap-3">
              <LoadingSpinner size="md" />
              <span>Processing...</span>
            </div>
          </div>
        )}
        {currentPage === "chat" && <Chat models={models} />}
        {currentPage === "models" && (
          <Models
            models={models}
            onLoadModel={handleLoadModel}
            onUnloadModel={handleUnloadModel}
            onRefresh={loadData}
          />
        )}
        {currentPage === "settings" && <Settings />}
      </main>
      <ToastContainer toasts={toast.toasts} onDismiss={toast.dismissToast} />
    </div>
  );
}

export default App;
