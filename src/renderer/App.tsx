import React, { useEffect, useState } from 'react';
import { Routes, Route } from 'react-router-dom';
import Sidebar from './components/Sidebar';
import Chat from './pages/Chat';
import Models from './pages/Models';
import Settings from './pages/Settings';
import { Model, ServerStatus } from '../shared/types';

function App() {
  const [models, setModels] = useState<Model[]>([]);
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadInitialData();
  }, []);

  const loadInitialData = async () => {
    try {
      // Load models
      const modelsResult = await window.electronAPI.listModels();
      if (modelsResult.success) {
        setModels(modelsResult.models);
      }

      // Get server status
      const statusResult = await window.electronAPI.getServerStatus();
      if (statusResult.success) {
        setServerStatus(statusResult.status);
      }
    } catch (error) {
      console.error('Error loading initial data:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleModelLoad = async (modelId: string) => {
    try {
      const result = await window.electronAPI.loadModel({ modelId });
      if (result.success) {
        // Reload models to update UI
        await loadInitialData();
      }
    } catch (error) {
      console.error('Error loading model:', error);
    }
  };

  const handleModelUnload = async (modelId: string) => {
    try {
      const result = await window.electronAPI.unloadModel({ modelId });
      if (result.success) {
        await loadInitialData();
      }
    } catch (error) {
      console.error('Error unloading model:', error);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background">
        <div className="text-foreground text-xl">Loading...</div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-background text-foreground">
      <Sidebar models={models} serverStatus={serverStatus} />
      <main className="flex-1 overflow-hidden">
        <Routes>
          <Route path="/" element={<Chat models={models} />} />
          <Route
            path="/models"
            element={
              <Models
                models={models}
                onLoadModel={handleModelLoad}
                onUnloadModel={handleModelUnload}
                onRefresh={loadInitialData}
              />
            }
          />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </main>
    </div>
  );
}

export default App;
