import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import Chat from "./components/Chat";
import Models from "./components/Models";
import Settings from "./components/Settings";
import Sidebar from "./components/Sidebar";

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
    } finally {
      setLoading(false);
    }
  };

  const handleLoadModel = async (modelId: string) => {
    try {
      await invoke("load_model", { modelId });
      await loadData();
    } catch (error) {
      console.error("Error loading model:", error);
      alert(`Error: ${error}`);
    }
  };

  const handleUnloadModel = async (modelId: string) => {
    try {
      await invoke("unload_model", { modelId });
      await loadData();
    } catch (error) {
      console.error("Error unloading model:", error);
      alert(`Error: ${error}`);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-xl">Loading...</div>
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
      <main className="flex-1 overflow-hidden">
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
    </div>
  );
}

export default App;
