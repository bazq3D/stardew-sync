import React, { useState, useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';
import {
  PublicDeviceInfo,
  PairedDeviceWithStatus,
  DiscoveredPeer,
  ActivePairingSessionView,
} from '../types';
import {
  fetchLocalDeviceInfo,
  setLocalDeviceName,
  fetchPairedDevices,
  unpairDevice,
  startPairingInvitation,
  cancelPairingInvitation,
  fetchActivePairingSession,
  confirmPairingCode,
  rejectPairing,
  connectToPeer,
  fetchDiscoveredPeers,
  broadcastDiscoveryBeacon,
} from '../api/tauri';
import { useTranslation } from '../i18n/LanguageContext';
import { PairDeviceModal } from '../components/PairDeviceModal';
import { SasVerificationModal } from '../components/SasVerificationModal';
import { ErrorBoundary } from '../components/ErrorBoundary';
import {
  Laptop,
  Radio,
  RefreshCw,
  Plus,
  Trash2,
  XCircle,
  AlertTriangle,
  Info,
  Key,
  Copy,
  Check,
  Edit2,
  Save,
  X,
  Wifi,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';

export const DevicesPage: React.FC = () => {
  const { t } = useTranslation();

  // State
  const [localDevice, setLocalDevice] = useState<PublicDeviceInfo | null>(null);
  const [pairedDevices, setPairedDevices] = useState<PairedDeviceWithStatus[]>([]);
  const [discoveredPeers, setDiscoveredPeers] = useState<DiscoveredPeer[]>([]);
  const [activeSession, setActiveSession] = useState<ActivePairingSessionView | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [refreshingBeacon, setRefreshingBeacon] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  const formatErrorMessage = (err: any): string => {
    const msg = String(err?.message || err || '');
    if (msg.includes('failed to fill whole buffer') || msg.includes('UnexpectedEof')) {
      return 'The remote device closed the connection unexpectedly. Ensure the device is powered on, has an active invitation, and is on the same network.';
    }
    return msg;
  };

  // Local device editing
  const [editingName, setEditingName] = useState<boolean>(false);
  const [newNameInput, setNewNameInput] = useState<string>('');
  const [savingName, setSavingName] = useState<boolean>(false);
  const [copiedId, setCopiedId] = useState<boolean>(false);
  const [copiedFingerprint, setCopiedFingerprint] = useState<boolean>(false);
  const [showAdvancedIdentity, setShowAdvancedIdentity] = useState<boolean>(false);

  // Modals state
  const [showPairModal, setShowPairModal] = useState<boolean>(false);
  const [targetPeerAddress, setTargetPeerAddress] = useState<string>('');
  const [isConnectingPeer, setIsConnectingPeer] = useState<boolean>(false);

  // SAS confirmation state
  const [isConfirmingSas, setIsConfirmingSas] = useState<boolean>(false);
  const [isRejectingSas, setIsRejectingSas] = useState<boolean>(false);

  // Unpair modal state
  const [deviceToUnpair, setDeviceToUnpair] = useState<PairedDeviceWithStatus | null>(null);
  const [isUnpairing, setIsUnpairing] = useState<boolean>(false);

  // Expandable device details (fingerprint / crypto details)
  const [expandedPeerIds, setExpandedPeerIds] = useState<Set<string>>(new Set());
  const [expandedPairedIds, setExpandedPairedIds] = useState<Set<string>>(new Set());

  const togglePeerDetails = (id: string) => {
    setExpandedPeerIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const togglePairedDetails = (id: string) => {
    setExpandedPairedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const loadAll = useCallback(async () => {
    try {
      const [local, paired, peers, session] = await Promise.all([
        fetchLocalDeviceInfo(),
        fetchPairedDevices(),
        fetchDiscoveredPeers(),
        fetchActivePairingSession(),
      ]);
      setLocalDevice(local);
      setPairedDevices(paired);
      setDiscoveredPeers(peers);
      setActiveSession(session);
      if (!editingName && local) {
        setNewNameInput(local.device_name);
      }
    } catch (err: any) {
      console.error('Failed to load device data:', err);
      setError(formatErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [editingName]);

  useEffect(() => {
    loadAll();
    const interval = setInterval(() => {
      loadAll();
    }, 3000); // Poll session, pairing, and discovery every 3s
    return () => clearInterval(interval);
  }, [loadAll]);

  // Auto-close pair device modal only when active session enters confirmation (SAS) modal state
  useEffect(() => {
    if (activeSession?.status === 'AwaitingDualConfirmation') {
      setShowPairModal(false);
    }
  }, [activeSession?.status]);

  // Handle Refresh Beacon & Paired Devices
  const handleRefreshBeacon = async () => {
    setRefreshingBeacon(true);
    setError(null);
    try {
      await broadcastDiscoveryBeacon();
      // Allow 350ms for local network interfaces and socket state to settle
      await new Promise((resolve) => setTimeout(resolve, 350));
      await loadAll();
      setSuccessMsg(t('devices.refreshSuccess'));
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (err: any) {
      console.error('Failed to broadcast beacon:', err);
      setError(String(err));
    } finally {
      setRefreshingBeacon(false);
    }
  };

  // Name Editing
  const handleSaveDeviceName = async () => {
    const trimmed = newNameInput.trim();
    if (!trimmed || trimmed.length > 32) return;
    setSavingName(true);
    try {
      const updated = await setLocalDeviceName(trimmed);
      setLocalDevice(updated);
      setEditingName(false);
      setSuccessMsg(t('devices.saveName') + ' ✓');
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (err: any) {
      setError(String(err));
    } finally {
      setSavingName(false);
    }
  };

  const handleCopyText = (text: string, type: 'id' | 'fingerprint') => {
    navigator.clipboard.writeText(text);
    if (type === 'id') {
      setCopiedId(true);
      setTimeout(() => setCopiedId(false), 2000);
    } else {
      setCopiedFingerprint(true);
      setTimeout(() => setCopiedFingerprint(false), 2000);
    }
  };

  // Pairing Invitation Actions
  const handleStartInvitation = async () => {
    setError(null);
    setTargetPeerAddress('');
    try {
      await startPairingInvitation();
      setShowPairModal(true);
      await loadAll();
    } catch (err: any) {
      setError(String(err));
    }
  };

  const handleCancelInvitation = async () => {
    try {
      await cancelPairingInvitation();
      await loadAll();
    } catch (err: any) {
      setError(String(err));
    }
  };

  const handleConnectPeer = async (address: string, code: string) => {
    setIsConnectingPeer(true);
    setError(null);
    try {
      await connectToPeer(address, code);
      // Once connected, close pair modal; SAS modal will appear reactively
      setShowPairModal(false);
      await loadAll();
    } catch (err: any) {
      setError(formatErrorMessage(err));
    } finally {
      setIsConnectingPeer(false);
    }
  };

  // SAS Confirmation Actions
  const handleConfirmSas = async () => {
    if (!activeSession?.sas_code) return;
    setIsConfirmingSas(true);
    try {
      await confirmPairingCode(activeSession.sas_code);
      await loadAll();
    } catch (err: any) {
      setError(String(err));
    } finally {
      setIsConfirmingSas(false);
    }
  };

  const handleRejectSas = async () => {
    setIsRejectingSas(true);
    try {
      await rejectPairing();
      await loadAll();
    } catch (err: any) {
      setError(String(err));
    } finally {
      setIsRejectingSas(false);
    }
  };

  const handleDismissSasResult = async () => {
    try {
      await cancelPairingInvitation();
    } catch (err: any) {
      console.error('Failed to reset pairing session on dismiss:', err);
    }
    setActiveSession(null);
    await loadAll();
  };

  // Unpair Action with Authenticated Remote Revocation
  const handleExecuteUnpair = async () => {
    if (!deviceToUnpair) return;
    setIsUnpairing(true);
    try {
      const result = await unpairDevice(deviceToUnpair.device_id);
      setDeviceToUnpair(null);
      if (result.remote_notified) {
        setSuccessMsg(result.message);
        setTimeout(() => setSuccessMsg(null), 4000);
      } else {
        // Show clear warning if peer could not be reached
        setError(formatErrorMessage(result.message));
      }
      await loadAll();
    } catch (err: any) {
      setError(formatErrorMessage(err));
    } finally {
      setIsUnpairing(false);
    }
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'connected':
        return (
          <span className="badge badge-success">
            <span className="pulse-dot pulse-dot-active" style={{ width: '6px', height: '6px' }} />
            {t('devices.statusConnected')}
          </span>
        );
      case 'online':
        return (
          <span
            className="badge badge-success"
            style={{
              backgroundColor: 'rgba(34, 197, 94, 0.15)',
              borderColor: 'rgba(34, 197, 94, 0.4)',
              color: 'var(--status-success)',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
            }}
          >
            <Radio size={11} />
            {t('devices.statusOnline')}
          </span>
        );
      case 'connecting':
        return (
          <span className="badge badge-warning">
            <RefreshCw size={11} className="animate-spin" />
            {t('devices.statusConnecting')}
          </span>
        );
      case 'error':
        return (
          <span className="badge badge-danger">
            <XCircle size={12} />
            {t('devices.statusError')}
          </span>
        );
      case 'offline':
      case 'disconnected':
      default:
        return (
          <span className="badge badge-info" style={{ opacity: 0.7 }}>
            <span className="pulse-dot pulse-dot-inactive" style={{ width: '6px', height: '6px' }} />
            {t('devices.statusOffline')}
          </span>
        );
    }
  };

  return (
    <div className="page-container animate-fade-in" style={{ maxWidth: '1000px', margin: '0 auto' }}>
      {/* Page Header */}
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '24px' }}>
        <div>
          <h1 className="page-title">{t('devices.title')}</h1>
          <p className="page-subtitle" style={{ fontSize: '13px', color: 'var(--text-secondary)', marginTop: '4px' }}>
            {t('devices.subtitle')}
          </p>
        </div>
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            className="btn btn-secondary"
            onClick={handleRefreshBeacon}
            disabled={loading || refreshingBeacon}
            title={t('devices.refresh')}
          >
            <RefreshCw size={14} className={refreshingBeacon || loading ? 'animate-spin' : ''} />
            <span>{t('devices.refresh')}</span>
          </button>
          <button
            className="btn btn-secondary"
            style={{ borderColor: 'var(--accent-primary)', color: 'var(--accent-primary)' }}
            onClick={async () => {
              await handleStartInvitation();
            }}
            disabled={loading}
            title={t('devices.generateInvitationDirect')}
          >
            <Radio size={14} />
            <span>{t('devices.generateInvitationDirect')}</span>
          </button>
          <button
            className="btn btn-primary"
            onClick={() => {
              setTargetPeerAddress('');
              setShowPairModal(true);
            }}
          >
            <Plus size={15} />
            <span>{t('devices.pairNewDevice')}</span>
          </button>
        </div>
      </div>

      {/* Save Sync Notice */}
      <div
        className="card"
        style={{
          borderLeft: '4px solid var(--accent-gold)',
          backgroundColor: 'var(--accent-gold-subtle)',
          marginBottom: '20px',
          padding: '14px 18px',
        }}
      >
        <div style={{ display: 'flex', gap: '14px', alignItems: 'flex-start' }}>
          <AlertTriangle size={20} style={{ color: 'var(--accent-gold)', flexShrink: 0, marginTop: '2px' }} />
          <div>
            <h4 style={{ margin: '0 0 4px 0', fontSize: '13.5px', fontWeight: 600, color: 'var(--text-primary)' }}>
              {t('devices.saveSyncNoticeTitle')}
            </h4>
            <p style={{ margin: 0, fontSize: '12.5px', color: 'var(--text-secondary)', lineHeight: '1.45' }}>
              {t('devices.saveSyncNoticeDesc')}
            </p>
          </div>
        </div>
      </div>

      {/* Global Error or Success Feedback */}
      {error && (
        <div
          className="security-notice-box"
          style={{
            backgroundColor: 'var(--status-danger-bg)',
            border: '1px solid rgba(245, 101, 101, 0.3)',
            color: 'var(--text-primary)',
            marginBottom: '16px',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <XCircle size={16} style={{ color: 'var(--status-danger)' }} />
            <span style={{ fontSize: '13px' }}>{error}</span>
          </div>
          <button className="btn-icon" onClick={() => setError(null)} style={{ width: '22px', height: '22px' }}>
            <X size={13} />
          </button>
        </div>
      )}
      {successMsg && (
        <div
          className="security-notice-box success"
          style={{ marginBottom: '16px', alignItems: 'center' }}
        >
          <Check size={16} style={{ color: 'var(--status-success)' }} />
          <span style={{ fontSize: '13px' }}>{successMsg}</span>
        </div>
      )}

      {/* 1. This Device Card */}
      {localDevice && (
        <div className="card" style={{ marginBottom: '24px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <div
                style={{
                  width: '40px',
                  height: '40px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: 'var(--accent-primary-subtle)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'var(--accent-primary)',
                }}
              >
                <Laptop size={22} />
              </div>
              <div>
                <h3 style={{ margin: 0, fontSize: '15.5px', fontWeight: 600 }}>{t('devices.thisDevice')}</h3>
                <span style={{ fontSize: '11.5px', color: 'var(--text-muted)' }}>
                  {t('devices.createdAt')}: {new Date(localDevice.created_at).toLocaleDateString()}
                </span>
              </div>
            </div>

            {/* Device Name Editor */}
            <div>
              {editingName ? (
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <input
                    type="text"
                    className="form-input"
                    value={newNameInput}
                    onChange={(e) => setNewNameInput(e.target.value)}
                    maxLength={32}
                    style={{ padding: '6px 10px', fontSize: '13px', width: '200px' }}
                    autoFocus
                  />
                  <button
                    className="btn btn-primary btn-sm"
                    onClick={handleSaveDeviceName}
                    disabled={savingName || !newNameInput.trim()}
                  >
                    {savingName ? <RefreshCw size={12} className="animate-spin" /> : <Save size={13} />}
                  </button>
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={() => {
                      setEditingName(false);
                      setNewNameInput(localDevice.device_name);
                    }}
                  >
                    <X size={13} />
                  </button>
                </div>
              ) : (
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <span style={{ fontWeight: 600, fontSize: '14.5px', color: 'var(--text-primary)' }}>
                    {localDevice.device_name}
                  </span>
                  <button
                    className="btn-icon"
                    onClick={() => setEditingName(true)}
                    title={t('devices.editName')}
                    style={{ width: '28px', height: '28px' }}
                  >
                    <Edit2 size={13} />
                  </button>
                </div>
              )}
            </div>
          </div>

          {/* Identifiers Grid */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
              gap: '12px',
              padding: '12px 16px',
              borderRadius: 'var(--radius-md)',
              backgroundColor: 'var(--bg-card-subtle)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            {/* Fingerprint */}
            <div>
              <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', fontWeight: 600 }}>
                {t('devices.fingerprint')}
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '4px' }}>
                <span
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 700,
                    fontSize: '13.5px',
                    color: 'var(--accent-primary)',
                    letterSpacing: '0.05em',
                  }}
                >
                  {localDevice.fingerprint}
                </span>
                <span className="badge badge-success" style={{ fontSize: '10px' }}>
                  ED25519
                </span>
                <button
                  className="btn-icon"
                  onClick={() => handleCopyText(localDevice.fingerprint, 'fingerprint')}
                  title={t('devices.copyFingerprint')}
                  style={{ width: '24px', height: '24px' }}
                >
                  {copiedFingerprint ? <Check size={12} style={{ color: 'var(--status-success)' }} /> : <Copy size={12} />}
                </button>
              </div>
            </div>

            {/* Device ID */}
            <div>
              <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', fontWeight: 600 }}>
                {t('devices.deviceId')}
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '4px' }}>
                <span
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: '11.5px',
                    color: 'var(--text-secondary)',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    maxWidth: '220px',
                  }}
                  title={localDevice.device_id}
                >
                  {localDevice.device_id}
                </span>
                <button
                  className="btn-icon"
                  onClick={() => handleCopyText(localDevice.device_id, 'id')}
                  title="Copy full Device ID"
                  style={{ width: '24px', height: '24px' }}
                >
                  {copiedId ? <Check size={12} style={{ color: 'var(--status-success)' }} /> : <Copy size={12} />}
                </button>
              </div>
            </div>
          </div>

          {/* Expandable Advanced Cryptographic Details */}
          <div style={{ marginTop: '10px' }}>
            <button
              className="btn btn-ghost btn-sm"
              onClick={() => setShowAdvancedIdentity(!showAdvancedIdentity)}
              style={{ fontSize: '11.5px', padding: '4px 8px' }}
            >
              <span>{showAdvancedIdentity ? 'Hide Cryptographic Keys' : 'Show Public Key Details'}</span>
              {showAdvancedIdentity ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            </button>
            {showAdvancedIdentity && (
              <div
                style={{
                  marginTop: '8px',
                  padding: '10px 12px',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'var(--bg-input)',
                  border: '1px solid var(--border-subtle)',
                  fontSize: '11.5px',
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--text-secondary)',
                  wordBreak: 'break-all',
                }}
              >
                <div style={{ color: 'var(--text-muted)', marginBottom: '4px', textTransform: 'uppercase' }}>
                  Public Key (Hex):
                </div>
                <div>{localDevice.public_key_hex}</div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* 2. Paired Devices List */}
      <div className="card" style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <h3 style={{ margin: 0, fontSize: '15.5px', fontWeight: 600 }}>{t('devices.pairedDevices')}</h3>
            <span className="badge badge-info">{pairedDevices.length}</span>
          </div>
        </div>

        {pairedDevices.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '36px 16px', color: 'var(--text-muted)' }}>
            <Radio size={36} style={{ margin: '0 auto 12px auto', opacity: 0.3 }} />
            <p style={{ margin: '0 0 12px 0', fontSize: '13.5px' }}>{t('devices.noPairedDevices')}</p>
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => {
                setTargetPeerAddress('');
                setShowPairModal(true);
              }}
            >
              <Plus size={13} />
              <span>{t('devices.pairNewDevice')}</span>
            </button>
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            {pairedDevices.map((dev) => (
              <div
                key={dev.device_id}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '14px 16px',
                  borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--border-subtle)',
                  backgroundColor: 'var(--bg-card-subtle)',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
                  <div
                    style={{
                      width: '38px',
                      height: '38px',
                      borderRadius: 'var(--radius-sm)',
                      backgroundColor: 'var(--accent-primary-subtle)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: 'var(--accent-primary)',
                    }}
                  >
                    <Laptop size={20} />
                  </div>
                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                      <span style={{ fontWeight: 600, fontSize: '14px', color: 'var(--text-primary)' }}>
                        {dev.device_name}
                      </span>
                      {getStatusBadge(dev.connection_status)}
                    </div>
                    <div
                      style={{
                        display: 'flex',
                        flexWrap: 'wrap',
                        alignItems: 'center',
                        gap: '12px',
                        marginTop: '4px',
                        fontSize: '12px',
                        color: 'var(--text-muted)',
                      }}
                    >
                      {dev.last_ip && (
                        <span>
                          {t('devices.address')}: {dev.last_ip}
                        </span>
                      )}
                      <span>
                        {t('devices.lastSeen')}:{' '}
                        {dev.last_seen_at ? new Date(dev.last_seen_at).toLocaleTimeString() : t('devices.never')}
                      </span>
                      <button
                        type="button"
                        className="btn btn-ghost btn-sm"
                        onClick={() => togglePairedDetails(dev.device_id)}
                        style={{ fontSize: '11px', padding: '1px 6px', height: 'auto', minHeight: 'unset', color: 'var(--text-muted)' }}
                      >
                        <span>{expandedPairedIds.has(dev.device_id) ? t('devices.hideDetails') : t('devices.deviceDetails')}</span>
                        {expandedPairedIds.has(dev.device_id) ? <ChevronUp size={11} /> : <ChevronDown size={11} />}
                      </button>
                    </div>
                    {expandedPairedIds.has(dev.device_id) && (
                      <div
                        style={{
                          marginTop: '8px',
                          padding: '8px 12px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          fontSize: '11.5px',
                          fontFamily: 'var(--font-mono)',
                          color: 'var(--text-secondary)',
                        }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                          <Key size={12} style={{ color: 'var(--accent-primary)' }} />
                          <span>{t('devices.fingerprint')}: {dev.fingerprint}</span>
                        </div>
                        <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>
                          ID: {dev.device_id}
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                <div>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ color: 'var(--status-danger)', borderColor: 'rgba(245, 101, 101, 0.3)' }}
                    onClick={() => setDeviceToUnpair(dev)}
                  >
                    <Trash2 size={13} />
                    <span>{t('devices.unpair')}</span>
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 3. Nearby LAN Devices (Auto-Discovery) */}
      <div className="card" style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <h3 style={{ margin: 0, fontSize: '15.5px', fontWeight: 600 }}>{t('devices.nearbyDevices')}</h3>
              <span className="badge badge-info">{discoveredPeers.length}</span>
            </div>
            <span style={{ fontSize: '11.5px', color: 'var(--text-muted)' }}>
              UDP 42890 • Subnet Beacon
            </span>
          </div>
          <button
            className="btn btn-secondary btn-sm"
            onClick={() => {
              setTargetPeerAddress('');
              setShowPairModal(true);
            }}
          >
            <Plus size={13} />
            <span>{t('devices.connectManual')}</span>
          </button>
        </div>

        {discoveredPeers.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '24px 16px', color: 'var(--text-muted)' }}>
            <p style={{ margin: 0, fontSize: '13px' }}>{t('devices.noNearbyDevices')}</p>
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            {discoveredPeers.map((peer) => (
              <div
                key={peer.device_id}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '12px 16px',
                  borderRadius: 'var(--radius-md)',
                  border: '1px solid var(--border-subtle)',
                  backgroundColor: 'var(--bg-card-subtle)',
                }}
              >
                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span style={{ fontWeight: 600, fontSize: '13.5px' }}>{peer.device_name}</span>
                    {peer.is_paired ? (
                      <span className="badge badge-success" style={{ fontSize: '10px' }}>
                        {t('devices.statusConnected')}
                      </span>
                    ) : peer.is_inviting ? (
                      <span className="badge badge-success" style={{ fontSize: '10px' }}>
                        <span className="pulse-dot pulse-dot-active" style={{ width: '6px', height: '6px' }} />
                        {t('devices.readyToPair')}
                      </span>
                    ) : (
                      <span className="badge badge-info" style={{ fontSize: '10px', opacity: 0.8 }}>
                        {t('devices.lanDiscoveredBadge')}
                      </span>
                    )}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginTop: '4px', fontSize: '11.5px', color: 'var(--text-muted)' }}>
                    <span style={{ fontFamily: 'var(--font-mono)' }}>{peer.address}</span>
                    <span>•</span>
                    <span>{peer.last_seen_secs_ago}s ago</span>
                    <button
                      type="button"
                      className="btn btn-ghost btn-sm"
                      onClick={() => togglePeerDetails(peer.device_id)}
                      style={{ fontSize: '11px', padding: '1px 6px', height: 'auto', minHeight: 'unset', color: 'var(--text-muted)' }}
                    >
                      <span>{expandedPeerIds.has(peer.device_id) ? t('devices.hideDetails') : t('devices.deviceDetails')}</span>
                      {expandedPeerIds.has(peer.device_id) ? <ChevronUp size={11} /> : <ChevronDown size={11} />}
                    </button>
                  </div>
                  {expandedPeerIds.has(peer.device_id) && (
                    <div
                      style={{
                        marginTop: '8px',
                        padding: '8px 12px',
                        borderRadius: 'var(--radius-sm)',
                        backgroundColor: 'var(--bg-input)',
                        border: '1px solid var(--border-subtle)',
                        fontSize: '11.5px',
                        fontFamily: 'var(--font-mono)',
                        color: 'var(--text-secondary)',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <Key size={12} style={{ color: 'var(--accent-primary)' }} />
                        <span>{t('devices.fingerprint')}: {peer.fingerprint}</span>
                      </div>
                    </div>
                  )}
                </div>

                {!peer.is_paired && (
                  <button
                    className="btn btn-primary btn-sm"
                    onClick={() => {
                      setTargetPeerAddress(peer.address);
                      setShowPairModal(true);
                    }}
                  >
                    {peer.is_inviting ? (
                      <>
                        <Radio size={13} />
                        <span>{t('devices.pairNowBtn')}</span>
                      </>
                    ) : (
                      <>
                        <Wifi size={13} />
                        <span>{t('devices.connect')}</span>
                      </>
                    )}
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 4. Troubleshooting & Firewall Advisory */}
      <div
        className="card"
        style={{
          backgroundColor: 'var(--bg-card-subtle)',
          border: '1px solid var(--border-subtle)',
          padding: '16px 20px',
        }}
      >
        <div style={{ display: 'flex', gap: '14px', alignItems: 'flex-start' }}>
          <Info size={18} style={{ color: 'var(--accent-primary)', flexShrink: 0, marginTop: '2px' }} />
          <div>
            <h4 style={{ margin: '0 0 4px 0', fontSize: '13px', fontWeight: 600 }}>{t('devices.firewallNotice')}</h4>
            <p style={{ margin: '0 0 6px 0', fontSize: '12px', color: 'var(--text-secondary)', lineHeight: '1.45' }}>
              {t('devices.firewallNoticeDesc')}
            </p>
            <span style={{ fontSize: '11.5px', color: 'var(--text-muted)' }}>
              🔒 {t('devices.wanUnsupported')}
            </span>
          </div>
        </div>
      </div>

      {/* PAIR NEW DEVICE MODAL */}
      <PairDeviceModal
        isOpen={showPairModal}
        onClose={() => setShowPairModal(false)}
        activeSession={activeSession}
        discoveredPeers={discoveredPeers}
        initialPeerAddress={targetPeerAddress}
        onStartInvitation={handleStartInvitation}
        onCancelInvitation={handleCancelInvitation}
        onConnectPeer={handleConnectPeer}
        isConnecting={isConnectingPeer}
        error={error}
        onClearError={() => setError(null)}
      />

      {/* ACTIVE DUAL-CONFIRMATION (SAS) MODAL */}
      <ErrorBoundary fallbackTitle="SAS Verification Display Error">
        <SasVerificationModal
          session={activeSession}
          onConfirm={handleConfirmSas}
          onReject={handleRejectSas}
          onDismiss={handleDismissSasResult}
          isConfirming={isConfirmingSas}
          isRejecting={isRejectingSas}
        />
      </ErrorBoundary>

      {/* UNPAIR CONFIRMATION MODAL */}
      {deviceToUnpair && typeof document !== 'undefined' && createPortal(
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
          onClick={() => setDeviceToUnpair(null)}
          role="dialog"
          aria-modal="true"
        >
          <div
            className="modal-dialog modal-dialog-sm"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <h3 className="modal-title" style={{ color: 'var(--status-danger)' }}>
                {t('devices.unpairConfirmTitle')}
              </h3>
              <button className="btn-icon" onClick={() => setDeviceToUnpair(null)}>
                <X size={15} />
              </button>
            </div>
            <div className="modal-body">
              <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-secondary)', lineHeight: '1.5' }}>
                {t('devices.unpairConfirmDesc', { name: deviceToUnpair.device_name })}
              </p>
            </div>
            <div className="modal-footer">
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => setDeviceToUnpair(null)}
                disabled={isUnpairing}
              >
                {t('devices.cancel')}
              </button>
              <button
                type="button"
                className="btn btn-primary"
                style={{ backgroundColor: 'var(--status-danger)', borderColor: 'var(--status-danger)' }}
                onClick={handleExecuteUnpair}
                disabled={isUnpairing}
              >
                {isUnpairing ? <RefreshCw size={14} className="animate-spin" /> : <Trash2 size={14} />}
                <span>{t('devices.unpair')}</span>
              </button>
            </div>
          </div>
        </div>,
        document.body
      )}
    </div>
  );
};
