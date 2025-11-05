import { contextBridge, ipcRenderer, IpcRendererEvent } from 'electron';
import { IPCEvents } from '../shared/types';

// Expose protected methods that allow the renderer process to use
// the ipcRenderer without exposing the entire object
contextBridge.exposeInMainWorld('electronAPI', {
  // Model management
  loadModel: (data: IPCEvents['model:load']) => ipcRenderer.invoke('model:load', data),
  unloadModel: (data: IPCEvents['model:unload']) => ipcRenderer.invoke('model:unload', data),
  listModels: () => ipcRenderer.invoke('model:list'),
  downloadModel: (data: IPCEvents['model:download']) => ipcRenderer.invoke('model:download', data),

  // Model events
  onModelProgress: (callback: (progress: IPCEvents['model:progress']) => void) => {
    ipcRenderer.on('model:progress', (event: IpcRendererEvent, progress) => callback(progress));
  },

  // Chat management
  sendMessage: (data: IPCEvents['chat:send']) => ipcRenderer.invoke('chat:send', data),

  // Chat events
  onChatToken: (callback: (data: IPCEvents['chat:token']) => void) => {
    ipcRenderer.on('chat:token', (event: IpcRendererEvent, data) => callback(data));
  },
  onChatComplete: (callback: (data: IPCEvents['chat:complete']) => void) => {
    ipcRenderer.on('chat:complete', (event: IpcRendererEvent, data) => callback(data));
  },

  // Config management
  getConfig: () => ipcRenderer.invoke('config:get'),
  updateConfig: (data: IPCEvents['config:update']) => ipcRenderer.invoke('config:update', data),

  // Server status
  getServerStatus: () => ipcRenderer.invoke('server:status'),

  // Remove listeners
  removeListener: (channel: string, callback: any) => {
    ipcRenderer.removeListener(channel, callback);
  },
});

// Type declaration for TypeScript
declare global {
  interface Window {
    electronAPI: {
      loadModel: (data: IPCEvents['model:load']) => Promise<any>;
      unloadModel: (data: IPCEvents['model:unload']) => Promise<any>;
      listModels: () => Promise<any>;
      downloadModel: (data: IPCEvents['model:download']) => Promise<any>;
      onModelProgress: (callback: (progress: IPCEvents['model:progress']) => void) => void;
      sendMessage: (data: IPCEvents['chat:send']) => Promise<any>;
      onChatToken: (callback: (data: IPCEvents['chat:token']) => void) => void;
      onChatComplete: (callback: (data: IPCEvents['chat:complete']) => void) => void;
      getConfig: () => Promise<any>;
      updateConfig: (data: IPCEvents['config:update']) => Promise<any>;
      getServerStatus: () => Promise<any>;
      removeListener: (channel: string, callback: any) => void;
    };
  }
}

export {};
