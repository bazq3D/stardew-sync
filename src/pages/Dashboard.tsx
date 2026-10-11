import React, { useState, useEffect } from 'react';
import {
  AppStatus,
  ProcessStatus,
  FarmInfo,
  SnapshotInfo,
  ActiveTab,
  FarmMonitoringStatus,
  ObservationJournalEntry,
} from '../types';
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
  Activity,
  CheckCircle2,
  AlertTriangle,
  RefreshCw,
  Copy,
  Check,
  History,
  X,
  Clock,
  Hash,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';
import {
  fetchFarmMonitoringStatus,
  triggerMonitoringCheck,
  fetchFarmObservationJournal,
} from '../api/tauri';

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
  const currentFarm = farms.find((f) => f.is_legacy_production) || farms[0] || null;
  const isRunning = processStatus?.is_stardew_running ?? false;

  const [monitoringStatus, setMonitoringStatus] = useState<FarmMonitoringStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [copied, setCopied] = useState(false);
  const [journalOpen, setJournalOpen] = useState(false);
  const [journalEntries, setJournalEntries] = useState<ObservationJournalEntry[]>([]);
  const [journalLoading, setJournalLoading] = useState(false);

  useEffect(() => {
    if (currentFarm) {
      fetchFarmMonitoringStatus(currentFarm.folder_name)
        .then(setMonitoringStatus)
        .catch(console.error);
    }
  }, [currentFarm]);

  const handleCheckNow = async () => {
    if (!currentFarm || checking) return;
    setChecking(true);
    try {
      const updated = await triggerMonitoringCheck(currentFarm.folder_name);
      setMonitoringStatus(updated);
    } catch (err) {
      console.error('Failed to trigger monitoring check:', err);
    } finally {
      setChecking(false);
    }
  };

  const handleCopyFingerprint = () => {
    if (monitoringStatus?.current_fingerprint) {
      navigator.clipboard.writeText(monitoringStatus.current_fingerprint);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleOpenJournal = async () => {
    if (!currentFarm) return;
    setJournalOpen(true);
    setJournalLoading(true);
    try {
      const entries = await fetchFarmObservationJournal(currentFarm.folder_name);
      setJournalEntries(entries);
    } catch (err) {
      console.error('Failed to fetch observation journal:', err);
    } finally {
      setJournalLoading(false);
    }
  };

  const getStatusBadgeClass = (classification?: string) => {
    switch (classification) {
      case 'local_change_detected':
        return 'badge-success';
      case 'unchanged':
        return 'badge-secondary';
      case 'waiting_for_game_closure':
        return 'badge-warning';
      case 'waiting_for_file_stability':
        return 'badge-info';
      case 'invalid_or_incomplete_save':
      case 'source_missing':
      case 'monitoring_error':
        return 'badge-danger';
      default:
        return 'badge-secondary';
    }
  };

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
            v{appStatus?.app_version || '0.1.4'}
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

      {/* Phase 4: Local Save Monitoring & Change Detection Card */}
      {currentFarm && (
        <div className="card" style={{ borderLeft: '4px solid var(--accent-primary)' }}>
          <div className="card-header" style={{ flexWrap: 'wrap', gap: '10px' }}>
            <div className="card-title">
              <Activity size={18} style={{ color: 'var(--accent-primary)' }} />
              <span>{t('monitoring.title')}</span>
              <span className="badge badge-success" style={{ fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'currentColor' }} />
                {t('monitoring.active')}
              </span>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              {/* Sync Status Badge (Strictly labeled Sync Status Unknown per specification) */}
              <span
                className="badge badge-secondary"
                title={t('monitoring.syncUnknownDesc')}
                style={{ cursor: 'help', display: 'flex', alignItems: 'center', gap: '5px' }}
              >
                <CloudOff size={11} />
                {t('monitoring.syncUnknown')}
              </span>

              <button
                className="btn btn-secondary btn-sm"
                onClick={handleCheckNow}
                disabled={checking}
                title={t('monitoring.checkNow')}
              >
                <RefreshCw size={12} className={checking ? 'animate-spin' : ''} />
                <span>{checking ? t('monitoring.checking') : t('monitoring.checkNow')}</span>
              </button>

              <button
                className="btn btn-secondary btn-sm"
                onClick={handleOpenJournal}
                title={t('monitoring.viewJournal')}
              >
                <History size={12} />
                <span>{t('monitoring.viewJournal')}</span>
              </button>
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <p style={{ fontSize: '13px', color: 'var(--text-muted)', margin: 0 }}>
              {t('monitoring.subtitle')}
            </p>

            {/* Classification Alert Banner */}
            {monitoringStatus && (
              <div
                className={`banner ${
                  monitoringStatus.classification === 'waiting_for_game_closure'
                    ? 'banner-warning'
                    : monitoringStatus.classification === 'waiting_for_file_stability'
                    ? 'banner-info'
                    : monitoringStatus.classification === 'local_change_detected'
                    ? 'banner-success'
                    : monitoringStatus.classification === 'unchanged'
                    ? 'banner-info'
                    : 'banner-danger'
                }`}
                style={{ padding: '10px 14px' }}
              >
                {monitoringStatus.classification === 'waiting_for_game_closure' ? (
                  <Gamepad2 size={16} />
                ) : monitoringStatus.classification === 'waiting_for_file_stability' ? (
                  <RefreshCw size={16} className="animate-spin" />
                ) : monitoringStatus.classification === 'local_change_detected' ? (
                  <CheckCircle2 size={16} />
                ) : monitoringStatus.classification === 'unchanged' ? (
                  <CheckCircle2 size={16} />
                ) : (
                  <AlertTriangle size={16} />
                )}
                <div style={{ fontSize: '13px' }}>
                  <strong>
                    {t(`monitoring.status.${monitoringStatus.classification}`) || monitoringStatus.classification_label}:
                  </strong>{' '}
                  {monitoringStatus.classification === 'waiting_for_game_closure'
                    ? t('monitoring.banners.waitingGame')
                    : monitoringStatus.classification === 'waiting_for_file_stability'
                    ? t('monitoring.banners.waitingStability')
                    : monitoringStatus.classification === 'local_change_detected'
                    ? t('monitoring.banners.changeDetected')
                    : monitoringStatus.classification === 'unchanged'
                    ? t('monitoring.banners.unchanged')
                    : monitoringStatus.error_message || t('monitoring.banners.invalid')}
                </div>
              </div>
            )}

            {/* Monitoring Metrics Grid */}
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
                gap: '14px',
                padding: '14px',
                backgroundColor: 'var(--bg-card-subtle)',
                borderRadius: 'var(--radius-md)',
                border: '1px solid var(--border-subtle)',
              }}
            >
              {/* Metric: Current Fingerprint */}
              <div>
                <span className="metric-label">{t('monitoring.currentFingerprint')}</span>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '4px' }}>
                  <code
                    className="code-box"
                    style={{ fontSize: '13px', fontWeight: 600, color: 'var(--accent-primary)' }}
                    title={monitoringStatus?.current_fingerprint || ''}
                  >
                    {monitoringStatus?.short_fingerprint
                      ? `${monitoringStatus.short_fingerprint}...`
                      : 'None'}
                  </code>
                  {monitoringStatus?.current_fingerprint && (
                    <button
                      className="btn btn-ghost btn-sm"
                      style={{ padding: '2px 6px' }}
                      onClick={handleCopyFingerprint}
                      title={copied ? t('monitoring.copied') : t('monitoring.copyFingerprint')}
                    >
                      {copied ? <Check size={13} style={{ color: 'var(--status-success)' }} /> : <Copy size={13} />}
                    </button>
                  )}
                </div>
              </div>

              {/* Metric: Sequence Number */}
              <div>
                <span
                  className="metric-label"
                  title={t('monitoring.sequenceTooltip')}
                  style={{ cursor: 'help' }}
                >
                  {t('monitoring.observationSequence')} ℹ️
                </span>
                <span style={{ fontSize: '16px', fontWeight: 700, color: 'var(--text-primary)', marginTop: '4px', display: 'block' }}>
                  #{monitoringStatus?.observation_sequence ?? 1}
                </span>
              </div>

              {/* Metric: Last Observed */}
              <div>
                <span className="metric-label">{t('monitoring.lastObserved')}</span>
                <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-primary)', marginTop: '4px', display: 'block' }}>
                  {monitoringStatus?.last_observed_at
                    ? new Date(monitoringStatus.last_observed_at).toLocaleTimeString()
                    : t('common.unknown')}
                </span>
              </div>

              {/* Metric: Validation Status */}
              <div>
                <span className="metric-label">{t('monitoring.validationStatus')}</span>
                <span
                  className={`badge ${
                    monitoringStatus?.validation_status === 'valid' ? 'badge-success' : 'badge-danger'
                  }`}
                  style={{ marginTop: '4px', display: 'inline-block' }}
                >
                  {monitoringStatus?.validation_status === 'valid'
                    ? t('monitoring.valid')
                    : String(monitoringStatus?.validation_status ?? t('common.unknown'))}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}

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

      {/* Observation History Modal */}
      {journalOpen && currentFarm && (
        <div className="modal-backdrop" onClick={() => setJournalOpen(false)}>
          <div
            className="modal-container"
            style={{ maxWidth: '780px', width: '92%' }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <History size={18} style={{ color: 'var(--accent-primary)' }} />
                <h3 style={{ margin: 0 }}>
                  {t('monitoring.journalTitle', { farm: currentFarm.farm_name })}
                </h3>
              </div>
              <button className="btn btn-ghost btn-sm" onClick={() => setJournalOpen(false)}>
                <X size={16} />
              </button>
            </div>

            <div className="modal-body" style={{ maxHeight: '60vh', overflowY: 'auto' }}>
              <p style={{ fontSize: '13px', color: 'var(--text-muted)', marginBottom: '14px' }}>
                {t('monitoring.journalDesc')}
              </p>

              {journalLoading ? (
                <div style={{ padding: '32px', textAlign: 'center', color: 'var(--text-muted)' }}>
                  <RefreshCw size={24} className="animate-spin" style={{ margin: '0 auto 8px' }} />
                  <p>{t('common.loading')}</p>
                </div>
              ) : journalEntries.length === 0 ? (
                <div style={{ padding: '32px', textAlign: 'center', color: 'var(--text-muted)' }}>
                  <Info size={24} style={{ margin: '0 auto 8px', opacity: 0.6 }} />
                  <p>{t('monitoring.noEntries')}</p>
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                  {journalEntries
                    .slice()
                    .reverse()
                    .map((entry) => (
                      <div
                        key={entry.entry_id}
                        style={{
                          padding: '12px 14px',
                          borderRadius: 'var(--radius-md)',
                          border: '1px solid var(--border-subtle)',
                          backgroundColor: 'var(--bg-card-subtle)',
                          display: 'flex',
                          flexDirection: 'column',
                          gap: '8px',
                        }}
                      >
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                            <span
                              style={{
                                fontSize: '13px',
                                fontWeight: 700,
                                color: 'var(--accent-primary)',
                                display: 'flex',
                                alignItems: 'center',
                                gap: '3px',
                              }}
                            >
                              <Hash size={12} />
                              {entry.sequence_number}
                            </span>
                            <span className={`badge ${getStatusBadgeClass(entry.classification)}`} style={{ fontSize: '11px' }}>
                              {t(`monitoring.status.${entry.classification}`) || entry.classification}
                            </span>
                            <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                              • {t(`monitoring.reason.${entry.detection_reason}`) || entry.detection_reason}
                            </span>
                          </div>

                          <span style={{ fontSize: '11px', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '4px' }}>
                            <Clock size={11} />
                            {new Date(entry.last_seen).toLocaleString()}
                          </span>
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '12px' }}>
                          <div>
                            <span style={{ color: 'var(--text-muted)' }}>{t('monitoring.currentFingerprint')}: </span>
                            <code className="code-box" style={{ padding: '1px 4px', fontSize: '11px' }}>
                              {entry.current_fingerprint.slice(0, 12)}...
                            </code>
                          </div>

                          {entry.in_game_metadata?.in_game_date && (
                            <span style={{ color: 'var(--text-primary)', fontWeight: 500 }}>
                              {formatDateSummary(entry.in_game_metadata.in_game_date)}
                            </span>
                          )}
                        </div>
                      </div>
                    ))}
                </div>
              )}
            </div>

            <div className="modal-footer" style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <button className="btn btn-secondary" onClick={() => setJournalOpen(false)}>
                {t('common.close')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
