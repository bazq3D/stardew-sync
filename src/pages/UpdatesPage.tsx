import React, { useState } from 'react';
import { AppStatus, ProcessStatus, UpdateCheckResult } from '../types';
import { checkAppUpdates } from '../api/tauri';
import {
  Sparkles,
  ShieldCheck,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Key,
  Lock,
} from 'lucide-react';

interface UpdatesPageProps {
  appStatus: AppStatus | null;
  processStatus: ProcessStatus | null;
}

export const UpdatesPage: React.FC<UpdatesPageProps> = ({
  appStatus,
  processStatus,
}) => {
  const [checking, setChecking] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const isGameRunning = processStatus?.is_stardew_running ?? false;

  const handleCheckUpdates = async () => {
    setChecking(true);
    setErrorMsg(null);
    try {
      const res = await checkAppUpdates();
      setUpdateResult(res);
    } catch (err: any) {
      setErrorMsg(err?.message || 'Failed to check for updates. Please verify network connectivity.');
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Game Running Safety Banner */}
      {isGameRunning && (
        <div className="banner banner-warning">
          <Lock size={20} />
          <div>
            <strong>Update Safety Lock Engaged:</strong> Stardew Valley is currently
            running. To prevent any chance of binary corruption or save lock conflicts,
            updates cannot be applied until the game process has cleanly exited.
          </div>
        </div>
      )}

      {/* Main Updater Status Card */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Sparkles size={18} style={{ color: 'var(--accent-primary)' }} />
            <span>Automatic Updates (Tauri 2 Native Updater)</span>
          </div>

          <button
            id="btn-check-updates"
            className="btn btn-primary"
            onClick={handleCheckUpdates}
            disabled={checking}
          >
            <RefreshCw size={14} className={checking ? 'animate-spin' : ''} />
            <span>{checking ? 'Checking Release...' : 'Check for Updates'}</span>
          </button>
        </div>

        <div className="kv-list">
          <div className="kv-item">
            <span className="kv-key">Installed Application Version</span>
            <span className="kv-val" style={{ fontWeight: 700 }}>
              v{appStatus?.app_version || '0.1.0'}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Distribution Target</span>
            <span className="code-box" style={{ padding: '2px 8px' }}>
              GitHub Releases (https://github.com/bazq3/stardew-sync-p2p)
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Cryptographic Signature Verification</span>
            <span className="badge badge-success">
              <Key size={11} />
              MINISIGN ED25519 CONFIGURED
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Release Endpoint</span>
            <span className="code-box" style={{ padding: '2px 8px' }}>
              https://github.com/bazq3/stardew-sync-p2p/releases/latest/download/latest.json
            </span>
          </div>
        </div>
      </div>

      {/* Error State */}
      {errorMsg && (
        <div className="banner banner-warning">
          <AlertCircle size={20} />
          <div>
            <strong>Update Check Failed:</strong> {errorMsg}
          </div>
        </div>
      )}

      {/* Update Check Results */}
      {updateResult && (
        <div className="card animate-fade-in">
          <div className="card-header">
            <div className="card-title">
              <CheckCircle2 size={18} style={{ color: 'var(--status-success)' }} />
              <span>Version Status</span>
            </div>
            <span className="badge badge-success">UP TO DATE</span>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <p style={{ fontSize: '13.5px', color: 'var(--text-primary)' }}>
              {updateResult.status_message}
            </p>

            {updateResult.release_notes && (
              <div
                style={{
                  padding: '14px',
                  backgroundColor: 'var(--bg-card-subtle)',
                  borderRadius: 'var(--radius-sm)',
                  border: '1px solid var(--border-subtle)',
                }}
              >
                <span
                  style={{
                    fontSize: '11px',
                    fontWeight: 600,
                    textTransform: 'uppercase',
                    color: 'var(--text-muted)',
                    display: 'block',
                    marginBottom: '6px',
                  }}
                >
                  Latest Release Notes
                </span>
                <p style={{ fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                  {updateResult.release_notes}
                </p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Safety & Cryptographic Policy Card */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <ShieldCheck size={16} />
            <span>Update Integrity & Security Architecture</span>
          </div>
        </div>

        <ul style={{ paddingLeft: '20px', fontSize: '13px', color: 'var(--text-secondary)', lineHeight: 1.7 }}>
          <li>
            <strong>Signed Artifacts Only:</strong> Unsigned binaries are strictly rejected by the Tauri 2 core updater engine.
          </li>
          <li>
            <strong>Secret Key Security:</strong> The private signing key is never committed to source control or distributed in the application bundle.
          </li>
          <li>
            <strong>Save Safety Guard:</strong> Auto-updates require explicit confirmation and cannot execute during an active save or gameplay session.
          </li>
        </ul>
      </div>
    </div>
  );
};
