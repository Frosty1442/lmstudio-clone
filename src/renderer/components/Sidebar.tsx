import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Model, ServerStatus } from '../../shared/types';

interface SidebarProps {
  models: Model[];
  serverStatus: ServerStatus | null;
}

const Sidebar: React.FC<SidebarProps> = ({ models, serverStatus }) => {
  const location = useLocation();
  const loadedModels = models.filter((m) => m.loaded);

  const isActive = (path: string) => location.pathname === path;

  return (
    <div className="w-64 bg-card border-r border-border flex flex-col">
      <div className="p-4 border-b border-border">
        <h1 className="text-xl font-bold">LMStudio Clone</h1>
        <p className="text-xs text-muted-foreground mt-1">Open Source Edition</p>
      </div>

      <nav className="flex-1 p-2">
        <Link
          to="/"
          className={`block px-3 py-2 rounded-md mb-1 ${
            isActive('/') ? 'bg-primary text-primary-foreground' : 'hover:bg-accent'
          }`}
        >
          Chat
        </Link>
        <Link
          to="/models"
          className={`block px-3 py-2 rounded-md mb-1 ${
            isActive('/models') ? 'bg-primary text-primary-foreground' : 'hover:bg-accent'
          }`}
        >
          Models
        </Link>
        <Link
          to="/settings"
          className={`block px-3 py-2 rounded-md mb-1 ${
            isActive('/settings') ? 'bg-primary text-primary-foreground' : 'hover:bg-accent'
          }`}
        >
          Settings
        </Link>
      </nav>

      <div className="p-4 border-t border-border">
        <div className="text-xs space-y-2">
          <div className="flex justify-between">
            <span className="text-muted-foreground">Server:</span>
            <span className={serverStatus?.running ? 'text-green-500' : 'text-red-500'}>
              {serverStatus?.running ? 'Running' : 'Stopped'}
            </span>
          </div>
          {serverStatus?.running && (
            <div className="flex justify-between">
              <span className="text-muted-foreground">Port:</span>
              <span>{serverStatus.port}</span>
            </div>
          )}
          <div className="flex justify-between">
            <span className="text-muted-foreground">Loaded Models:</span>
            <span>{loadedModels.length}</span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Sidebar;
