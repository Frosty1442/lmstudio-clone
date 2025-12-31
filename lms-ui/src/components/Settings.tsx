import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

interface SettingsProps {
  onError?: (title: string, message: string) => void;
  onSuccess?: (title: string, message: string) => void;
}

export default function Settings({ onError, onSuccess }: SettingsProps) {
  const [serverUrl, setServerUrl] = useState("http://localhost:1234");
  const [saved, setSaved] = useState(false);

  const handleSave = async () => {
    try {
      await invoke("set_server_url", { url: serverUrl });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
      onSuccess?.("Settings Saved", "Server URL updated successfully");
    } catch (error) {
      console.error("Error saving settings:", error);
      onError?.("Failed to Save", String(error));
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-gray-800 p-4">
        <h2 className="text-lg font-semibold">Settings</h2>
        <p className="text-sm text-gray-400 mt-1">Configure your application</p>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        <div className="max-w-2xl space-y-6">
          <section>
            <h3 className="text-lg font-semibold mb-4">Server Connection</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">
                  Server URL
                </label>
                <input
                  type="text"
                  value={serverUrl}
                  onChange={(e) => setServerUrl(e.target.value)}
                  className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="http://localhost:1234"
                />
                <p className="text-xs text-gray-400 mt-1">
                  The URL of your lms-server instance
                </p>
              </div>

              <button
                onClick={handleSave}
                className={`px-6 py-2 rounded-lg transition-colors ${
                  saved
                    ? "bg-green-600 hover:bg-green-700"
                    : "bg-blue-600 hover:bg-blue-700"
                } text-white`}
              >
                {saved ? "✓ Saved!" : "Save Changes"}
              </button>
            </div>
          </section>

          <section>
            <h3 className="text-lg font-semibold mb-4">About</h3>
            <div className="bg-gray-800 rounded-lg p-4 space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Application:</span>
                <span>LMStudio Clone</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Version:</span>
                <span>0.1.0</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Built with:</span>
                <span>Rust + Tauri + React</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">License:</span>
                <span>MIT</span>
              </div>
            </div>
          </section>

          <section>
            <h3 className="text-lg font-semibold mb-4">Server Info</h3>
            <div className="bg-gray-800 rounded-lg p-4 space-y-2 text-sm">
              <p className="text-gray-400">
                Make sure the lms-server is running:
              </p>
              <pre className="bg-gray-900 p-3 rounded mt-2 text-green-400">
                $ cargo run --bin lms-server
              </pre>
              <p className="text-gray-400 mt-2">Or build and run:</p>
              <pre className="bg-gray-900 p-3 rounded mt-2 text-green-400">
                $ cargo build --release{"\n"}
                $ ./target/release/lms-server
              </pre>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
