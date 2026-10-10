import React, { useState, useRef, useEffect } from 'react';
import { ProcessStatus, ThemeMode } from '../types';
import { RefreshCw, Sun, Moon, ShieldCheck, Globe, Check } from 'lucide-react';
import { useTranslation, SupportedLanguage } from '../i18n/LanguageContext';

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
  const { language, setLanguage, t } = useTranslation();
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const isRunning = processStatus?.is_stardew_running ?? false;

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setDropdownOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setDropdownOpen(false);
      }
    };

    if (dropdownOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [dropdownOpen]);

  const handleSelectLanguage = (lang: SupportedLanguage) => {
    setLanguage(lang);
    setDropdownOpen(false);
  };

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
              ? t('header.gameRunningTooltip')
              : t('header.gameClosedTooltip')
          }
        >
          <span
            className={`pulse-dot ${
              isRunning ? 'pulse-dot-danger' : 'pulse-dot-active'
            }`}
          />
          <span>{isRunning ? t('header.gameRunning') : t('header.gameClosed')}</span>
        </div>

        {/* Process Refresh Button */}
        <button
          id="btn-refresh-process"
          className="btn btn-secondary btn-sm"
          onClick={onRefreshProcess}
          disabled={loadingProcess}
          title={t('header.refreshProcessTooltip')}
        >
          <RefreshCw
            size={13}
            className={loadingProcess ? 'animate-spin' : ''}
          />
          <span>{t('common.checkStatus')}</span>
        </button>

        {/* Safety Badge */}
        <div
          id="badge-production-guard"
          className="badge badge-gold"
          title={t('header.productionGuardTooltip')}
        >
          <ShieldCheck size={12} />
          <span>{t('header.productionGuardActive')}</span>
        </div>

        {/* Language Selector Dropdown */}
        <div className="lang-dropdown-wrapper" ref={dropdownRef}>
          <button
            id="btn-language-selector"
            className="btn btn-ghost btn-icon"
            onClick={() => setDropdownOpen((prev) => !prev)}
            title={t('header.selectLanguage')}
            aria-label={t('header.selectLanguage')}
            aria-haspopup="true"
            aria-expanded={dropdownOpen}
          >
            <Globe size={17} />
          </button>

          {dropdownOpen && (
            <div className="lang-dropdown-menu" role="menu">
              <button
                id="btn-lang-en"
                className={`lang-dropdown-item ${language === 'en' ? 'active' : ''}`}
                onClick={() => handleSelectLanguage('en')}
                role="menuitem"
              >
                <span>English</span>
                {language === 'en' && <Check size={14} />}
              </button>
              <button
                id="btn-lang-tr"
                className={`lang-dropdown-item ${language === 'tr' ? 'active' : ''}`}
                onClick={() => handleSelectLanguage('tr')}
                role="menuitem"
              >
                <span>Türkçe</span>
                {language === 'tr' && <Check size={14} />}
              </button>
            </div>
          )}
        </div>

        {/* Theme Toggle Button */}
        <button
          id="btn-toggle-theme"
          className="btn btn-ghost btn-icon"
          onClick={onToggleTheme}
          title={t('header.switchTheme', { theme: theme === 'dark' ? 'light' : 'dark' })}
          aria-label={t('header.toggleTheme')}
        >
          {theme === 'dark' ? <Sun size={17} /> : <Moon size={17} />}
        </button>
      </div>
    </header>
  );
};
