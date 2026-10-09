import React from 'react';
import { AppStatus, ProcessStatus, ThemeMode } from '../types';
import {
  Palette,
  Folder,
  Cpu,
  Heart,
} from 'lucide-react';

interface SettingsPageProps {
  appStatus: AppStatus | null;
  processStatus: ProcessStatus | null;
  theme: ThemeMode;
  onSetTheme: (theme: ThemeMode) => void;
}

export const SettingsPage: React.FC<SettingsPageProps> = ({
  appStatus,
  processStatus,
  theme,
  onSetTheme,
}) => {
  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Settings Section: Appearance */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Palette size={16} />
            <span>Theme & Appearance</span>
          </div>
        </div>

        <div style={{ display: 'flex', gap: '12px' }}>
          <button
            id="theme-btn-dark"
            className={`btn ${theme === 'dark' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => onSetTheme('dark')}
          >
            Dark Mode (Forest Slate)
          </button>
          <button
            id="theme-btn-light"
            className={`btn ${theme === 'light' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => onSetTheme('light')}
          >
            Light Mode (Parchment Meadow)
          </button>
        </div>
      </div>

      {/* Settings Section: Directory & Environment Paths */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Folder size={16} />
            <span>Stardew Valley Save Locations</span>
          </div>
        </div>

        <div className="kv-list">
          <div className="kv-item">
            <span className="kv-key">Working Saves Directory (%APPDATA%)</span>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span className="code-box" style={{ padding: '2px 8px' }}>
                {appStatus?.saves_dir || 'Resolving...'}
              </span>
              <span
                className={`badge ${appStatus?.saves_dir_exists ? 'badge-success' : 'badge-danger'
                  }`}
              >
                {appStatus?.saves_dir_exists ? 'DETECTED' : 'NOT FOUND'}
              </span>
            </div>
          </div>

          <div className="kv-item">
            <span className="kv-key">Xbox WGS Connected Storage</span>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span className="code-box" style={{ padding: '2px 8px' }}>
                {appStatus?.wgs_dir || 'Not detected on this machine'}
              </span>
              <span
                className={`badge ${appStatus?.wgs_dir_exists ? 'badge-success' : 'badge-info'
                  }`}
              >
                {appStatus?.wgs_dir_exists ? 'ACTIVE' : 'INACTIVE'}
              </span>
            </div>
          </div>

          <div className="kv-item">
            <span className="kv-key">Game Installation Status</span>
            <span
              className={`badge ${appStatus?.stardew_installed ? 'badge-success' : 'badge-warning'
                }`}
            >
              {appStatus?.stardew_installed ? 'INSTALLED & RECOGNIZED' : 'NOT DETECTED'}
            </span>
          </div>
        </div>
      </div>

      {/* Settings Section: Diagnostic Information */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Cpu size={16} />
            <span>System Diagnostics & Environment</span>
          </div>
        </div>

        <div className="kv-list">
          <div className="kv-item">
            <span className="kv-key">Operating System</span>
            <span className="kv-val">
              {appStatus?.platform} ({appStatus?.arch})
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Process Monitor Checked Targets</span>
            <span className="code-box" style={{ padding: '2px 8px' }}>
              {processStatus?.process_names_checked.join(', ') || 'N/A'}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Process Detection State</span>
            <span
              className={`badge ${processStatus?.is_stardew_running ? 'badge-warning' : 'badge-success'
                }`}
            >
              {processStatus?.is_stardew_running
                ? 'GAME ACTIVE (LOCK ENGAGED)'
                : 'GAME DORMANT (SAFE)'}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Last Diagnostic Poll</span>
            <span className="kv-val">{processStatus?.checked_at || 'Just now'}</span>
          </div>
        </div>
      </div>

      {/* Settings Section: Development Standards */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Heart size={16} />
            <span>Made with Love</span>
          </div>
        </div>

        <div style={{ textAlign: 'center', padding: '12px 0' }}>
          <p style={{
            fontSize: '15px',
            color: 'var(--text-primary)',
            fontWeight: 500,
            marginBottom: '8px'
          }}>
            Your farm. Your progress. Wherever you play.
          </p>

          <p style={{
            fontSize: '13px',
            color: 'var(--text-secondary)',
            lineHeight: 1.7
          }}>
            Made with 💚 by <strong>bazq</strong>
          </p>

          <p style={{
            fontSize: '12px',
            color: 'var(--text-secondary)',
            opacity: 0.65,
            marginTop: '8px',
            fontStyle: 'italic'
          }}>
            Two farmers. One shared story.
          </p>
        </div>
      </div>
    </div>
  );
};
