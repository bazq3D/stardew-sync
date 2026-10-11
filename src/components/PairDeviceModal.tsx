import React, { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { DiscoveredPeer, ActivePairingSessionView } from '../types';
import { useTranslation } from '../i18n/LanguageContext';
import {
  Radio,
  Wifi,
  X,
  Copy,
  Check,
  RefreshCw,
  Clock,
  ShieldCheck,
  Laptop,
  AlertCircle,
  ChevronDown,
  ChevronUp,
  AlertTriangle,
} from 'lucide-react';

interface PairDeviceModalProps {
  isOpen: boolean;
  onClose: () => void;
  activeSession: ActivePairingSessionView | null;
  discoveredPeers: DiscoveredPeer[];
  initialPeerAddress?: string;
  onStartInvitation: () => Promise<void>;
  onCancelInvitation: () => Promise<void>;
  onConnectPeer: (address: string, code: string) => Promise<void>;
  isConnecting: boolean;
  error: string | null;
  onClearError: () => void;
}

export const PairDeviceModal: React.FC<PairDeviceModalProps> = ({
  isOpen,
  onClose,
  activeSession,
  discoveredPeers,
  initialPeerAddress = '',
  onStartInvitation,
  onCancelInvitation,
  onConnectPeer,
  isConnecting,
  error,
  onClearError,
}) => {
  const { t } = useTranslation();

  const isInviting = activeSession?.status === 'Inviting' && Boolean(activeSession.invitation_code);
  const isExpired = activeSession?.status === 'Expired';

  const CODE_PREFIX = 'SD-';
  const CODE_MAX_LENGTH = 7; // 'SD-' + 4 characters

  const [peerAddress, setPeerAddress] = useState<string>(initialPeerAddress);
  const [selectedPeer, setSelectedPeer] = useState<DiscoveredPeer | null>(null);
  const [showAdvancedManual, setShowAdvancedManual] = useState<boolean>(false);
  const [invitationCode, setInvitationCode] = useState<string>(CODE_PREFIX);
  const [copiedCode, setCopiedCode] = useState<boolean>(false);
  const [startingInvitation, setStartingInvitation] = useState<boolean>(false);

  // Sync initial peer address and auto-resolve peer without changing view
  useEffect(() => {
    if (initialPeerAddress) {
      const found = discoveredPeers.find((p) => p.address === initialPeerAddress);
      if (found) {
        setSelectedPeer(found);
        setPeerAddress(found.address);
      } else {
        setSelectedPeer({
          device_id: '',
          device_name: initialPeerAddress,
          fingerprint: '',
          address: initialPeerAddress,
          is_inviting: false,
          is_paired: false,
          last_seen_secs_ago: 0,
        });
        setPeerAddress(initialPeerAddress);
      }
    } else if (discoveredPeers.length === 1 && !selectedPeer) {
      setSelectedPeer(discoveredPeers[0]);
      setPeerAddress(discoveredPeers[0].address);
    }
  }, [initialPeerAddress, discoveredPeers]);

  // Ensure SD- prefix is populated
  useEffect(() => {
    if (!invitationCode || !invitationCode.startsWith(CODE_PREFIX)) {
      setInvitationCode(CODE_PREFIX);
    }
  }, [invitationCode]);

  // Handle escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const handleCopyCode = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCode(true);
    setTimeout(() => setCopiedCode(false), 2000);
  };

  const handleStartInvitationClick = async () => {
    onClearError();
    setStartingInvitation(true);
    try {
      await onStartInvitation();
    } finally {
      setStartingInvitation(false);
    }
  };

  const handleCodeFocus = (e: React.FocusEvent<HTMLInputElement>) => {
    if (e.target.value === CODE_PREFIX) {
      e.target.setSelectionRange(CODE_PREFIX.length, CODE_PREFIX.length);
    }
  };

  const handleCodeKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    const input = e.currentTarget;
    const start = input.selectionStart ?? 0;
    const end = input.selectionEnd ?? 0;
    const val = input.value;

    // SPACE key handling
    if (e.key === ' ' || e.code === 'Space') {
      e.preventDefault();
      if (start <= CODE_PREFIX.length && end <= CODE_PREFIX.length) {
        if (!val.startsWith(CODE_PREFIX)) {
          const suffix = val.replace(/^SD-?/i, '').replace(/[^A-Z0-9]/g, '').slice(0, 4);
          setInvitationCode(CODE_PREFIX + suffix);
        }
        requestAnimationFrame(() => {
          input.setSelectionRange(CODE_PREFIX.length, CODE_PREFIX.length);
        });
      }
      return;
    }

    // BACKSPACE key handling
    if (e.key === 'Backspace') {
      if (start === CODE_PREFIX.length && end === CODE_PREFIX.length) {
        e.preventDefault();
        return;
      }
      if (start < CODE_PREFIX.length && end <= CODE_PREFIX.length) {
        e.preventDefault();
        input.setSelectionRange(CODE_PREFIX.length, CODE_PREFIX.length);
        return;
      }
      if (start < CODE_PREFIX.length && end > CODE_PREFIX.length) {
        e.preventDefault();
        const suffixRemaining = val.slice(end);
        setInvitationCode(CODE_PREFIX + suffixRemaining);
        requestAnimationFrame(() => {
          input.setSelectionRange(CODE_PREFIX.length, CODE_PREFIX.length);
        });
        return;
      }
    }

    // DELETE key handling
    if (e.key === 'Delete') {
      if (start < CODE_PREFIX.length && end < CODE_PREFIX.length) {
        e.preventDefault();
        input.setSelectionRange(CODE_PREFIX.length, CODE_PREFIX.length);
        return;
      }
    }
  };

  const handleCodeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let raw = e.target.value.toUpperCase();

    if (!raw.startsWith(CODE_PREFIX)) {
      const suffix = raw.replace(/^SD-?/i, '').replace(/[^A-Z0-9]/g, '').slice(0, 4);
      setInvitationCode(CODE_PREFIX + suffix);
    } else {
      const suffix = raw.slice(CODE_PREFIX.length).replace(/[^A-Z0-9]/g, '').slice(0, 4);
      setInvitationCode(CODE_PREFIX + suffix);
    }
  };

  const handleCodePaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    e.preventDefault();
    const pasted = e.clipboardData.getData('text').trim().toUpperCase();

    const cleanSuffix = pasted
      .replace(/^(SD[- ]?)+/i, '')
      .replace(/[^A-Z0-9]/g, '')
      .slice(0, 4);

    const formatted = CODE_PREFIX + cleanSuffix;
    setInvitationCode(formatted);
    requestAnimationFrame(() => {
      if (e.currentTarget) {
        e.currentTarget.setSelectionRange(formatted.length, formatted.length);
      }
    });
  };

  const handleConnectSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    let addr = peerAddress.trim();
    if (!addr) return;
    if (!addr.includes(':')) {
      addr = `${addr}:42890`;
    }

    const code = invitationCode.trim().toUpperCase();
    if (!code || code === CODE_PREFIX || code.length < CODE_MAX_LENGTH) return;

    onClearError();
    await onConnectPeer(addr, code);
  };

  const handleSelectDiscoveredPeer = (peer: DiscoveredPeer) => {
    setSelectedPeer(peer);
    setPeerAddress(peer.address);
    onClearError();
  };

  const formatSeconds = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  };

  const isConnectCodeValid = invitationCode.length === CODE_MAX_LENGTH;
  const isAddressValid = peerAddress.trim().length > 0;

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
        zIndex: 9998,
      }}
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-labelledby="pair-modal-title"
    >
      <div
        className="modal-dialog modal-dialog-xl"
        onClick={(e) => e.stopPropagation()}
        style={{
          maxHeight: 'calc(100vh - 40px)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}
      >
        {/* Modal Header */}
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <div
              style={{
                width: '36px',
                height: '36px',
                borderRadius: '10px',
                backgroundColor: 'var(--accent-primary-subtle)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--accent-primary)',
              }}
            >
              <Radio size={20} />
            </div>
            <div>
              <h3 id="pair-modal-title" className="modal-title" style={{ margin: 0, fontSize: '16px' }}>
                {t('devices.pairNewDevice')}
              </h3>
              <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                {t('devices.subtitle')}
              </span>
            </div>
          </div>
          <button
            className="btn-icon"
            onClick={onClose}
            aria-label={t('devices.cancel')}
            style={{ width: '32px', height: '32px' }}
          >
            <X size={18} />
          </button>
        </div>

        {/* Modal Body: Unified Two-Section Grid */}
        <div className="modal-body" style={{ flex: 1, overflowY: 'auto', padding: '20px' }}>
          {/* Top Error Banner if present */}
          {error && (
            <div
              className="security-notice-box"
              style={{
                backgroundColor: 'var(--status-danger-bg)',
                border: '1px solid rgba(245, 101, 101, 0.3)',
                color: 'var(--text-primary)',
                marginBottom: '18px',
                alignItems: 'flex-start',
              }}
            >
              <AlertCircle size={16} style={{ color: 'var(--status-danger)', flexShrink: 0, marginTop: '2px' }} />
              <div style={{ flex: 1, fontSize: '12.5px' }}>{error}</div>
              <button
                className="btn-icon"
                onClick={onClearError}
                style={{ width: '20px', height: '20px', padding: 0 }}
              >
                <X size={12} />
              </button>
            </div>
          )}

          <div className="unified-pairing-grid">
            {/* ======================================================== */}
            {/* SECTION A: YOUR INVITATION                               */}
            {/* ======================================================== */}
            <div className="pairing-panel" style={{ position: 'relative' }}>
              <div className="pairing-panel-header">
                <div
                  style={{
                    width: '30px',
                    height: '30px',
                    borderRadius: '8px',
                    backgroundColor: 'var(--accent-primary-subtle)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'var(--accent-primary)',
                    flexShrink: 0,
                  }}
                >
                  <Radio size={16} />
                </div>
                <div style={{ flex: 1 }}>
                  <h4 className="pairing-panel-title">{t('devices.yourInvitation')}</h4>
                  <div className="pairing-panel-subtitle">{t('devices.yourInvitationDesc')}</div>
                </div>
              </div>

              {/* State A1: Active Invitation */}
              {isInviting && activeSession?.invitation_code ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', flex: 1, justifyContent: 'space-between' }}>
                  {/* Status & Countdown Banner */}
                  <div
                    className="security-notice-box success"
                    style={{ alignItems: 'center', justifyContent: 'space-between', padding: '10px 14px' }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <span className="pulse-dot pulse-dot-active" />
                      <span style={{ fontWeight: 600, fontSize: '13px' }}>
                        {t('devices.invitationActive')}
                      </span>
                    </div>
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '6px',
                        fontSize: '12px',
                        color: 'var(--text-secondary)',
                        fontFamily: 'var(--font-mono)',
                        fontWeight: 600,
                      }}
                    >
                      <Clock size={13} />
                      <span>{formatSeconds(activeSession.seconds_remaining)}</span>
                    </div>
                  </div>

                  {/* Prominent Code Card */}
                  <div
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      padding: '22px 16px',
                      borderRadius: 'var(--radius-md)',
                      backgroundColor: 'var(--bg-card)',
                      border: '2px solid var(--border-medium)',
                      gap: '12px',
                    }}
                  >
                    <span
                      style={{
                        fontSize: '11px',
                        color: 'var(--text-muted)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.08em',
                        fontWeight: 700,
                      }}
                    >
                      {t('devices.invitationCode')}
                    </span>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                      <span
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: '32px',
                          fontWeight: 800,
                          letterSpacing: '0.15em',
                          color: 'var(--accent-primary)',
                        }}
                      >
                        {activeSession.invitation_code}
                      </span>
                      <button
                        type="button"
                        className="btn btn-secondary btn-sm"
                        onClick={() => handleCopyCode(activeSession.invitation_code!)}
                        title={t('devices.codeCopied')}
                      >
                        {copiedCode ? <Check size={14} style={{ color: 'var(--status-success)' }} /> : <Copy size={14} />}
                        <span>{copiedCode ? t('devices.copied') : t('devices.copyFingerprint')}</span>
                      </button>
                    </div>
                    <span
                      style={{
                        fontSize: '11.5px',
                        color: 'var(--text-secondary)',
                        textAlign: 'center',
                        maxWidth: '280px',
                        lineHeight: '1.4',
                      }}
                    >
                      {t('devices.invitationHelp')}
                    </span>
                  </div>

                  {/* Actions Bar */}
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      gap: '10px',
                      marginTop: 'auto',
                      paddingTop: '8px',
                    }}
                  >
                    <button
                      type="button"
                      className="btn btn-secondary btn-sm"
                      onClick={handleStartInvitationClick}
                      disabled={startingInvitation}
                      title="Generate a fresh invitation code"
                    >
                      <RefreshCw size={13} className={startingInvitation ? 'animate-spin' : ''} />
                      <span>{t('devices.regenerateInvitation')}</span>
                    </button>
                    <button
                      type="button"
                      className="btn btn-ghost btn-sm"
                      onClick={onCancelInvitation}
                      style={{ color: 'var(--status-danger)' }}
                    >
                      {t('devices.cancelInvitation')}
                    </button>
                  </div>
                </div>
              ) : isExpired ? (
                /* State A2: Expired Invitation */
                <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', flex: 1, justifyContent: 'space-between' }}>
                  <div
                    className="security-notice-box"
                    style={{
                      backgroundColor: 'var(--status-warning-bg, rgba(237, 137, 54, 0.12))',
                      border: '1px solid rgba(237, 137, 54, 0.3)',
                      color: 'var(--text-primary)',
                      padding: '12px 14px',
                      alignItems: 'flex-start',
                    }}
                  >
                    <AlertTriangle size={18} style={{ color: 'var(--accent-gold, #ed8936)', flexShrink: 0, marginTop: '2px' }} />
                    <div style={{ flex: 1 }}>
                      <div style={{ fontWeight: 600, fontSize: '13px', marginBottom: '2px' }}>
                        {t('devices.invitationExpired')}
                      </div>
                      <div style={{ fontSize: '12px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>
                        {t('devices.invitationExpiredDesc')}
                      </div>
                    </div>
                  </div>

                  {/* Dimmed Expired Card */}
                  <div
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      padding: '20px 16px',
                      borderRadius: 'var(--radius-md)',
                      backgroundColor: 'var(--bg-card)',
                      border: '1px dashed var(--border-medium)',
                      opacity: 0.65,
                      gap: '8px',
                    }}
                  >
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', fontWeight: 600 }}>
                      {t('devices.invitationCode')}
                    </span>
                    <span
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: '26px',
                        fontWeight: 700,
                        letterSpacing: '0.12em',
                        color: 'var(--text-muted)',
                        textDecoration: 'line-through',
                      }}
                    >
                      {activeSession?.invitation_code || 'SD-XXXX'}
                    </span>
                    <span style={{ fontSize: '11px', color: 'var(--status-danger)', fontWeight: 600 }}>
                      0:00 (Expired)
                    </span>
                  </div>

                  {/* Regenerate Action */}
                  <div style={{ marginTop: 'auto', paddingTop: '8px' }}>
                    <button
                      type="button"
                      className="btn btn-primary"
                      onClick={handleStartInvitationClick}
                      disabled={startingInvitation}
                      style={{ width: '100%', justifyContent: 'center', padding: '9px 16px' }}
                    >
                      <RefreshCw size={14} className={startingInvitation ? 'animate-spin' : ''} />
                      <span>{t('devices.regenerateInvitation')}</span>
                    </button>
                  </div>
                </div>
              ) : (
                /* State A3: Idle / Generate Invitation */
                <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', flex: 1, justifyContent: 'center', textAlign: 'center', padding: '12px 0' }}>
                  <div
                    style={{
                      width: '46px',
                      height: '46px',
                      borderRadius: '50%',
                      backgroundColor: 'var(--accent-primary-subtle)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: 'var(--accent-primary)',
                      margin: '0 auto',
                    }}
                  >
                    <Radio size={22} />
                  </div>
                  <div>
                    <h5 style={{ margin: '0 0 6px 0', fontSize: '14.5px', fontWeight: 600 }}>
                      {t('devices.generateInvitation')}
                    </h5>
                    <p style={{ margin: 0, fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: '1.45', maxWidth: '300px', marginInline: 'auto' }}>
                      {t('devices.noInvitationActiveDesc')}
                    </p>
                  </div>

                  <div
                    className="security-notice-box info"
                    style={{ textAlign: 'left', marginTop: '4px', fontSize: '11.5px', padding: '8px 12px' }}
                  >
                    <ShieldCheck size={16} style={{ color: 'var(--status-info)', flexShrink: 0, marginTop: '2px' }} />
                    <span style={{ color: 'var(--text-secondary)' }}>
                      {t('devices.invitationHelp')}
                    </span>
                  </div>

                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={handleStartInvitationClick}
                    disabled={startingInvitation}
                    style={{ padding: '9px 20px', margin: '4px auto 0 auto', minWidth: '220px', justifyContent: 'center' }}
                  >
                    {startingInvitation ? (
                      <>
                        <RefreshCw size={14} className="animate-spin" />
                        <span>{t('devices.statusConnecting')}</span>
                      </>
                    ) : (
                      <>
                        <Radio size={14} />
                        <span>{t('devices.generateInvitation')}</span>
                      </>
                    )}
                  </button>
                </div>
              )}
            </div>

            {/* ======================================================== */}
            {/* SECTION B: CONNECT TO ANOTHER DEVICE                     */}
            {/* ======================================================== */}
            <div className="pairing-panel">
              <div className="pairing-panel-header">
                <div
                  style={{
                    width: '30px',
                    height: '30px',
                    borderRadius: '8px',
                    backgroundColor: 'var(--accent-gold-subtle)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'var(--accent-gold)',
                    flexShrink: 0,
                  }}
                >
                  <Wifi size={16} />
                </div>
                <div style={{ flex: 1 }}>
                  <h4 className="pairing-panel-title">{t('devices.connectToDevice')}</h4>
                  <div className="pairing-panel-subtitle">{t('devices.connectToDeviceDesc')}</div>
                </div>
              </div>

              <form onSubmit={handleConnectSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '14px', flex: 1 }}>
                {/* 1. LAN Peer Selection */}
                <div>
                  <label
                    style={{
                      display: 'block',
                      fontSize: '12px',
                      fontWeight: 600,
                      color: 'var(--text-secondary)',
                      marginBottom: '6px',
                    }}
                  >
                    {t('devices.nearbyDevices')} ({discoveredPeers.length})
                  </label>

                  {discoveredPeers.length > 0 ? (
                    <div
                      style={{
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '6px',
                        maxHeight: '140px',
                        overflowY: 'auto',
                        paddingRight: '2px',
                      }}
                    >
                      {discoveredPeers.map((peer) => {
                        const isSelected = selectedPeer?.address === peer.address;
                        return (
                          <div
                            key={peer.address}
                            onClick={() => handleSelectDiscoveredPeer(peer)}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'space-between',
                              padding: '8px 12px',
                              borderRadius: 'var(--radius-sm)',
                              backgroundColor: isSelected ? 'var(--accent-primary-subtle)' : 'var(--bg-card)',
                              border: isSelected
                                ? '1.5px solid var(--accent-primary)'
                                : '1px solid var(--border-subtle)',
                              cursor: 'pointer',
                              transition: 'all var(--transition-fast)',
                            }}
                          >
                            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', minWidth: 0 }}>
                              <Laptop size={14} style={{ color: isSelected ? 'var(--accent-primary)' : 'var(--text-muted)', flexShrink: 0 }} />
                              <div style={{ minWidth: 0 }}>
                                <div style={{ fontSize: '12.5px', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                                  {peer.device_name}
                                </div>
                                <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)' }}>
                                  {peer.address}
                                </div>
                              </div>
                            </div>

                            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexShrink: 0 }}>
                              {peer.is_inviting && (
                                <span className="badge badge-success" style={{ fontSize: '10.5px', padding: '2px 6px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                                  <Radio size={10} />
                                  <span>{t('devices.readyToPair')}</span>
                                </span>
                              )}
                              {isSelected && (
                                <span className="badge badge-info" style={{ fontSize: '10px', padding: '2px 5px' }}>
                                  {t('devices.selectedPeerBadge')}
                                </span>
                              )}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  ) : (
                    <div
                      style={{
                        padding: '12px 14px',
                        borderRadius: 'var(--radius-sm)',
                        backgroundColor: 'var(--bg-input)',
                        border: '1px dashed var(--border-subtle)',
                        fontSize: '11.5px',
                        color: 'var(--text-muted)',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '8px',
                      }}
                    >
                      <Wifi size={14} style={{ opacity: 0.5, flexShrink: 0 }} />
                      <span>{t('devices.noNearbyDevices')}</span>
                    </div>
                  )}
                </div>

                {/* 2. Invitation Code Input */}
                <div>
                  <label
                    htmlFor="connect-invitation-code"
                    style={{
                      display: 'block',
                      fontSize: '12px',
                      fontWeight: 600,
                      color: 'var(--text-secondary)',
                      marginBottom: '6px',
                    }}
                  >
                    {t('devices.invitationCode')}
                  </label>
                  <div style={{ position: 'relative' }}>
                    <input
                      id="connect-invitation-code"
                      type="text"
                      className="form-control"
                      value={invitationCode}
                      onChange={handleCodeChange}
                      onKeyDown={handleCodeKeyDown}
                      onFocus={handleCodeFocus}
                      onPaste={handleCodePaste}
                      maxLength={CODE_MAX_LENGTH}
                      placeholder="SD-XXXX"
                      autoComplete="off"
                      spellCheck="false"
                      style={{
                        fontFamily: 'var(--font-mono)',
                        fontSize: '18px',
                        fontWeight: 700,
                        letterSpacing: '0.12em',
                        padding: '10px 14px',
                        textTransform: 'uppercase',
                        borderRadius: 'var(--radius-sm)',
                        border: isConnectCodeValid
                          ? '1.5px solid var(--accent-primary)'
                          : '1px solid var(--border-medium)',
                        backgroundColor: 'var(--bg-input)',
                        color: 'var(--text-primary)',
                        width: '100%',
                      }}
                    />
                  </div>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>
                    {t('devices.invitationCodePlaceholder')} (SD- + 4 characters)
                  </div>
                </div>

                {/* 3. Advanced Manual IP/Port Accordion */}
                <div style={{ marginTop: 'auto', paddingTop: '4px' }}>
                  <button
                    type="button"
                    className="btn btn-ghost btn-sm"
                    onClick={() => setShowAdvancedManual(!showAdvancedManual)}
                    style={{ fontSize: '11.5px', padding: '2px 4px', color: 'var(--text-muted)' }}
                  >
                    <span>{t('devices.advancedManual')}</span>
                    {showAdvancedManual ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                  </button>

                  {showAdvancedManual && (
                    <div style={{ marginTop: '8px' }}>
                      <label
                        htmlFor="manual-peer-address"
                        style={{
                          display: 'block',
                          fontSize: '11px',
                          fontWeight: 600,
                          color: 'var(--text-muted)',
                          marginBottom: '4px',
                        }}
                      >
                        {t('devices.peerAddress')}
                      </label>
                      <input
                        id="manual-peer-address"
                        type="text"
                        className="form-control"
                        value={peerAddress}
                        onChange={(e) => {
                          setPeerAddress(e.target.value);
                          setSelectedPeer(null);
                        }}
                        placeholder="192.168.1.50:42890"
                        style={{
                          fontFamily: 'var(--font-mono)',
                          fontSize: '12px',
                          padding: '7px 10px',
                        }}
                      />
                    </div>
                  )}
                </div>

                {/* 4. Connect Button */}
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={isConnecting || !isConnectCodeValid || !isAddressValid}
                  style={{
                    width: '100%',
                    justifyContent: 'center',
                    padding: '10px 16px',
                    fontSize: '13.5px',
                    marginTop: '6px',
                  }}
                >
                  {isConnecting ? (
                    <>
                      <RefreshCw size={15} className="animate-spin" />
                      <span>{t('devices.connectingToPeer')}</span>
                    </>
                  ) : (
                    <>
                      <Wifi size={15} />
                      <span>{t('devices.connect')}</span>
                    </>
                  )}
                </button>
              </form>
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  return typeof document !== 'undefined' ? createPortal(modalContent, document.body) : null;
};
