interface SidebarProps {
  currentPage: string;
  onPageChange: (page: string) => void;
  serverStatus: any;
  loadedModelsCount: number;
}

export default function Sidebar({
  currentPage,
  onPageChange,
  serverStatus,
  loadedModelsCount,
}: SidebarProps) {
  const NavButton = ({ page, label }: { page: string; label: string }) => (
    <button
      onClick={() => onPageChange(page)}
      className={`w-full text-left px-4 py-2 rounded-lg mb-1 transition-colors ${
        currentPage === page
          ? "bg-blue-600 text-white"
          : "text-gray-300 hover:bg-gray-800"
      }`}
    >
      {label}
    </button>
  );

  return (
    <div className="w-64 bg-gray-950 border-r border-gray-800 flex flex-col">
      <div className="p-4 border-b border-gray-800">
        <h1 className="text-xl font-bold">LMStudio Clone</h1>
        <p className="text-xs text-gray-400 mt-1">Rust Edition</p>
      </div>

      <nav className="flex-1 p-2">
        <NavButton page="chat" label="💬 Chat" />
        <NavButton page="models" label="🎯 Models" />
        <NavButton page="settings" label="⚙️ Settings" />
      </nav>

      <div className="p-4 border-t border-gray-800 text-xs space-y-2">
        <div className="flex justify-between">
          <span className="text-gray-400">Server:</span>
          <span
            className={
              serverStatus?.running ? "text-green-500" : "text-red-500"
            }
          >
            {serverStatus?.running ? "Running" : "Stopped"}
          </span>
        </div>
        {serverStatus?.running && (
          <>
            <div className="flex justify-between">
              <span className="text-gray-400">Version:</span>
              <span>{serverStatus.version}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Loaded:</span>
              <span>{loadedModelsCount} models</span>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
