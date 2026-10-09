import React from 'react';
import { FarmInfo } from '../types';
import {
  Sprout,
  ShieldCheck,
  FlaskConical,
  Lock,
  ArrowRight,
  RefreshCw,
} from 'lucide-react';

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
  const prodFarms = farms.filter((f) => f.is_production);
  const testFarms = farms.filter((f) => f.is_disposable || !f.is_production);

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Informational Banner */}
      <div className="banner banner-info" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <ShieldCheck size={20} />
          <div>
            <strong>Read-Only Farm Discovery:</strong> All saves detected in{' '}
            <code>%APPDATA%\StardewValley\Saves</code> are shown below. Production saves
            are strictly protected against modification. Destructive controls and
            arbitrary file writes are completely disabled.
          </div>
        </div>
        <button
          className="btn btn-secondary btn-sm"
          onClick={onRefresh}
          disabled={loading}
          style={{ flexShrink: 0, marginLeft: '12px' }}
        >
          <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
          <span>Refresh</span>
        </button>
      </div>

      {/* Section 1: Production Farms */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '14px' }}>
          <ShieldCheck size={18} style={{ color: 'var(--accent-gold)' }} />
          <h2 style={{ fontSize: '16px', fontWeight: 700 }}>
            Production Farm (Protected by ProductionGuard)
          </h2>
          <span className="badge badge-gold">
            <Lock size={10} />
            READ-ONLY
          </span>
        </div>

        {prodFarms.length === 0 ? (
          <div className="card" style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)' }}>
            No production save folder detected.
          </div>
        ) : (
          <div className="card-grid">
            {prodFarms.map((farm) => (
              <div
                key={farm.folder_name}
                id={`farm-card-${farm.folder_name}`}
                className="card card-clickable"
                onClick={() => onInspectFarm(farm.folder_name)}
                style={{ borderColor: 'rgba(236, 201, 75, 0.3)' }}
              >
                <div className="card-header">
                  <div className="card-title">
                    <Sprout size={16} style={{ color: 'var(--accent-gold)' }} />
                    <span>{farm.farm_name} Çiftliği</span>
                  </div>
                  <span className="badge badge-gold">PRODUCTION</span>
                </div>

                <div className="card-body">
                  <div className="kv-list">
                    <div className="kv-item">
                      <span className="kv-key">Host Farmer</span>
                      <strong className="kv-val" style={{ color: 'var(--accent-gold)' }}>
                        {farm.host_name}
                      </strong>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">Farmhands</span>
                      <span className="kv-val">
                        {farm.farmhands.length > 0
                          ? farm.farmhands.join(', ')
                          : 'elbi'}
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">In-Game Date</span>
                      <span className="kv-val">{farm.date_summary}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">Current Wealth</span>
                      <span className="kv-val" style={{ color: 'var(--accent-gold)' }}>
                        {farm.money.toLocaleString()}g
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">Save Directory</span>
                      <span className="code-box" style={{ padding: '1px 6px' }}>{farm.folder_name}</span>
                    </div>
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '8px' }}>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      Last saved: {farm.last_modified}
                    </span>
                    <button className="btn btn-secondary btn-sm">
                      <span>Inspect</span>
                      <ArrowRight size={12} />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Section 2: Disposable Test Saves */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '14px' }}>
          <FlaskConical size={18} style={{ color: 'var(--status-info)' }} />
          <h2 style={{ fontSize: '16px', fontWeight: 700 }}>
            Disposable Test Saves (Isolated R&D Slots)
          </h2>
          <span className="badge badge-info">DISPOSABLE</span>
        </div>

        {testFarms.length === 0 ? (
          <div className="card" style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)' }}>
            No disposable test saves detected.
          </div>
        ) : (
          <div className="card-grid">
            {testFarms.map((farm) => (
              <div
                key={farm.folder_name}
                id={`farm-card-${farm.folder_name}`}
                className="card card-clickable"
                onClick={() => onInspectFarm(farm.folder_name)}
              >
                <div className="card-header">
                  <div className="card-title">
                    <FlaskConical size={16} style={{ color: 'var(--status-info)' }} />
                    <span>{farm.farm_name} Çiftliği</span>
                  </div>
                  <span className="badge badge-info">TEST SAVE</span>
                </div>

                <div className="card-body">
                  <div className="kv-list">
                    <div className="kv-item">
                      <span className="kv-key">Designated Host</span>
                      <strong className="kv-val">{farm.host_name}</strong>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">Farmhands</span>
                      <span className="kv-val">
                        {farm.farmhands.length > 0
                          ? farm.farmhands.join(', ')
                          : 'None'}
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">In-Game Date</span>
                      <span className="kv-val">{farm.date_summary}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">Funds</span>
                      <span className="kv-val">{farm.money.toLocaleString()}g</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">Folder Name</span>
                      <span className="code-box" style={{ padding: '1px 6px' }}>{farm.folder_name}</span>
                    </div>
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '8px' }}>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      Last saved: {farm.last_modified}
                    </span>
                    <button className="btn btn-secondary btn-sm">
                      <span>Inspect</span>
                      <ArrowRight size={12} />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
