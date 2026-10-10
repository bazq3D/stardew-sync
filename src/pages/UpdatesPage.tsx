import React, { useState, useEffect, useRef } from 'react';
import { AppStatus, ProcessStatus, UpdateCheckResult } from '../types';
import {
  isTauriEnvironment,
  fetchUpdateEligibility,
  reserveUpdateSlot,
  releaseUpdateSlot,
} from '../api/tauri';
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
import { useTranslation } from '../i18n/LanguageContext';

interface UpdatesPageProps {
  appStatus: AppStatus | null;
  processStatus: ProcessStatus | null;
  isSaveOperationActive?: boolean;
}

export const UpdatesPage: React.FC<UpdatesPageProps> = ({
  appStatus,
  processStatus,
  isSaveOperationActive = false,
}) => {
  const { t } = useTranslation();
  const [checking, setChecking] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [statusFeedback, setStatusFeedback] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const activeReservationTokenRef = useRef<string | null>(null);
  const isUpdateInProgressRef = useRef(false);

  // Unmount cleanup: if the component unmounts while holding an active reservation token,
  // safely release it ONLY IF no update is actively in progress. If an update is in progress,
  // preserve the reservation lock across view transitions to maintain fail-closed save safety.
  useEffect(() => {
    return () => {
      if (isUpdateInProgressRef.current) {
        console.warn('UpdatesPage unmounted during active update installation; preserving reservation lock.');
        return;
      }
      const token = activeReservationTokenRef.current;
      if (token) {
        releaseUpdateSlot(token).catch((err) => {
          console.warn('Cleanup: failed to release update reservation on unmount:', err);
        });
        activeReservationTokenRef.current = null;
      }
    };
  }, []);

  const isGameRunning = processStatus?.is_stardew_running ?? false;
  const currentAppVersion = appStatus?.app_version || '0.1.3';
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
            status_message: t('updates.upToDateMsg', { version: currentAppVersion }),
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
          release_notes: 'Desktop Foundation with secure signed auto-updater.',
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
    if (!pendingUpdate || isSaveOperationActive) return;

    // 1. Authoritative backend eligibility check
    try {
      const eligibility = await fetchUpdateEligibility(isSaveOperationActive);
      if (!eligibility.can_update) {
        setErrorMsg(eligibility.reason);
        return;
      }
    } catch (err: any) {
      console.warn('Backend update eligibility check warning:', err);
    }

    // 2. Authoritative backend reservation with unique ownership token to eliminate TOCTOU race condition
    let acquiredToken: string | null = null;
    try {
      acquiredToken = await reserveUpdateSlot();
      activeReservationTokenRef.current = acquiredToken;
      isUpdateInProgressRef.current = true;
    } catch (err: any) {
      setErrorMsg(`Update paused: ${err?.message || String(err)}`);
      return;
    }

    setDownloading(true);
    setErrorMsg(null);
    setStatusFeedback(t('updates.downloading'));

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
          setStatusFeedback(t('updates.verifiedLaunching'));
        }
      });
    } catch (err: any) {
      setErrorMsg(`Update installation failed: ${err?.message || String(err)}`);
      setDownloading(false);
    } finally {
      isUpdateInProgressRef.current = false;
      if (acquiredToken) {
        try {
          await releaseUpdateSlot(acquiredToken);
        } catch (releaseErr) {
          console.warn('Failed to release update reservation slot:', releaseErr);
        } finally {
          if (activeReservationTokenRef.current === acquiredToken) {
            activeReservationTokenRef.current = null;
          }
        }
      }
    }
  };

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Active Save Operation Pause Notice */}
      {isSaveOperationActive && (
        <div className="banner banner-warning">
          <Lock size={20} />
          <div>
            <strong>{t('updates.saveOpNoticeTitle')}</strong> {t('updates.saveOpNoticeText')}
          </div>
        </div>
      )}

      {/* Game Running Informational Notice (Decoupled from App Updates) */}
      {isGameRunning && (
        <div className="banner banner-info">
          <Info size={20} />
          <div>
            <strong>{t('updates.gameRunningNoticeTitle')}</strong> {t('updates.gameRunningNoticeText')}
          </div>
        </div>
      )}

      {/* Main Updater Status Card */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Sparkles size={18} style={{ color: 'var(--accent-primary)' }} />
            <span>{t('updates.title')}</span>
          </div>

          <button
            id="btn-check-updates"
            className="btn btn-primary"
            onClick={handleCheckUpdates}
            disabled={checking || downloading}
          >
            <RefreshCw size={14} className={checking ? 'animate-spin' : ''} />
            <span>{checking ? t('updates.btnChecking') : t('updates.btnCheck')}</span>
          </button>
        </div>

        <div className="kv-list">
          <div className="kv-item">
            <span className="kv-key">{t('updates.installedVersion')}</span>
            <span className="kv-val" style={{ fontWeight: 700 }}>
              v{currentAppVersion}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('updates.distributionTarget')}</span>
            <span className="code-box" style={{ padding: '2px 8px' }}>
              GitHub Releases ({repoUrl})
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('updates.signatureVerification')}</span>
            <span className="badge badge-success">
              <Key size={11} />
              {t('updates.signatureMethod')}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('updates.releaseEndpoint')}</span>
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
              <span>{t('common.status')}</span>
            </div>
            <span className="badge badge-success">{t('updates.upToDateBadge')}</span>
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
              <span>{t('updates.updateAvailable', { version: pendingUpdate.version })}</span>
            </div>
            <button
              id="btn-install-update"
              className="btn btn-primary"
              onClick={handleDownloadAndInstall}
              disabled={downloading || Boolean(isSaveOperationActive)}
            >
              <Download size={14} className={downloading ? 'animate-bounce' : ''} />
              <span>{downloading ? t('updates.btnInstalling') : t('updates.btnInstall')}</span>
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

            {/* Release notes kept in original language as required */}
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
                  {t('updates.releaseNotes')}
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
            <span>{t('updates.compatNoticeTitle')}</span>
          </div>
        </div>

        <div style={{ fontSize: '13px', color: 'var(--text-secondary)', lineHeight: 1.7 }}>
          <p style={{ marginBottom: '8px' }}>
            {t('updates.compatNoticeText1')}
          </p>
          <p style={{ marginBottom: '8px' }}>
            {t('updates.compatNoticeText2')}
          </p>
          <p>
            {t('updates.compatNoticeText3')}
          </p>
        </div>
      </div>

      {/* Safety & Cryptographic Policy Card */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <ShieldCheck size={16} />
            <span>{t('updates.securityPolicyTitle')}</span>
          </div>
        </div>

        <ul style={{ paddingLeft: '20px', fontSize: '13px', color: 'var(--text-secondary)', lineHeight: 1.7 }}>
          <li>
            {t('updates.securityPolicy1')}
          </li>
          <li>
            {t('updates.securityPolicy2')}
          </li>
          <li>
            {t('updates.securityPolicy3')}
          </li>
          <li>
            {t('updates.securityPolicy4')}
          </li>
        </ul>
      </div>
    </div>
  );
};
