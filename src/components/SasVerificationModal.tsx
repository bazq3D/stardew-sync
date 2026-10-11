import React, { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { ActivePairingSessionView } from '../types';
import { useTranslation } from '../i18n/LanguageContext';
import {
  Shield,
  ShieldCheck,
  ShieldAlert,
  Clock,
  Check,
  X,
  Laptop,
  AlertTriangle,
  RefreshCw,
  Key,
} from 'lucide-react';

interface SasVerificationModalProps {
  session: ActivePairingSessionView | null;
  onConfirm: () => Promise<void>;
  onReject: () => Promise<void>;
  onDismiss: () => void;
  isConfirming: boolean;
  isRejecting: boolean;
}

export const SasVerificationModal: React.FC<SasVerificationModalProps> = ({
  session,
  onConfirm,
  onReject,
  onDismiss,
  isConfirming,
  isRejecting,
}) => {
  const { t } = useTranslation();

  const [safetyVerified, setSafetyVerified] = useState<boolean>(false);

  const isCompleted = session?.status === 'Completed';
  const isRejected = session?.status === 'Rejected';
  const isExpired = session?.status === 'Expired';
  const isAwaiting = session?.status === 'AwaitingDualConfirmation';

  // Only render modal if session is active or in a terminal result state
  const shouldRender = Boolean(session && (isAwaiting || isCompleted || isRejected || isExpired));

  // Reset safety check when a new session starts (called unconditionally)
  useEffect(() => {
    setSafetyVerified(false);
  }, [session?.session_id]);

  // Keyboard navigation: Escape key dismisses terminal results cleanly (called unconditionally)
  useEffect(() => {
    if (!shouldRender) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && (isCompleted || isRejected || isExpired)) {
        onDismiss();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [shouldRender, isCompleted, isRejected, isExpired, onDismiss]);

  if (!shouldRender || !session) return null;

  // Parse SAS code into two 3-digit groups defensively (e.g. "481 920" or "481920" -> ["481", "920"])
  const sasRaw = (session.sas_code || '000000').trim();
  const parts = sasRaw.includes(' ')
    ? sasRaw.split(' ')
    : [sasRaw.slice(0, 3), sasRaw.slice(3, 6)];
  const partA = parts[0] || '000';
  const partB = parts[1] || '000';

  const peerName = session.peer_device?.device_name || t('devices.peerDeviceStatus');
  const peerFingerprint = session.peer_device?.fingerprint || '----';

  const formatSeconds = (secs: number | null | undefined) => {
    const s = typeof secs === 'number' && !isNaN(secs) ? Math.max(0, secs) : 0;
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${sec < 10 ? '0' : ''}${sec}`;
  };

  const modalContent = (
    <div
      className="modal-backdrop animate-fade-in"
      style={{
        position: 'fixed',
        inset: 0,
        width: '100vw',
        height: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 9999,
      }}
      role="dialog"
      aria-modal="true"
      aria-labelledby="sas-modal-title"
    >
      <div
        className="modal-dialog modal-dialog-sm"
        style={{
          maxHeight: 'calc(100vh - 48px)',
          display: 'flex',
          flexDirection: 'column',
          textAlign: 'center',
        }}
      >
        {/* Modal Header */}
        <div className="modal-header" style={{ justifyContent: 'center', position: 'relative' }}>
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '4px' }}>
            <div
              style={{
                width: '42px',
                height: '42px',
                borderRadius: '50%',
                backgroundColor: isCompleted
                  ? 'var(--status-success-bg)'
                  : isRejected || isExpired
                  ? 'var(--status-danger-bg)'
                  : 'var(--accent-primary-subtle)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: isCompleted
                  ? 'var(--status-success)'
                  : isRejected || isExpired
                  ? 'var(--status-danger)'
                  : 'var(--accent-primary)',
                marginBottom: '2px',
              }}
            >
              {isCompleted ? (
                <ShieldCheck size={24} />
              ) : isRejected || isExpired ? (
                <ShieldAlert size={24} />
              ) : (
                <Shield size={24} />
              )}
            </div>
            <h3 id="sas-modal-title" className="modal-title" style={{ fontSize: '16px', margin: 0 }}>
              {isCompleted
                ? t('devices.pairingCompleted')
                : isRejected
                ? t('devices.pairingRejected')
                : isExpired
                ? t('devices.pairingExpired')
                : t('devices.verifySasTitle')}
            </h3>
            <span style={{ fontSize: '11.5px', color: 'var(--text-muted)' }}>
              {isCompleted
                ? t('devices.statusConfirmed')
                : t('devices.verifySasSubtitle')}
            </span>
          </div>

          {(isCompleted || isRejected || isExpired) && (
            <button
              className="btn-icon"
              onClick={onDismiss}
              aria-label={t('devices.done')}
              style={{ position: 'absolute', right: '16px', top: '16px', width: '28px', height: '28px' }}
            >
              <X size={15} />
            </button>
          )}
        </div>

        {/* Modal Body */}
        <div className="modal-body" style={{ flex: 1, overflowY: 'auto', gap: '14px' }}>
          {/* Peer Context Card */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '10px 14px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-card-subtle)',
              border: '1px solid var(--border-subtle)',
              textAlign: 'left',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Laptop size={18} style={{ color: 'var(--accent-primary)', flexShrink: 0 }} />
              <div>
                <div style={{ fontWeight: 600, fontSize: '13px' }}>{peerName}</div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                  <Key size={10} />
                  <span>{peerFingerprint}</span>
                </div>
              </div>
            </div>
            <span className="badge badge-success" style={{ fontSize: '10px' }}>
              X25519 ECDH
            </span>
          </div>

          {/* STATE A: AWAITING DUAL CONFIRMATION */}
          {isAwaiting && (
            <>
              {/* SAS Code Big Hero Display */}
              <div className="sas-hero">
                <span
                  style={{
                    fontSize: '11.5px',
                    color: 'var(--text-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.08em',
                    fontWeight: 600,
                  }}
                >
                  6-Digit Short Authentication String
                </span>

                <div className="sas-code-grid">
                  <div className="sas-group-card">{partA}</div>
                  <span className="sas-group-divider">•</span>
                  <div className="sas-group-card">{partB}</div>
                </div>

                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    fontSize: '11.5px',
                    color: 'var(--text-secondary)',
                    fontFamily: 'var(--font-mono)',
                  }}
                >
                  <Clock size={12} />
                  <span>{t('devices.expiresIn')}: {formatSeconds(session.seconds_remaining)}</span>
                </div>
              </div>

              {/* Dual-Confirmation Tracker */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', textAlign: 'left' }}>
                <span style={{ fontSize: '11.5px', fontWeight: 600, color: 'var(--text-muted)' }}>
                  {t('devices.dualConfirmNotice')}
                </span>
                <div className="dual-tracker">
                  {/* Our Confirmation */}
                  <div className={`dual-tracker-card ${session.our_confirmed ? 'confirmed' : ''}`}>
                    <span style={{ fontWeight: 600, color: 'var(--text-secondary)' }}>
                      {t('devices.thisDeviceStatus')}
                    </span>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
                      {session.our_confirmed ? (
                        <>
                          <Check size={13} style={{ color: 'var(--status-success)' }} />
                          <span style={{ color: 'var(--status-success)', fontWeight: 600 }}>
                            {t('devices.statusConfirmed')}
                          </span>
                        </>
                      ) : (
                        <>
                          <span className="pulse-dot pulse-dot-active" style={{ width: '6px', height: '6px' }} />
                          <span style={{ color: 'var(--status-warning)' }}>
                            {t('devices.statusPending')}
                          </span>
                        </>
                      )}
                    </div>
                  </div>

                  {/* Peer Confirmation */}
                  <div className={`dual-tracker-card ${session.peer_confirmed ? 'confirmed' : ''}`}>
                    <span style={{ fontWeight: 600, color: 'var(--text-secondary)' }}>
                      {peerName}
                    </span>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
                      {session.peer_confirmed ? (
                        <>
                          <Check size={13} style={{ color: 'var(--status-success)' }} />
                          <span style={{ color: 'var(--status-success)', fontWeight: 600 }}>
                            {t('devices.statusConfirmed')}
                          </span>
                        </>
                      ) : (
                        <>
                          <span className="pulse-dot pulse-dot-inactive" style={{ width: '6px', height: '6px' }} />
                          <span style={{ color: 'var(--text-muted)' }}>
                            {t('devices.statusPending')}
                          </span>
                        </>
                      )}
                    </div>
                  </div>
                </div>
              </div>

              {/* Status Message based on who confirmed */}
              {session.our_confirmed && !session.peer_confirmed ? (
                <div className="security-notice-box info" style={{ alignItems: 'center' }}>
                  <RefreshCw size={16} className="animate-spin" style={{ color: 'var(--status-info)', flexShrink: 0 }} />
                  <span style={{ fontSize: '12.5px', textAlign: 'left' }}>
                    {t('devices.waitingForPeer', { name: peerName })}
                  </span>
                </div>
              ) : !session.our_confirmed && session.peer_confirmed ? (
                <div className="security-notice-box warning" style={{ alignItems: 'flex-start' }}>
                  <AlertTriangle size={16} style={{ color: 'var(--status-warning)', flexShrink: 0, marginTop: '2px' }} />
                  <span style={{ fontSize: '12.5px', textAlign: 'left' }}>
                    {t('devices.peerAlreadyConfirmed', { name: peerName })}
                  </span>
                </div>
              ) : (
                <p style={{ margin: 0, fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: '1.45', textAlign: 'left' }}>
                  {t('devices.verifySasDesc')}
                </p>
              )}

              {/* Safety Checkbox: only shown if we haven't confirmed yet */}
              {!session.our_confirmed && (
                <label
                  style={{
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: '10px',
                    padding: '10px 12px',
                    borderRadius: 'var(--radius-sm)',
                    backgroundColor: 'var(--bg-input)',
                    border: '1px solid var(--border-medium)',
                    cursor: 'pointer',
                    textAlign: 'left',
                    fontSize: '12px',
                    color: 'var(--text-primary)',
                    userSelect: 'none',
                  }}
                >
                  <input
                    type="checkbox"
                    checked={safetyVerified}
                    onChange={(e) => setSafetyVerified(e.target.checked)}
                    style={{ marginTop: '2px', accentColor: 'var(--accent-primary)', width: '15px', height: '15px' }}
                  />
                  <span>{t('devices.safetyCheckbox')}</span>
                </label>
              )}
            </>
          )}

          {/* STATE B: COMPLETED */}
          {isCompleted && (
            <div className="security-notice-box success" style={{ flexDirection: 'column', alignItems: 'center', padding: '16px', gap: '8px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--status-success)', fontWeight: 700, fontSize: '14px' }}>
                <Check size={18} />
                <span>{t('devices.pairingCompleted')}</span>
              </div>
              <p style={{ margin: 0, fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>
                {t('devices.pairingCompletedDesc', { name: peerName })}
              </p>
            </div>
          )}

          {/* STATE C: REJECTED */}
          {isRejected && (
            <div className="security-notice-box" style={{ backgroundColor: 'var(--status-danger-bg)', border: '1px solid rgba(245, 101, 101, 0.3)', flexDirection: 'column', alignItems: 'center', padding: '16px', gap: '8px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--status-danger)', fontWeight: 700, fontSize: '14px' }}>
                <X size={18} />
                <span>{t('devices.pairingRejected')}</span>
              </div>
              <p style={{ margin: 0, fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>
                {t('devices.pairingRejectedDesc')}
              </p>
            </div>
          )}

          {/* STATE D: EXPIRED */}
          {isExpired && (
            <div className="security-notice-box warning" style={{ flexDirection: 'column', alignItems: 'center', padding: '16px', gap: '8px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--status-warning)', fontWeight: 700, fontSize: '14px' }}>
                <Clock size={18} />
                <span>{t('devices.pairingExpired')}</span>
              </div>
              <p style={{ margin: 0, fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>
                {t('devices.pairingExpiredDesc')}
              </p>
            </div>
          )}
        </div>

        {/* Modal Footer */}
        <div className="modal-footer" style={{ justifyContent: 'space-between' }}>
          {isAwaiting ? (
            <>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={onReject}
                disabled={isRejecting}
                style={{ color: 'var(--status-danger)', borderColor: 'rgba(245, 101, 101, 0.3)' }}
              >
                {isRejecting ? <RefreshCw size={14} className="animate-spin" /> : <X size={14} />}
                <span>{t('devices.reject')}</span>
              </button>

              {!session.our_confirmed ? (
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={onConfirm}
                  disabled={!safetyVerified || isConfirming}
                  style={{ minWidth: '130px' }}
                >
                  {isConfirming ? (
                    <>
                      <RefreshCw size={14} className="animate-spin" />
                      <span>{t('devices.statusConnecting')}</span>
                    </>
                  ) : (
                    <>
                      <Check size={14} />
                      <span>{t('devices.confirm')}</span>
                    </>
                  )}
                </button>
              ) : (
                <button
                  type="button"
                  className="btn btn-secondary"
                  disabled
                  style={{ opacity: 0.8 }}
                >
                  <RefreshCw size={14} className="animate-spin" />
                  <span>{t('devices.statusConfirmed')}</span>
                </button>
              )}
            </>
          ) : (
            <div style={{ width: '100%', display: 'flex', justifyContent: 'flex-end' }}>
              <button type="button" className="btn btn-primary" onClick={onDismiss}>
                {t('devices.done')}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );

  return typeof document !== 'undefined'
    ? createPortal(modalContent, document.body)
    : modalContent;
};
