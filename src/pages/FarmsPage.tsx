import React from 'react';
import { FarmInfo } from '../types';
import {
  Sprout,
  ShieldCheck,
  FlaskConical,
  Lock,
  ArrowRight,
  RefreshCw,
  Info,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';

interface FarmsPageProps {
  farms: FarmInfo[];
  loading: boolean;
  onRefresh: () => void;
  onInspectFarm: (folderName: string) => void;
}

export const FarmsPage: React.FC<FarmsPageProps> = ({
  farms,
  loading,
  onRefresh,
  onInspectFarm,
}) => {
  const { t, formatNumber, formatDateSummary } = useTranslation();

  // Separate standard/active farms from registered test fixtures
  const standardFarms = farms.filter((f) => !f.is_test_fixture);
  const fixtureFarms = farms.filter((f) => f.is_test_fixture);

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Informational Banner */}
      <div className="banner banner-info" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <ShieldCheck size={20} />
          <div>
            <strong>{t('farms.bannerTitle')}</strong> {t('farms.bannerText')}
          </div>
        </div>
        <button
          className="btn btn-secondary btn-sm"
          onClick={onRefresh}
          disabled={loading}
          style={{ flexShrink: 0, marginLeft: '12px' }}
        >
          <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
          <span>{t('common.refresh')}</span>
        </button>
      </div>

      {/* Section 1: Standard / Active Player Farms */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '14px' }}>
          <Sprout size={18} style={{ color: 'var(--accent-primary)' }} />
          <h2 style={{ fontSize: '16px', fontWeight: 700 }}>
            {t('farms.discoveredFarms', { count: standardFarms.length })}
          </h2>
          <span className="badge badge-success">
            <Lock size={10} />
            {t('common.readOnly')}
          </span>
        </div>

        {standardFarms.length === 0 ? (
          <div className="card" style={{ padding: '32px', textAlign: 'center', color: 'var(--text-muted)' }}>
            <Info size={24} style={{ marginBottom: '8px', opacity: 0.7 }} />
            <p style={{ fontWeight: 600, color: 'var(--text-primary)', marginBottom: '4px' }}>
              {t('farms.noFarmsTitle')}
            </p>
            <p style={{ fontSize: '13px' }}>
              {t('farms.noFarmsDesc')}
            </p>
          </div>
        ) : (
          <div className="card-grid">
            {standardFarms.map((farm) => (
              <div
                key={farm.folder_name}
                id={`farm-card-${farm.folder_name}`}
                className="card card-clickable"
                onClick={() => onInspectFarm(farm.folder_name)}
              >
                <div className="card-header">
                  <div className="card-title">
                    <Sprout size={16} style={{ color: 'var(--accent-primary)' }} />
                    <span>{farm.farm_name}</span>
                  </div>
                  {farm.is_legacy_production ? (
                    <span className="badge badge-gold">{t('common.primaryBaseline')}</span>
                  ) : (
                    <span className="badge badge-success">{t('common.protected')}</span>
                  )}
                </div>

                <div className="card-body">
                  <div className="kv-list">
                    <div className="kv-item">
                      <span className="kv-key">{t('farms.farmHost')}</span>
                      <strong className="kv-val" style={{ color: 'var(--accent-gold)' }}>
                        {farm.host_name}
                      </strong>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('farms.farmhandsCount')}</span>
                      <span className="kv-val">
                        {farm.farmhands.length > 0
                          ? farm.farmhands.join(', ')
                          : t('dashboard.singlePlayer')}
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('farmModal.inGameCalendar')}</span>
                      <span className="kv-val">{formatDateSummary(farm.date_summary)}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('dashboard.currentFunds')}</span>
                      <span className="kv-val" style={{ color: 'var(--accent-gold)' }}>
                        {formatNumber(farm.money)}g
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('farmModal.saveFolder')}</span>
                      <span className="code-box" style={{ padding: '1px 6px' }}>{farm.folder_name}</span>
                    </div>
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '8px' }}>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      {t('farms.lastPlayed')}: {farm.last_modified}
                    </span>
                    <button className="btn btn-secondary btn-sm">
                      <span>{t('common.inspect')}</span>
                      <ArrowRight size={12} />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Section 2: Registered Test Fixtures (if any exist) */}
      {fixtureFarms.length > 0 && (
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '14px' }}>
            <FlaskConical size={18} style={{ color: 'var(--status-info)' }} />
            <h2 style={{ fontSize: '16px', fontWeight: 700 }}>
              {t('farms.registeredFixtures', { count: fixtureFarms.length })}
            </h2>
            <span className="badge badge-info">{t('common.testFixture')}</span>
          </div>

          <div className="card-grid">
            {fixtureFarms.map((farm) => (
              <div
                key={farm.folder_name}
                id={`farm-card-${farm.folder_name}`}
                className="card card-clickable"
                onClick={() => onInspectFarm(farm.folder_name)}
              >
                <div className="card-header">
                  <div className="card-title">
                    <FlaskConical size={16} style={{ color: 'var(--status-info)' }} />
                    <span>{farm.farm_name}</span>
                  </div>
                  <span className="badge badge-info">{t('common.testFixture')}</span>
                </div>

                <div className="card-body">
                  <div className="kv-list">
                    <div className="kv-item">
                      <span className="kv-key">{t('farms.farmHost')}</span>
                      <strong className="kv-val">{farm.host_name}</strong>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('farms.farmhandsCount')}</span>
                      <span className="kv-val">
                        {farm.farmhands.length > 0
                          ? farm.farmhands.join(', ')
                          : t('dashboard.singlePlayer')}
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('farmModal.inGameCalendar')}</span>
                      <span className="kv-val">{formatDateSummary(farm.date_summary)}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('dashboard.currentFunds')}</span>
                      <span className="kv-val">{formatNumber(farm.money)}g</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('farmModal.saveFolder')}</span>
                      <span className="code-box" style={{ padding: '1px 6px' }}>{farm.folder_name}</span>
                    </div>
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '8px' }}>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      {t('farms.lastPlayed')}: {farm.last_modified}
                    </span>
                    <button className="btn btn-secondary btn-sm">
                      <span>{t('common.inspect')}</span>
                      <ArrowRight size={12} />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
