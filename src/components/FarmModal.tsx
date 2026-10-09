import React from 'react';
import { FarmDetailedMetadata } from '../types';
import {
  X,
  User,
  Users,
  Home,
  Clock,
  Calendar,
  Hash,
  Database,
  Lock,
} from 'lucide-react';

interface FarmModalProps {
  metadata: FarmDetailedMetadata | null;
  loading: boolean;
  onClose: () => void;
}

export const FarmModal: React.FC<FarmModalProps> = ({
  metadata,
  loading,
  onClose,
}) => {
  if (!metadata && !loading) return null;

  return (
    <div
      className="modal-backdrop animate-fade-in"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
    >
      <div
        className="modal-dialog"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <span className="modal-title">
              {loading ? 'Inspecting Save...' : `${metadata?.farm_name} Farm Details`}
            </span>
            {metadata && (
              <span
                className={`badge ${
                  metadata.is_legacy_production
                    ? 'badge-gold'
                    : metadata.is_test_fixture
                    ? 'badge-info'
                    : 'badge-success'
                }`}
              >
                {metadata.is_legacy_production
                  ? 'PRIMARY BASELINE'
                  : metadata.is_test_fixture
                  ? 'TEST FIXTURE'
                  : 'PROTECTED SAVE'}
              </span>
            )}
          </div>
          <button
            className="btn btn-ghost btn-icon"
            onClick={onClose}
            aria-label="Close dialog"
          >
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          {loading ? (
            <div style={{ textAlign: 'center', padding: '40px 0' }}>
              <div className="pulse-dot pulse-dot-active" style={{ width: '16px', height: '16px' }} />
              <p style={{ marginTop: '12px', color: 'var(--text-secondary)' }}>
                Parsing Stardew save XML safely...
              </p>
            </div>
          ) : metadata ? (
            <>
              {/* Safety notice */}
              <div className="banner banner-info" style={{ marginBottom: '12px' }}>
                <Lock size={16} />
                <div>
                  <strong>Read-Only Inspection Mode:</strong> This view inspects
                  the save structure without altering bytes. Writing to live production
                  is strictly locked.
                </div>
              </div>

              {/* General Metadata */}
              <div className="card">
                <div className="card-header" style={{ marginBottom: '8px' }}>
                  <div className="card-title">
                    <Database size={15} />
                    <span>General Information</span>
                  </div>
                </div>
                <div className="kv-list">
                  <div className="kv-item">
                    <span className="kv-key">Save Folder</span>
                    <span className="code-box" style={{ padding: '2px 6px' }}>{metadata.folder_name}</span>
                  </div>
                  <div className="kv-item">
                    <span className="kv-key">Game ID (uniqueIDForThisGame)</span>
                    <span className="code-box" style={{ padding: '2px 6px' }}>{metadata.game_id}</span>
                  </div>
                  <div className="kv-item">
                    <span className="kv-key">In-Game Calendar</span>
                    <span className="kv-val" style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
                      <Calendar size={13} style={{ color: 'var(--accent-primary)' }} />
                      {metadata.in_game_date}
                    </span>
                  </div>
                  <div className="kv-item">
                    <span className="kv-key">Total Play Time</span>
                    <span className="kv-val" style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
                      <Clock size={13} />
                      {metadata.play_time_hours.toFixed(1)} hours
                    </span>
                  </div>
                  <div className="kv-item">
                    <span className="kv-key">Engine Version</span>
                    <span className="kv-val">{metadata.game_version}</span>
                  </div>
                </div>
              </div>

              {/* Host Player */}
              <div className="card">
                <div className="card-header" style={{ marginBottom: '8px' }}>
                  <div className="card-title">
                    <User size={15} style={{ color: 'var(--accent-gold)' }} />
                    <span>Root Host (Primary Farmer)</span>
                  </div>
                </div>
                {metadata.host ? (
                  <div className="kv-list">
                    <div className="kv-item">
                      <span className="kv-key">Host Name</span>
                      <strong className="kv-val" style={{ color: 'var(--accent-gold)' }}>
                        {metadata.host.name}
                      </strong>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">UniqueMultiplayerID</span>
                      <span className="code-box" style={{ padding: '2px 6px' }}>
                        {metadata.host.unique_multiplayer_id}
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">Home Residence</span>
                      <span className="kv-val">{metadata.host.home_location}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">House Upgrade Level</span>
                      <span className="kv-val">Level {metadata.host.house_upgrade_level}</span>
                    </div>
                  </div>
                ) : (
                  <p style={{ color: 'var(--text-muted)' }}>No host farmer found.</p>
                )}
              </div>

              {/* Farmhands */}
              <div className="card">
                <div className="card-header" style={{ marginBottom: '8px' }}>
                  <div className="card-title">
                    <Users size={15} style={{ color: 'var(--accent-primary)' }} />
                    <span>Farmhands ({metadata.farmhands.length})</span>
                  </div>
                </div>
                {metadata.farmhands.length > 0 ? (
                  <div className="kv-list">
                    {metadata.farmhands.map((fh, idx) => (
                      <div key={idx} className="kv-item">
                        <div>
                          <strong>{fh.name}</strong>
                          <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginLeft: '8px' }}>
                            ({fh.cabin_indoors_name || 'Assigned Cabin'})
                          </span>
                        </div>
                        <span className="code-box" style={{ padding: '2px 6px' }}>
                          ID: {fh.unique_multiplayer_id}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p style={{ color: 'var(--text-muted)', fontSize: '13px' }}>
                    No additional farmhands in this save.
                  </p>
                )}
              </div>

              {/* Cabins Summary */}
              {metadata.cabins.length > 0 && (
                <div className="card">
                  <div className="card-header" style={{ marginBottom: '8px' }}>
                    <div className="card-title">
                      <Home size={15} />
                      <span>Cabins ({metadata.cabins.length})</span>
                    </div>
                  </div>
                  <div className="kv-list">
                    {metadata.cabins.map((cabin, idx) => (
                      <div key={idx} className="kv-item">
                        <div>
                          <span>{cabin.building_type}</span>
                          <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginLeft: '6px' }}>
                            at ({cabin.tile_x}, {cabin.tile_y})
                          </span>
                        </div>
                        <span className="badge badge-info" style={{ fontSize: '10px' }}>
                          {cabin.indoors_name}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Cryptographic Verification Hashes */}
              <div className="card">
                <div className="card-header" style={{ marginBottom: '8px' }}>
                  <div className="card-title">
                    <Hash size={15} />
                    <span>Live Cryptographic SHA-256</span>
                  </div>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  <div>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      Primary Save File Hash:
                    </span>
                    <div className="code-box">
                      {metadata.sha256_primary || 'Calculating...'}
                    </div>
                  </div>
                  <div>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      SaveGameInfo Hash:
                    </span>
                    <div className="code-box">
                      {metadata.sha256_savegameinfo || 'Calculating...'}
                    </div>
                  </div>
                </div>
              </div>
            </>
          ) : null}
        </div>

        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>
            Close Inspector
          </button>
        </div>
      </div>
    </div>
  );
};
