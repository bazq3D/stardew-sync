import React, { useState } from 'react';
import { SnapshotInfo, SnapshotVerification } from '../types';
import { verifySnapshot } from '../api/tauri';
import {
  Archive,
  ShieldCheck,
  CheckCircle2,
  Clock,
  RefreshCw,
  Lock,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';

interface BackupsPageProps {
  snapshots: SnapshotInfo[];
  loading: boolean;
  onRefresh: () => void;
}

export const BackupsPage: React.FC<BackupsPageProps> = ({
  snapshots,
  loading,
  onRefresh,
}) => {
  const { t } = useTranslation();
  const [verifyingId, setVerifyingId] = useState<string | null>(null);
  const [verificationResults, setVerificationResults] = useState<
    Record<string, SnapshotVerification>
  >({});

  const handleVerify = async (snapshotId: string) => {
    setVerifyingId(snapshotId);
    try {
      const res = await verifySnapshot(snapshotId);
      setVerificationResults((prev) => ({ ...prev, [snapshotId]: res }));
    } catch (err) {
      console.error('Verification error:', err);
    } finally {
      setVerifyingId(null);
    }
  };

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Safety Notice */}
      <div className="banner banner-warning">
        <Lock size={20} />
        <div>
          <strong>{t('backups.safetyNoticeTitle')}</strong> {t('backups.safetyNoticeText')}
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h2 style={{ fontSize: '16px', fontWeight: 700 }}>
            {t('backups.title')}
          </h2>
          <p style={{ fontSize: '13px', color: 'var(--text-muted)' }}>
            {t('backups.subtitle')}
          </p>
        </div>

        <button
          className="btn btn-secondary btn-sm"
          onClick={onRefresh}
          disabled={loading}
        >
          <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
          <span>{t('backups.refresh')}</span>
        </button>
      </div>

      {snapshots.length === 0 ? (
        <div className="card" style={{ padding: '32px', textAlign: 'center', color: 'var(--text-muted)' }}>
          {t('backups.noSnapshots')}
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {snapshots.map((snap) => {
            const ver = verificationResults[snap.id];
            const isCurrentlyVerifying = verifyingId === snap.id;

            return (
              <div
                key={snap.id}
                id={`snapshot-card-${snap.id}`}
                className="card"
                style={{
                  borderLeft: '4px solid var(--accent-primary)',
                }}
              >
                <div className="card-header">
                  <div className="card-title">
                    <Archive size={17} style={{ color: 'var(--accent-primary)' }} />
                    <span>{snap.folder_name}</span>
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span className="badge badge-success">
                      <CheckCircle2 size={11} />
                      {t('backups.verified')}
                    </span>
                    <button
                      id={`btn-verify-${snap.id}`}
                      className="btn btn-secondary btn-sm"
                      onClick={() => handleVerify(snap.id)}
                      disabled={isCurrentlyVerifying}
                    >
                      <RefreshCw
                        size={12}
                        className={isCurrentlyVerifying ? 'animate-spin' : ''}
                      />
                      <span>
                        {isCurrentlyVerifying ? t('backups.verifying') : t('backups.verifyButton')}
                      </span>
                    </button>
                  </div>
                </div>

                <div className="card-body">
                  <div className="kv-list">
                    <div className="kv-item">
                      <span className="kv-key">{t('backups.timestamp')}</span>
                      <span className="kv-val" style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                        <Clock size={13} />
                        {snap.timestamp}
                      </span>
                    </div>

                    <div className="kv-item">
                      <span className="kv-key">{t('backups.diskLocation')}</span>
                      <span className="code-box" style={{ padding: '2px 6px' }}>{snap.path}</span>
                    </div>

                    <div className="kv-item">
                      <span className="kv-key">{t('backups.filesInSnapshot', { count: snap.files.length })}</span>
                      <span className="kv-val">
                        {snap.files.join(', ')}
                      </span>
                    </div>

                    <div className="kv-item">
                      <span className="kv-key">{t('backups.totalArchiveSize')}</span>
                      <span className="kv-val">
                        {(snap.total_bytes / (1024 * 1024)).toFixed(2)} MB
                      </span>
                    </div>
                  </div>

                  {/* Verification Results Panel */}
                  {ver && (
                    <div
                      className={`banner ${
                        ver.is_valid ? 'banner-success' : 'banner-warning'
                      }`}
                      style={{ marginTop: '12px', marginBottom: 0 }}
                    >
                      <ShieldCheck size={18} />
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                        <strong>{ver.message}</strong>
                        <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)' }}>
                          Primary: {ver.primary_hash || 'N/A'}<br />
                          SaveGameInfo: {ver.savegameinfo_hash || 'N/A'}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
