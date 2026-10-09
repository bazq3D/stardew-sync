import React, { useState } from 'react';
import { AppStatus, ProcessStatus, UpdateCheckResult } from '../types';
import { isTauriEnvironment } from '../api/tauri';
import { check, Update } from '@tauri-apps/plugin-updater';
import {
  Sparkles,
  ShieldCheck,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Key,
  Lock,
  Download,
  Info,
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
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [statusFeedback, setStatusFeedback] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const isGameRunning = processStatus?.is_stardew_running ?? false;
  const currentAppVersion = appStatus?.app_version || '0.1.1';
  const repoUrl = 'https://github.com/bazq3D/stardew-sync';
  const releaseEndpoint = 'https://github.com/bazq3D/stardew-sync/releases/latest/download/latest.json';

  const handleCheckUpdates = async () => {
    setChecking(true);
    setErrorMsg(null);
    setStatusFeedback(null);
    setPendingUpdate(null);
    setDownloadProgress(null);

    try {
      if (isTauriEnvironment()) {
        const update = await check();
        if (update) {
          setPendingUpdate(update);
          setUpdateResult({
            current_version: update.currentVersion || currentAppVersion,
            endpoint: releaseEndpoint,
            public_key_configured: true,
            update_available: true,
            latest_version: update.version,
            release_notes: update.body || 'No release notes provided for this version.',
            status_message: `A new version (v${update.version}) is available for download!`,
          });
        } else {
          setUpdateResult({
            current_version: currentAppVersion,
            endpoint: releaseEndpoint,
            public_key_configured: true,
            update_available: false,
            latest_version: null,
            release_notes: null,
            status_message: `You are running the latest version of Stardew Sync (v${currentAppVersion}).`,
          });
        }
      } else {
        // Browser/preview fallback
        setUpdateResult({
          current_version: currentAppVersion,
          endpoint: releaseEndpoint,
          public_key_configured: true,
          update_available: false,
          latest_version: currentAppVersion,
          release_notes: 'Phase 5.2 Release-Ready Desktop Foundation with secure signed auto-updater.',
          status_message: `Browser preview mode: running mock Stardew Sync v${currentAppVersion}.`,
        });
      }
    } catch (err: any) {
      const rawError = err?.message || String(err);
      if (rawError.includes('404') || rawError.includes('release JSON') || rawError.includes('Could not fetch')) {
        setErrorMsg(
          `No published release manifest found on GitHub (HTTP 404 at ${releaseEndpoint}). Releases must be published on GitHub before remote updates can be retrieved.`
        );
      } else {
        setErrorMsg(rawError || 'Failed to check for updates. Please verify network connectivity.');
      }
    } finally {
      setChecking(false);
    }
  };

  const handleDownloadAndInstall = async () => {
    if (!pendingUpdate || isGameRunning) return;

    setDownloading(true);
    setErrorMsg(null);
    setStatusFeedback('Downloading signed update package...');

    try {
      let downloaded = 0;
      let total = 0;

      await pendingUpdate.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          if (total > 0) {
            setDownloadProgress(Math.round((downloaded / total) * 100));
          }
        } else if (event.event === 'Finished') {
          setStatusFeedback('Cryptographic signature verified. Launching installer...');
        }
      });
    } catch (err: any) {
      setErrorMsg(`Update installation failed: ${err?.message || String(err)}`);
      setDownloading(false);
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
            disabled={checking || downloading}
          >
            <RefreshCw size={14} className={checking ? 'animate-spin' : ''} />
            <span>{checking ? 'Checking Release...' : 'Check for Updates'}</span>
          </button>
        </div>

        <div className="kv-list">
          <div className="kv-item">
            <span className="kv-key">Installed Application Version</span>
            <span className="kv-val" style={{ fontWeight: 700 }}>
              v{currentAppVersion}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Distribution Target</span>
            <span className="code-box" style={{ padding: '2px 8px' }}>
              GitHub Releases ({repoUrl})
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Cryptographic Signature Verification</span>
            <span className="badge badge-success">
              <Key size={11} />
              MINISIGN ED25519 (PASSWORD-PROTECTED)
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">Release Endpoint</span>
            <span className="code-box" style={{ padding: '2px 8px' }}>
              {releaseEndpoint}
            </span>
          </div>
        </div>
      </div>

      {/* Error State */}
      {errorMsg && (
        <div className="banner banner-warning">
          <AlertCircle size={20} />
          <div>
            <strong>Update Diagnostic:</strong> {errorMsg}
          </div>
        </div>
      )}

      {/* Update Check Results (Up to Date) */}
      {updateResult && !updateResult.update_available && (
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
          </div>
        </div>
      )}

      {/* Update Check Results (Update Available) */}
      {pendingUpdate && (
        <div className="card animate-fade-in" style={{ border: '2px solid var(--accent-primary)' }}>
          <div className="card-header">
            <div className="card-title">
              <Sparkles size={18} style={{ color: 'var(--accent-primary)' }} />
              <span>New Release Available: v{pendingUpdate.version}</span>
            </div>
            <button
              id="btn-install-update"
              className="btn btn-primary"
              onClick={handleDownloadAndInstall}
              disabled={downloading || isGameRunning}
            >
              <Download size={14} className={downloading ? 'animate-bounce' : ''} />
              <span>{downloading ? 'Installing...' : 'Download & Install Update'}</span>
            </button>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
            {statusFeedback && (
              <p style={{ fontSize: '13px', color: 'var(--accent-primary)', fontWeight: 600 }}>
                {statusFeedback}
              </p>
            )}

            {downloadProgress !== null && (
              <div style={{ width: '100%', backgroundColor: 'var(--bg-card-subtle)', borderRadius: '4px', overflow: 'hidden', height: '8px' }}>
                <div
                  style={{
                    width: `${downloadProgress}%`,
                    height: '100%',
                    backgroundColor: 'var(--accent-primary)',
                    transition: 'width 0.3s ease',
                  }}
                />
              </div>
            )}

            {pendingUpdate.body && (
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
                  Release Notes
                </span>
                <p style={{ fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: 1.5, whiteSpace: 'pre-line' }}>
                  {pendingUpdate.body}
                </p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Upgrade Compatibility Advisory Card */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Info size={16} style={{ color: 'var(--accent-secondary)' }} />
            <span>Upgrade Compatibility & Legacy Transition Notice</span>
          </div>
        </div>

        <div style={{ fontSize: '13px', color: 'var(--text-secondary)', lineHeight: 1.7 }}>
          <p style={{ marginBottom: '8px' }}>
            <strong>Upgrading from Installed v0.1.0 to v0.1.1:</strong> The initial development build (v0.1.0) trusted an unencrypted development key and a preliminary repository URL. Because that preliminary endpoint does not host releases, in-app updating from v0.1.0 to v0.1.1 is intentionally disabled.
          </p>
          <p style={{ marginBottom: '8px' }}>
            To upgrade to v0.1.1, run the signed Windows NSIS setup package (<code>Stardew Sync_0.1.1_x64-setup.exe</code>) manually once. Your settings and saves will remain completely intact.
          </p>
          <p>
            <strong>Subsequent Updates (v0.1.1+):</strong> Future updates will be retrieved, cryptographically verified with password-protected Minisign keys, and installed automatically in-app via GitHub Releases.
          </p>
        </div>
      </div>

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
            <strong>Secret Key Security:</strong> The private signing key is stored outside the repository with restricted user ACLs and protected by a strong passphrase.
          </li>
          <li>
            <strong>Save Safety Guard:</strong> Auto-updates require explicit confirmation and cannot execute during an active save or gameplay session.
          </li>
        </ul>
      </div>
    </div>
  );
};
