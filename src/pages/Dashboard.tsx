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
} from 'lucide-react';

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
  const prodFarm = farms.find((f) => f.is_production);
  const isRunning = processStatus?.is_stardew_running ?? false;

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* P2P Sync Status Alert Banner */}
      <div className="banner banner-warning">
        <CloudOff size={20} />
        <div>
          <strong>Phase 5.0 Desktop Foundation Notice:</strong> P2P Save
          Synchronization & automated cloud replication are currently <strong>Offline</strong>.
          This release establishes the local security, process monitoring, save
          discovery, and verified snapshot foundation. Remote sync transport will be
          implemented in Phase 6.0.
        </div>
      </div>

      {/* Primary Metrics Row */}
      <div className="metrics-row">
        {/* Metric 1: App Version */}
        <div className="metric-card">
          <div className="metric-label">
            <ShieldCheck size={14} style={{ color: 'var(--accent-primary)' }} />
            <span>App Version</span>
          </div>
          <div className="metric-value">
            v{appStatus?.app_version || '0.1.0'}
          </div>
          <div className="metric-sub">
            Built by bazq • Tauri 2 Native
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
            <span>Stardew Process</span>
          </div>
          <div className="metric-value">
            <span
              className={`badge ${
                isRunning ? 'badge-warning' : 'badge-success'
              }`}
            >
              {isRunning ? 'RUNNING' : 'CLOSED'}
            </span>
          </div>
          <div className="metric-sub">
            {isRunning
              ? 'Game is active. Safe lock engaged.'
              : 'Safe for metadata inspection.'}
          </div>
        </div>

        {/* Metric 3: Stardew Valley Saves */}
        <div
          className="metric-card"
          style={{ cursor: 'pointer' }}
          onClick={() => onNavigate('farms')}
          title="Click to view discovered farms"
        >
          <div className="metric-label">
            <FolderCheck size={14} style={{ color: 'var(--status-info)' }} />
            <span>Saves Directory</span>
          </div>
          <div className="metric-value">
            {farms.length} {farms.length === 1 ? 'Farm' : 'Farms'}
          </div>
          <div className="metric-sub">
            {farms.filter((f) => f.is_production).length} Production •{' '}
            {farms.filter((f) => f.is_disposable).length} Disposable
          </div>
        </div>

        {/* Metric 4: Verified Backups */}
        <div
          className="metric-card"
          style={{ cursor: 'pointer' }}
          onClick={() => onNavigate('backups')}
          title="Click to view local snapshots"
        >
          <div className="metric-label">
            <Archive size={14} style={{ color: 'var(--accent-gold)' }} />
            <span>Local Snapshots</span>
          </div>
          <div className="metric-value">
            {snapshots.length} {snapshots.length === 1 ? 'Snapshot' : 'Snapshots'}
          </div>
          <div className="metric-sub">
            SHA-256 Verified Immutable
          </div>
        </div>
      </div>

      {/* Main Production Farm Overview */}
      <div className="card">
        <div className="card-header">
          <div className="card-title">
            <Users size={18} style={{ color: 'var(--accent-gold)' }} />
            <span>Current Production Farm</span>
          </div>
          <span className="badge badge-gold">
            <Lock size={11} />
            PROTECTED
          </span>
        </div>

        {prodFarm ? (
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
                <span className="metric-label">Farm Name</span>
                <span style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-primary)' }}>
                  {prodFarm.farm_name} Çiftliği
                </span>
              </div>

              <div>
                <span className="metric-label">Active Host</span>
                <span style={{ fontSize: '16px', fontWeight: 600, color: 'var(--accent-gold)' }}>
                  {prodFarm.host_name}
                </span>
              </div>

              <div>
                <span className="metric-label">Farmhand</span>
                <span style={{ fontSize: '16px', fontWeight: 600, color: 'var(--accent-primary)' }}>
                  {prodFarm.farmhands.length > 0
                    ? prodFarm.farmhands.join(', ')
                    : 'elbi'}
                </span>
              </div>

              <div>
                <span className="metric-label">Calendar Date</span>
                <span style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '14px', fontWeight: 500 }}>
                  <Calendar size={13} style={{ color: 'var(--accent-primary)' }} />
                  {prodFarm.date_summary}
                </span>
              </div>

              <div>
                <span className="metric-label">Current Funds</span>
                <span style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '14px', fontWeight: 600, color: 'var(--accent-gold)' }}>
                  <Coins size={13} />
                  {prodFarm.money.toLocaleString()}g
                </span>
              </div>
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                Target Folder: <code className="code-box" style={{ padding: '2px 6px' }}>{prodFarm.folder_name}</code>
              </span>
              <button
                id="btn-inspect-production-farm"
                className="btn btn-secondary btn-sm"
                onClick={() => onInspectFarm(prodFarm.folder_name)}
              >
                <span>Inspect Save Metadata</span>
                <ArrowRight size={13} />
              </button>
            </div>
          </div>
        ) : (
          <div style={{ padding: '20px', textAlign: 'center', color: 'var(--text-muted)' }}>
            No production farm detected in saves directory.
          </div>
        )}
      </div>

      {/* Safety & Co-Op Roles Summary */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: '20px' }}>
        {/* Card: Host / Farmhand Pairing */}
        <div className="card">
          <div className="card-header">
            <div className="card-title">
              <Users size={16} />
              <span>Co-op Players & Hosting Role</span>
            </div>
          </div>
          <div className="kv-list">
            <div className="kv-item">
              <span className="kv-key">Primary Farm Host</span>
              <span className="badge badge-gold">Kubilay (Windows PC)</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">Connected Farmhand</span>
              <span className="badge badge-info">elbi (Partner PC)</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">Platform Persistence</span>
              <span className="kv-val">Xbox PC / MS Store Connected Storage</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">Host Migration Engine</span>
              <span className="badge badge-success">Validated (Strategy F)</span>
            </div>
          </div>
        </div>

        {/* Card: Safety & ProductionGuard */}
        <div className="card">
          <div className="card-header">
            <div className="card-title">
              <ShieldCheck size={16} style={{ color: 'var(--status-success)' }} />
              <span>Production Safety Controls</span>
            </div>
          </div>
          <div className="kv-list">
            <div className="kv-item">
              <span className="kv-key">Production Save Writes</span>
              <span className="badge badge-danger">LOCKED (Read-Only)</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">Xbox WGS Direct Writes</span>
              <span className="badge badge-danger">PROHIBITED</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">Automated Restore</span>
              <span className="badge badge-warning">DISABLED</span>
            </div>
            <div className="kv-item">
              <span className="kv-key">SHA-256 Checksum Matching</span>
              <span className="badge badge-success">ACTIVE</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
