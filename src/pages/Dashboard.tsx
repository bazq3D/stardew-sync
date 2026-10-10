import React from 'react';
import { AppStatus, ProcessStatus, FarmInfo, SnapshotInfo, ActiveTab } from '../types';
import {
  ShieldCheck,
  Gamepad2,
  Users,
  Coins,
  Calendar,
  CloudOff,
  FolderCheck,
  Archive,
  ArrowRight,
  Lock,
  Sprout,
  Info,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';

interface DashboardProps {
  appStatus: AppStatus | null;
  processStatus: ProcessStatus | null;
  farms: FarmInfo[];
  snapshots: SnapshotInfo[];
  onNavigate: (tab: ActiveTab) => void;
  onInspectFarm: (folderName: string) => void;
}

export const Dashboard: React.FC<DashboardProps> = ({
  appStatus,
  processStatus,
  farms,
  snapshots,
  onNavigate,
  onInspectFarm,
}) => {
  const { t, formatNumber, formatDateSummary } = useTranslation();
  // Select the current farm: either legacy production if present, or the first discovered farm
  const currentFarm = farms.find((f) => f.is_legacy_production) || farms[0] || null;
  const isRunning = processStatus?.is_stardew_running ?? false;

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* P2P Sync Status Alert Banner */}
      <div className="banner banner-warning">
        <CloudOff size={20} />
        <div>
          <strong>{t('dashboard.bannerTitle')}</strong> {t('dashboard.bannerNotice')}
        </div>
      </div>

      {/* Primary Metrics Row */}
      <div className="metrics-row">
        {/* Metric 1: App Version */}
        <div className="metric-card">
          <div className="metric-label">
            <ShieldCheck size={14} style={{ color: 'var(--accent-primary)' }} />
            <span>{t('dashboard.appVersion')}</span>
          </div>
          <div className="metric-value">
            v{appStatus?.app_version || '0.1.3'}
          </div>
          <div className="metric-sub">
            {t('dashboard.builtBy')}
          </div>
        </div>

        {/* Metric 2: Game Status */}
        <div className="metric-card">
          <div className="metric-label">
            <Gamepad2
              size={14}
              style={{
                color: isRunning ? 'var(--status-warning)' : 'var(--status-success)',
              }}
            />
            <span>{t('dashboard.stardewProcess')}</span>
          </div>
          <div className="metric-value">
            <span
              className={`badge ${
                isRunning ? 'badge-warning' : 'badge-success'
              }`}
            >
              {isRunning ? t('dashboard.running') : t('dashboard.closed')}
            </span>
          </div>
          <div className="metric-sub">
            {isRunning
              ? t('dashboard.gameActiveSub')
              : t('dashboard.gameDormantSub')}
          </div>
        </div>

        {/* Metric 3: Stardew Valley Saves */}
        <div
          className="metric-card"
          style={{ cursor: 'pointer' }}
          onClick={() => onNavigate('farms')}
          title={t('farms.inspectButton')}
        >
          <div className="metric-label">
            <FolderCheck size={14} style={{ color: 'var(--status-info)' }} />
            <span>{t('dashboard.savesDir')}</span>
          </div>
          <div className="metric-value">
            {t('dashboard.farmsCount', {
              count: farms.length,
              unit: farms.length === 1 ? t('dashboard.farmSingle') : t('dashboard.farmPlural'),
            })}
          </div>
          <div className="metric-sub">
            {farms.length > 0 ? t('dashboard.discoveredProtected') : t('dashboard.noSavesDetected')}
          </div>
        </div>

        {/* Metric 4: Verified Backups */}
        <div
          className="metric-card"
          style={{ cursor: 'pointer' }}
          onClick={() => onNavigate('backups')}
          title={t('backups.title')}
        >
          <div className="metric-label">
            <Archive size={14} style={{ color: 'var(--accent-gold)' }} />
            <span>{t('dashboard.localSnapshots')}</span>
          </div>
          <div className="metric-value">
            {t('dashboard.snapshotsCount', {
              count: snapshots.length,
              unit: snapshots.length === 1 ? t('dashboard.snapshotSingle') : t('dashboard.snapshotPlural'),
            })}
          </div>
          <div className="metric-sub">
            {t('dashboard.sha256Immutable')}
          </div>
        </div>
      </div>

      {/* Main Farm Overview */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Sprout size={18} style={{ color: 'var(--accent-primary)' }} />
            <span>{t('dashboard.currentFarmOverview')}</span>
          </div>
          <span className="badge badge-success">
            <Lock size={11} />
            {t('dashboard.protectedSave')}
          </span>
        </div>

        {currentFarm ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                gap: '16px',
                padding: '16px',
                backgroundColor: 'var(--bg-card-subtle)',
                borderRadius: 'var(--radius-md)',
                border: '1px solid var(--border-subtle)',
              }}
            >
              <div>
                <span className="metric-label">{t('dashboard.farmName')}</span>
                <span style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-primary)' }}>
                  {currentFarm.farm_name}
                </span>
              </div>

              <div>
                <span className="metric-label">{t('dashboard.farmHost')}</span>
                <span style={{ fontSize: '16px', fontWeight: 600, color: 'var(--accent-gold)' }}>
                  {currentFarm.host_name}
                </span>
              </div>

              <div>
                <span className="metric-label">{t('dashboard.farmhands')}</span>
                <span style={{ fontSize: '15px', fontWeight: 500, color: 'var(--accent-primary)' }}>
                  {currentFarm.farmhands.length > 0
                    ? currentFarm.farmhands.join(', ')
                    : t('dashboard.singlePlayer')}
                </span>
              </div>

              <div>
                <span className="metric-label">{t('dashboard.inGameCalendar')}</span>
                <span style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '14px', fontWeight: 500 }}>
                  <Calendar size={13} style={{ color: 'var(--accent-primary)' }} />
                  {formatDateSummary(currentFarm.date_summary)}
                </span>
              </div>

              <div>
                <span className="metric-label">{t('dashboard.currentFunds')}</span>
                <span style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '14px', fontWeight: 600, color: 'var(--accent-gold)' }}>
                  <Coins size={13} />
                  {formatNumber(currentFarm.money)}g
                </span>
              </div>
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                {t('dashboard.targetFolder')}{' '}
                <code className="code-box" style={{ padding: '2px 6px' }}>{currentFarm.folder_name}</code>
              </span>
              <button
                id="btn-inspect-production-farm"
                className="btn btn-secondary btn-sm"
                onClick={() => onInspectFarm(currentFarm.folder_name)}
              >
                <span>{t('dashboard.inspectMetadata')}</span>
                <ArrowRight size={13} />
              </button>
            </div>
          </div>
        ) : (
          <div style={{ padding: '32px', textAlign: 'center', color: 'var(--text-muted)' }}>
            <Info size={24} style={{ marginBottom: '8px', opacity: 0.7 }} />
            <p style={{ fontWeight: 600, color: 'var(--text-primary)', marginBottom: '4px' }}>
              {t('dashboard.noFarmsFound')}
            </p>
            <p style={{ fontSize: '13px' }}>
              {t('dashboard.noFarmsHelp')}
            </p>
          </div>
        )}
      </div>

      {/* Roles & Generic Safety Summary */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: '20px' }}>
        {/* Card: Host / Farmhand Pairing */}
        <div className="card">
          <div className="card-header">
            <div className="card-title">
              <Users size={16} />
              <span>{t('dashboard.coopRoles')}</span>
            </div>
          </div>
          <div className="kv-list">
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.primaryHost')}</span>
              <span className="badge badge-gold">
                {currentFarm ? currentFarm.host_name : t('dashboard.noHostDetected')}
              </span>
            </div>
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.connectedFarmhands')}</span>
              <span className="kv-val">
                {currentFarm && currentFarm.farmhands.length > 0
                  ? currentFarm.farmhands.join(', ')
                  : t('dashboard.singlePlayer')}
              </span>
            </div>
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.connectedDevices')}</span>
              <span className="kv-val">{t('dashboard.localPc')}</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.migrationEngine')}</span>
              <span className="badge badge-success">{t('dashboard.migrationModel')}</span>
            </div>
          </div>
        </div>

        {/* Card: Safety & ProductionGuard */}
        <div className="card">
          <div className="card-header">
            <div className="card-title">
              <ShieldCheck size={16} style={{ color: 'var(--status-success)' }} />
              <span>{t('dashboard.safetyControls')}</span>
            </div>
          </div>
          <div className="kv-list">
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.allSavesProtection')}</span>
              <span className="badge badge-danger">{t('dashboard.readOnlyDefaultDeny')}</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.wgsWrites')}</span>
              <span className="badge badge-danger">{t('dashboard.prohibited')}</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.automatedRestore')}</span>
              <span className="badge badge-warning">{t('dashboard.disabled')}</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">{t('dashboard.checksumMatching')}</span>
              <span className="badge badge-success">{t('dashboard.active')}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
