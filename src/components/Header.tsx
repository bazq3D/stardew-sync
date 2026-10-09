import React from 'react';
import { ProcessStatus, ThemeMode } from '../types';
import { RefreshCw, Sun, Moon, ShieldCheck } from 'lucide-react';

interface HeaderProps {
  title: string;
  processStatus: ProcessStatus | null;
  loadingProcess: boolean;
  onRefreshProcess: () => void;
  theme: ThemeMode;
  onToggleTheme: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  title,
  processStatus,
  loadingProcess,
  onRefreshProcess,
  theme,
  onToggleTheme,
}) => {
  const isRunning = processStatus?.is_stardew_running ?? false;

  return (
    <header className="top-header">
      <div className="header-left">
        <h1 className="page-title">{title}</h1>
      </div>

      <div className="header-right">
        {/* Game Process Status Pill */}
        <div
          id="status-game-process"
          className={`badge ${isRunning ? 'badge-warning' : 'badge-success'}`}
          title={
            isRunning
              ? 'Stardew Valley is running. Safe synchronization operations are locked.'
              : 'Stardew Valley is closed. Safe offline inspection active.'
          }
        >
          <span
            className={`pulse-dot ${
              isRunning ? 'pulse-dot-danger' : 'pulse-dot-active'
            }`}
          />
          <span>{isRunning ? 'Game Running' : 'Game Closed'}</span>
        </div>

        {/* Process Refresh Button */}
        <button
          id="btn-refresh-process"
          className="btn btn-secondary btn-sm"
          onClick={onRefreshProcess}
          disabled={loadingProcess}
          title="Refresh game process status"
        >
          <RefreshCw
            size={13}
            className={loadingProcess ? 'animate-spin' : ''}
          />
          <span>Check Status</span>
        </button>

        {/* Safety Badge */}
        <div
          id="badge-production-guard"
          className="badge badge-gold"
          title="ProductionGuard is active: save files are protected from writes"
        >
          <ShieldCheck size={12} />
          <span>ProductionGuard Active</span>
        </div>

        {/* Theme Toggle Button */}
        <button
          id="btn-toggle-theme"
          className="btn btn-ghost btn-icon"
          onClick={onToggleTheme}
          title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
          aria-label="Toggle theme"
        >
          {theme === 'dark' ? <Sun size={17} /> : <Moon size={17} />}
        </button>
      </div>
    </header>
  );
};
