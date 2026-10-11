import React, { useState, useEffect } from 'react';
import { FarmInfo, FarmMonitoringStatus } from '../types';
import {
  Sprout,
  ShieldCheck,
  FlaskConical,
  Lock,
  ArrowRight,
  RefreshCw,
  Info,
  CheckCircle2,
  AlertTriangle,
  Gamepad2,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';
import { fetchFarmMonitoringStatus, triggerMonitoringCheck } from '../api/tauri';

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
  const [statuses, setStatuses] = useState<Record<string, FarmMonitoringStatus>>({});
  const [checkingFarm, setCheckingFarm] = useState<string | null>(null);

  // Separate standard/active farms from registered test fixtures
  const standardFarms = farms.filter((f) => !f.is_test_fixture);
  const fixtureFarms = farms.filter((f) => f.is_test_fixture);

  useEffect(() => {
    // Fetch monitoring status for each farm
    standardFarms.forEach((farm) => {
      fetchFarmMonitoringStatus(farm.folder_name)
        .then((status) => {
          setStatuses((prev) => ({ ...prev, [farm.folder_name]: status }));
        })
        .catch(console.error);
    });
  }, [farms]);

  const handleCheckFarm = async (e: React.MouseEvent, folderName: string) => {
    e.stopPropagation();
    if (checkingFarm) return;
    setCheckingFarm(folderName);
    try {
      const updated = await triggerMonitoringCheck(folderName);
      setStatuses((prev) => ({ ...prev, [folderName]: updated }));
    } catch (err) {
      console.error('Failed to trigger monitoring check for farm:', folderName, err);
    } finally {
      setCheckingFarm(null);
    }
  };

  const getStatusBadge = (status?: FarmMonitoringStatus) => {
    if (!status) return null;
    const classification = status.classification;

    let badgeClass = 'badge-secondary';
    let icon = <CheckCircle2 size={11} />;

    if (classification === 'local_change_detected') {
      badgeClass = 'badge-success';
      icon = <CheckCircle2 size={11} />;
    } else if (classification === 'waiting_for_game_closure') {
      badgeClass = 'badge-warning';
      icon = <Gamepad2 size={11} />;
    } else if (classification === 'waiting_for_file_stability') {
      badgeClass = 'badge-info';
      icon = <RefreshCw size={11} className="animate-spin" />;
    } else if (
      classification === 'invalid_or_incomplete_save' ||
      classification === 'source_missing' ||
      classification === 'monitoring_error'
    ) {
      badgeClass = 'badge-danger';
      icon = <AlertTriangle size={11} />;
    }

    return (
      <span className={`badge ${badgeClass}`} style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '11px' }}>
        {icon}
        {t(`monitoring.status.${classification}`) || status.classification_label}
      </span>
    );
  };

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
            {standardFarms.map((farm) => {
              const status = statuses[farm.folder_name];
              const isCheckingThis = checkingFarm === farm.folder_name;

              return (
                <div
                  key={farm.folder_name}
                  id={`farm-card-${farm.folder_name}`}
                  className="card card-clickable"
                  onClick={() => onInspectFarm(farm.folder_name)}
                >
                  <div className="card-header" style={{ alignItems: 'flex-start' }}>
                    <div className="card-title">
                      <Sprout size={16} style={{ color: 'var(--accent-primary)' }} />
                      <span>{farm.farm_name}</span>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: '4px' }}>
                      {farm.is_legacy_production ? (
                        <span className="badge badge-gold">{t('common.primaryBaseline')}</span>
                      ) : (
                        <span className="badge badge-success">{t('common.protected')}</span>
                      )}
                      {getStatusBadge(status)}
                    </div>
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
                        <span className="kv-key">{t('monitoring.currentFingerprint')}</span>
                        <span className="code-box" style={{ padding: '1px 6px', fontSize: '11px' }}>
                          {status?.short_fingerprint ? `${status.short_fingerprint}...` : '—'}
                        </span>
                      </div>
                      <div className="kv-item">
                        <span className="kv-key">{t('farmModal.saveFolder')}</span>
                        <span className="code-box" style={{ padding: '1px 6px' }}>{farm.folder_name}</span>
                      </div>
                    </div>

                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '10px' }}>
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{ fontSize: '11px', padding: '3px 8px' }}
                        onClick={(e) => handleCheckFarm(e, farm.folder_name)}
                        disabled={isCheckingThis}
                        title={t('monitoring.checkNow')}
                      >
                        <RefreshCw size={11} className={isCheckingThis ? 'animate-spin' : ''} />
                        <span>{isCheckingThis ? t('monitoring.checking') : t('monitoring.checkNow')}</span>
                      </button>

                      <button className="btn btn-secondary btn-sm">
                        <span>{t('common.inspect')}</span>
                        <ArrowRight size={12} />
                      </button>
                    </div>
                  </div>
                </div>
              );
            })}
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

          <p style={{ fontSize: '13px', color: 'var(--text-muted)', marginBottom: '14px' }}>
            {t('farms.fixtureNotice')}
          </p>

          <div className="card-grid">
            {fixtureFarms.map((farm) => (
              <div
                key={farm.folder_name}
                id={`fixture-card-${farm.folder_name}`}
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
                      <span className="kv-key">{t('farmModal.saveFolder')}</span>
                      <span className="code-box" style={{ padding: '1px 6px' }}>{farm.folder_name}</span>
                    </div>
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '8px' }}>
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
