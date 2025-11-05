import React, { useState, useEffect } from 'react';
import { AppConfig } from '../../shared/types';

const Settings: React.FC = () => {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const result = await window.electronAPI.getConfig();
      if (result.success) {
        setConfig(result.config);
      }
    } catch (error) {
      console.error('Error loading config:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (!config) return;

    setSaving(true);
    try {
      await window.electronAPI.updateConfig(config);
    } catch (error) {
      console.error('Error saving config:', error);
    } finally {
      setSaving(false);
    }
  };

  const updateConfig = (key: keyof AppConfig, value: any) => {
    if (!config) return;
    setConfig({ ...config, [key]: value });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-foreground">Loading settings...</div>
      </div>
    );
  }

  if (!config) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-destructive">Failed to load settings</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border p-4">
        <h2 className="text-lg font-semibold">Settings</h2>
        <p className="text-sm text-muted-foreground mt-1">Configure your application</p>
      </div>

      {/* Settings form */}
      <div className="flex-1 overflow-y-auto p-4">
        <div className="max-w-2xl space-y-6">
          {/* Appearance */}
          <section>
            <h3 className="text-lg font-semibold mb-4">Appearance</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">Theme</label>
                <select
                  value={config.theme}
                  onChange={(e) => updateConfig('theme', e.target.value)}
                  className="w-full bg-background border border-border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                  <option value="sepia">Sepia</option>
                  <option value="system">System</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">Language</label>
                <select
                  value={config.language}
                  onChange={(e) => updateConfig('language', e.target.value)}
                  className="w-full bg-background border border-border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  <option value="en">English</option>
                  <option value="es">Español</option>
                  <option value="de">Deutsch</option>
                  <option value="ru">Русский</option>
                </select>
              </div>
            </div>
          </section>

          {/* Server */}
          <section>
            <h3 className="text-lg font-semibold mb-4">Server</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">API Port</label>
                <input
                  type="number"
                  value={config.apiPort}
                  onChange={(e) => updateConfig('apiPort', parseInt(e.target.value))}
                  className="w-full bg-background border border-border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-ring"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Default: 1234. Requires restart.
                </p>
              </div>

              <div className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  id="networkAccess"
                  checked={config.networkAccess}
                  onChange={(e) => updateConfig('networkAccess', e.target.checked)}
                  className="w-4 h-4 rounded border-border"
                />
                <label htmlFor="networkAccess" className="text-sm">
                  Allow network access (0.0.0.0)
                </label>
              </div>
              <p className="text-xs text-muted-foreground">
                Enable this to access the API from other devices on your network
              </p>
            </div>
          </section>

          {/* Advanced */}
          <section>
            <h3 className="text-lg font-semibold mb-4">Advanced</h3>
            <div className="space-y-4">
              <div className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  id="autoLoadModel"
                  checked={config.autoLoadModel}
                  onChange={(e) => updateConfig('autoLoadModel', e.target.checked)}
                  className="w-4 h-4 rounded border-border"
                />
                <label htmlFor="autoLoadModel" className="text-sm">
                  Auto-load last used model on startup
                </label>
              </div>

              <div className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  id="developerMode"
                  checked={config.developerMode}
                  onChange={(e) => updateConfig('developerMode', e.target.checked)}
                  className="w-4 h-4 rounded border-border"
                />
                <label htmlFor="developerMode" className="text-sm">
                  Developer mode (detailed logging)
                </label>
              </div>
            </div>
          </section>

          {/* Paths */}
          <section>
            <h3 className="text-lg font-semibold mb-4">Storage Paths</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">Models Path</label>
                <input
                  type="text"
                  value={config.modelsPath}
                  readOnly
                  className="w-full bg-muted border border-border rounded-lg px-3 py-2"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">Chats Path</label>
                <input
                  type="text"
                  value={config.chatsPath}
                  readOnly
                  className="w-full bg-muted border border-border rounded-lg px-3 py-2"
                />
              </div>
            </div>
          </section>
        </div>
      </div>

      {/* Footer */}
      <div className="border-t border-border p-4">
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-6 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90 disabled:opacity-50"
        >
          {saving ? 'Saving...' : 'Save Changes'}
        </button>
      </div>
    </div>
  );
};

export default Settings;
