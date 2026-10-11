import React, { useState, useEffect } from 'react';
import { AppStatus, ProcessStatus, ThemeMode, AppSettings, BackgroundUpdateStatus } from '../types';
import {
  Palette,
  Globe,
  Folder,
  Cpu,
  Heart,
  Monitor,
  RefreshCw,
  Clock,
  AlertCircle,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';
import {
  fetchAppSettings,
  saveAppSettings,
  fetchAutostartStatus,
  setAutostartStatus,
  triggerBackgroundUpdateCheck,
  fetchBackgroundUpdateStatus,
} from '../api/tauri';

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
  const { t, language, setLanguage } = useTranslation();

  const [settings, setSettings] = useState<AppSettings>({
    autostart_enabled: false,
    start_minimized: false,
    minimize_to_tray_on_close: false,
    auto_check_updates: true,
    last_update_check: null,
    last_notified_version: null,
  });

  const [autostartActive, setAutostartActive] = useState<boolean>(false);
  const [updateStatus, setUpdateStatus] = useState<BackgroundUpdateStatus | null>(null);
  const [isChecking, setIsChecking] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    async function loadData() {
      try {
        const [appSet, osAutostart, upStatus] = await Promise.all([
          fetchAppSettings(),
          fetchAutostartStatus(),
          fetchBackgroundUpdateStatus(),
        ]);
        if (mounted) {
          setSettings(appSet);
          setAutostartActive(osAutostart);
          setUpdateStatus(upStatus);
        }
      } catch (err: any) {
        console.error('Failed to load settings data:', err);
      }
    }

    loadData();
    return () => {
      mounted = false;
    };
  }, []);

  const handleToggleAutostart = async () => {
    try {
      const nextState = !autostartActive;
      const res = await setAutostartStatus(nextState);
      setAutostartActive(res);
      setSettings((prev) => ({ ...prev, autostart_enabled: res }));
      setErrorMsg(null);
    } catch (err: any) {
      console.error('Autostart toggle failed:', err);
      setErrorMsg(String(err));
    }
  };

  const handleToggleStartMinimized = async () => {
    try {
      const updated = { ...settings, start_minimized: !settings.start_minimized };
      const saved = await saveAppSettings(updated);
      setSettings(saved);
    } catch (err: any) {
      console.error('Failed to save start_minimized setting:', err);
    }
  };

  const handleToggleMinimizeToTray = async () => {
    try {
      const updated = { ...settings, minimize_to_tray_on_close: !settings.minimize_to_tray_on_close };
      const saved = await saveAppSettings(updated);
      setSettings(saved);
    } catch (err: any) {
      console.error('Failed to save minimize_to_tray setting:', err);
    }
  };

  const handleToggleAutoCheck = async () => {
    try {
      const updated = { ...settings, auto_check_updates: !settings.auto_check_updates };
      const saved = await saveAppSettings(updated);
      setSettings(saved);
    } catch (err: any) {
      console.error('Failed to save auto_check_updates setting:', err);
    }
  };

  const handleCheckUpdatesNow = async () => {
    setIsChecking(true);
    setErrorMsg(null);
    try {
      const res = await triggerBackgroundUpdateCheck();
      setUpdateStatus(res);
      setSettings((prev) => ({
        ...prev,
        last_update_check: res.last_check_timestamp,
      }));
    } catch (err: any) {
      console.error('Manual update check error:', err);
      setErrorMsg(String(err));
    } finally {
      setIsChecking(false);
    }
  };

  const formatTimestamp = (iso: string | null) => {
    if (!iso) return t('settings.neverChecked');
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  };

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Settings Section: Appearance */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Palette size={16} />
            <span>{t('settings.themeTitle')}</span>
          </div>
        </div>

        <div style={{ display: 'flex', gap: '12px' }}>
          <button
            id="theme-btn-dark"
            className={`btn ${theme === 'dark' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => onSetTheme('dark')}
          >
            {t('settings.themeDark')}
          </button>
          <button
            id="theme-btn-light"
            className={`btn ${theme === 'light' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => onSetTheme('light')}
          >
            {t('settings.themeLight')}
          </button>
        </div>
      </div>

      {/* Settings Section: Language Preference */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Globe size={16} />
            <span>{t('settings.languageTitle')}</span>
          </div>
        </div>

        <div style={{ display: 'flex', gap: '12px' }}>
          <button
            id="settings-lang-btn-en"
            className={`btn ${language === 'en' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => setLanguage('en')}
          >
            {t('settings.langEn')}
          </button>
          <button
            id="settings-lang-btn-tr"
            className={`btn ${language === 'tr' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => setLanguage('tr')}
          >
            {t('settings.langTr')}
          </button>
        </div>
      </div>

      {/* Settings Section: Windows System Integration */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Monitor size={16} />
            <span>{t('settings.windowsIntegrationTitle')}</span>
          </div>
        </div>

        <p style={{ fontSize: '13px', color: 'var(--text-secondary)', marginBottom: '16px' }}>
          {t('settings.windowsIntegrationSubtitle')}
        </p>

        {errorMsg && (
          <div className="banner banner-warning" style={{ marginBottom: '16px' }}>
            <AlertCircle size={16} />
            <span>{errorMsg}</span>
          </div>
        )}

        <div className="kv-list">
          {/* Autostart Toggle */}
          <div className="kv-item" style={{ padding: '12px 0' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxWidth: '75%' }}>
              <span className="kv-val">{t('settings.autostartLabel')}</span>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                {t('settings.autostartDesc')}
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <span className={`badge ${autostartActive ? 'badge-success' : 'badge-info'}`}>
                {autostartActive ? t('settings.enabled') : t('settings.disabled')}
              </span>
              <button
                id="setting-toggle-autostart"
                className={`btn btn-sm ${autostartActive ? 'btn-primary' : 'btn-secondary'}`}
                onClick={handleToggleAutostart}
              >
                {autostartActive ? t('settings.enabled') : t('settings.disabled')}
              </button>
            </div>
          </div>

          {/* Start Minimized Toggle */}
          <div className="kv-item" style={{ padding: '12px 0' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxWidth: '75%' }}>
              <span className="kv-val">{t('settings.startMinimizedLabel')}</span>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                {t('settings.startMinimizedDesc')}
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <span className={`badge ${settings.start_minimized ? 'badge-success' : 'badge-info'}`}>
                {settings.start_minimized ? t('settings.enabled') : t('settings.disabled')}
              </span>
              <button
                id="setting-toggle-start-minimized"
                className={`btn btn-sm ${settings.start_minimized ? 'btn-primary' : 'btn-secondary'}`}
                onClick={handleToggleStartMinimized}
              >
                {settings.start_minimized ? t('settings.enabled') : t('settings.disabled')}
              </button>
            </div>
          </div>

          {/* Minimize to Tray on Close Toggle */}
          <div className="kv-item" style={{ padding: '12px 0' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxWidth: '75%' }}>
              <span className="kv-val">{t('settings.minimizeToTrayLabel')}</span>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                {t('settings.minimizeToTrayDesc')}
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <span className={`badge ${settings.minimize_to_tray_on_close ? 'badge-success' : 'badge-info'}`}>
                {settings.minimize_to_tray_on_close ? t('settings.enabled') : t('settings.disabled')}
              </span>
              <button
                id="setting-toggle-minimize-to-tray"
                className={`btn btn-sm ${settings.minimize_to_tray_on_close ? 'btn-primary' : 'btn-secondary'}`}
                onClick={handleToggleMinimizeToTray}
              >
                {settings.minimize_to_tray_on_close ? t('settings.enabled') : t('settings.disabled')}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Settings Section: Background Update Polling */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <RefreshCw size={16} />
            <span>{t('settings.backgroundUpdatesTitle')}</span>
          </div>
          <button
            id="setting-btn-check-updates"
            className="btn btn-sm btn-secondary"
            onClick={handleCheckUpdatesNow}
            disabled={isChecking}
          >
            <RefreshCw size={14} className={isChecking ? 'spin' : ''} />
            <span>{isChecking ? t('settings.checkingStatus') : t('settings.checkNowBtn')}</span>
          </button>
        </div>

        <p style={{ fontSize: '13px', color: 'var(--text-secondary)', marginBottom: '16px' }}>
          {t('settings.backgroundUpdatesSubtitle')}
        </p>

        <div className="kv-list">
          {/* Automatic Poller Toggle */}
          <div className="kv-item" style={{ padding: '12px 0' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxWidth: '75%' }}>
              <span className="kv-val">{t('settings.autoCheckLabel')}</span>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                {t('settings.autoCheckDesc')}
              </span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <span className={`badge ${settings.auto_check_updates ? 'badge-success' : 'badge-info'}`}>
                {settings.auto_check_updates ? t('settings.enabled') : t('settings.disabled')}
              </span>
              <button
                id="setting-toggle-auto-check"
                className={`btn btn-sm ${settings.auto_check_updates ? 'btn-primary' : 'btn-secondary'}`}
                onClick={handleToggleAutoCheck}
              >
                {settings.auto_check_updates ? t('settings.enabled') : t('settings.disabled')}
              </button>
            </div>
          </div>

          {/* Last Check Timestamp */}
          <div className="kv-item">
            <span className="kv-key">{t('settings.lastCheckLabel')}</span>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Clock size={14} color="var(--text-muted)" />
              <span className="kv-val">{formatTimestamp(settings.last_update_check)}</span>
            </div>
          </div>

          {/* Update Status Summary */}
          <div className="kv-item">
            <span className="kv-key">{t('settings.updateStatusLabel')}</span>
            <div>
              {updateStatus?.update_available ? (
                <span className="badge badge-warning">
                  {t('settings.updateAvailableStatus', { version: updateStatus.latest_version || '' })}
                </span>
              ) : updateStatus?.error ? (
                <span className="badge badge-danger">
                  {t('settings.checkFailedStatus', { error: updateStatus.error })}
                </span>
              ) : (
                <span className="badge badge-success">
                  {t('settings.upToDateStatus')}
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Settings Section: Directory & Environment Paths */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Folder size={16} />
            <span>{t('settings.pathsTitle')}</span>
          </div>
        </div>

        <div className="kv-list">
          <div className="kv-item">
            <span className="kv-key">{t('settings.savesDirLabel')}</span>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span className="code-box" style={{ padding: '2px 8px' }}>
                {appStatus?.saves_dir || 'Resolving...'}
              </span>
              <span
                className={`badge ${appStatus?.saves_dir_exists ? 'badge-success' : 'badge-danger'}`}
              >
                {appStatus?.saves_dir_exists ? t('dashboard.detected') : t('dashboard.notFound')}
              </span>
            </div>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('settings.wgsDirLabel')}</span>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span className="code-box" style={{ padding: '2px 8px' }}>
                {appStatus?.wgs_dir || 'Not detected on this machine'}
              </span>
              <span
                className={`badge ${appStatus?.wgs_dir_exists ? 'badge-success' : 'badge-info'}`}
              >
                {appStatus?.wgs_dir_exists ? t('settings.active') : t('settings.inactive')}
              </span>
            </div>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('settings.gameInstallStatus')}</span>
            <span
              className={`badge ${appStatus?.stardew_installed ? 'badge-success' : 'badge-warning'}`}
            >
              {appStatus?.stardew_installed ? t('settings.installedRecognized') : t('settings.notDetected')}
            </span>
          </div>
        </div>
      </div>

      {/* Settings Section: Diagnostic Information */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Cpu size={16} />
            <span>{t('settings.diagnosticsTitle')}</span>
          </div>
        </div>

        <div className="kv-list">
          <div className="kv-item">
            <span className="kv-key">{t('settings.os')}</span>
            <span className="kv-val">
              {appStatus?.platform} ({appStatus?.arch})
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('settings.checkedTargets')}</span>
            <span className="code-box" style={{ padding: '2px 8px' }}>
              {processStatus?.process_names_checked.join(', ') || 'N/A'}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('settings.processState')}</span>
            <span
              className={`badge ${processStatus?.is_stardew_running ? 'badge-warning' : 'badge-success'}`}
            >
              {processStatus?.is_stardew_running
                ? t('settings.gameActive')
                : t('settings.gameDormant')}
            </span>
          </div>

          <div className="kv-item">
            <span className="kv-key">{t('settings.lastPoll')}</span>
            <span className="kv-val">{processStatus?.checked_at || t('settings.justNow')}</span>
          </div>
        </div>
      </div>

      {/* Settings Section: Development Standards */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Heart size={16} />
            <span>{t('settings.madeWithLove')}</span>
          </div>
        </div>

        <div style={{ textAlign: 'center', padding: '12px 0' }}>
          <p style={{
            fontSize: '15px',
            color: 'var(--text-primary)',
            fontWeight: 500,
            marginBottom: '8px'
          }}>
            {t('settings.tagline')}
          </p>

          <p style={{
            fontSize: '13px',
            color: 'var(--text-secondary)',
            lineHeight: 1.7
          }}>
            {t('settings.madeBy')}
          </p>

          <p style={{
            fontSize: '12px',
            color: 'var(--text-secondary)',
            opacity: 0.65,
            marginTop: '8px',
            fontStyle: 'italic'
          }}>
            {t('settings.story')}
          </p>
        </div>
      </div>
    </div>
  );
};
