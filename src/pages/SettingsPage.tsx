import React from 'react';
import { AppStatus, ProcessStatus, ThemeMode } from '../types';
import {
  Palette,
  Globe,
  Folder,
  Cpu,
  Heart,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';

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
