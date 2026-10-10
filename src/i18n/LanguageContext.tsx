import React, { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react';
import enTranslations from '../locales/en.json';
import trTranslations from '../locales/tr.json';

export type SupportedLanguage = 'en' | 'tr';

type TranslationDictionary = Record<string, any>;

const translations: Record<SupportedLanguage, TranslationDictionary> = {
  en: enTranslations,
  tr: trTranslations,
};

interface LanguageContextType {
  language: SupportedLanguage;
  setLanguage: (lang: SupportedLanguage) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
  formatSeasonDate: (seasonRaw: string, day: number, year: number) => string;
  formatDateSummary: (dateSummaryStr: string | null | undefined) => string;
  formatNumber: (num: number) => string;
  formatPlaytimeHours: (hours: number | null | undefined) => string;
}

const LanguageContext = createContext<LanguageContextType | undefined>(undefined);

const LANGUAGE_STORAGE_KEY = 'stardew_sync_language';

// Helper to resolve nested key paths like "header.pageTitles.dashboard"
function getNestedValue(obj: any, path: string): string | undefined {
  if (!obj || typeof obj !== 'object') return undefined;
  const parts = path.split('.');
  let current = obj;
  for (const part of parts) {
    if (current && typeof current === 'object' && part in current) {
      current = current[part];
    } else {
      return undefined;
    }
  }
  return typeof current === 'string' ? current : undefined;
}

// Helper to replace parameters like "{count}" or "{name}"
function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, paramName) => {
    if (paramName in params) {
      return String(params[paramName]);
    }
    return match;
  });
}

export const LanguageProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [language, setLanguageState] = useState<SupportedLanguage>(() => {
    const saved = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (saved === 'en' || saved === 'tr') {
      return saved;
    }
    return 'en';
  });

  const setLanguage = useCallback((lang: SupportedLanguage) => {
    setLanguageState(lang);
    localStorage.setItem(LANGUAGE_STORAGE_KEY, lang);
  }, []);

  // Update HTML document lang attribute when language changes
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const t = useCallback(
    (key: string, params?: Record<string, string | number>): string => {
      // 1. Look up in active language dictionary
      let template = getNestedValue(translations[language], key);

      // 2. Fall back to English dictionary if missing
      if (template === undefined && language !== 'en') {
        template = getNestedValue(translations.en, key);
      }

      // 3. Fall back to key itself if missing from both
      if (template === undefined) {
        return key;
      }

      // 4. Interpolate variables
      return interpolate(template, params);
    },
    [language]
  );

  const formatSeasonDate = useCallback(
    (seasonRaw: string, day: number, year: number): string => {
      const s = seasonRaw.trim().toLowerCase();
      let seasonKey = 'spring';
      if (s === '1' || s === 'summer') seasonKey = 'summer';
      else if (s === '2' || s === 'fall' || s === 'autumn') seasonKey = 'fall';
      else if (s === '3' || s === 'winter') seasonKey = 'winter';

      const localizedSeason = t(`seasons.${seasonKey}`);
      const localizedDay = t('seasons.day');
      const localizedYear = t('seasons.year');

      const yearFormatted = language === 'tr' ? `${year}. ${localizedYear}` : `${localizedYear} ${year}`;
      return `${localizedSeason}, ${localizedDay} ${day} (${yearFormatted})`;
    },
    [t, language]
  );

  const formatDateSummary = useCallback(
    (dateSummaryStr: string | null | undefined): string => {
      if (!dateSummaryStr) return t('common.unknown');
      if (dateSummaryStr.trim().toLowerCase() === 'unknown date') return t('common.unknown');

      const match = dateSummaryStr.match(/^([a-zA-Z]+),\s*Day\s*(\d+)\s*\((?:Year\s*)?(\d+)\)$/i);
      if (match) {
        const season = match[1];
        const day = parseInt(match[2], 10);
        const year = parseInt(match[3], 10);
        return formatSeasonDate(season, day, year);
      }
      return dateSummaryStr;
    },
    [t, formatSeasonDate]
  );

  const formatNumber = useCallback(
    (num: number): string => {
      return new Intl.NumberFormat(language === 'tr' ? 'tr-TR' : 'en-US').format(num);
    },
    [language]
  );

  const formatPlaytimeHours = useCallback(
    (hours: number | null | undefined): string => {
      if (hours === null || hours === undefined || isNaN(hours)) {
        return t('common.unknown');
      }
      return `${hours.toFixed(1)} ${t('common.hours')}`;
    },
    [t]
  );

  const contextValue = useMemo(
    () => ({
      language,
      setLanguage,
      t,
      formatSeasonDate,
      formatDateSummary,
      formatNumber,
      formatPlaytimeHours,
    }),
    [language, setLanguage, t, formatSeasonDate, formatDateSummary, formatNumber, formatPlaytimeHours]
  );

  return <LanguageContext.Provider value={contextValue}>{children}</LanguageContext.Provider>;
};

export const useTranslation = (): LanguageContextType => {
  const context = useContext(LanguageContext);
  if (!context) {
    throw new Error('useTranslation must be used within a LanguageProvider');
  }
  return context;
};
