import React, { useState, useEffect } from 'react';
import {
  FarmInfo,
  SnapshotInfo,
  VersionedSnapshotSummary,
  SnapshotIntegrityResult,
  SnapshotComparison,
} from '../types';
import {
  fetchVersionedSnapshots,
  createManualSnapshot,
  verifyVersionedSnapshot,
  renameVersionedSnapshotLabel,
  deleteVersionedSnapshot,
  compareVersionedSnapshots,
} from '../api/tauri';
import {
  Archive,
  ShieldCheck,
  CheckCircle2,
  Clock,
  RefreshCw,
  Lock,
  Plus,
  Trash2,
  Edit3,
  Scale,
  AlertTriangle,
  Tag,
  Calendar,
  Coins,
  Users,
  Check,
  X,
} from 'lucide-react';
import { useTranslation } from '../i18n/LanguageContext';

interface BackupsPageProps {
  farms?: FarmInfo[];
  snapshots?: SnapshotInfo[];
  loading?: boolean;
  onRefresh?: () => void;
}

export const BackupsPage: React.FC<BackupsPageProps> = ({
  farms = [],
  loading: initialLoading = false,
  onRefresh,
}) => {
  const { t } = useTranslation();

  const [versionedSnapshots, setVersionedSnapshots] = useState<VersionedSnapshotSummary[]>([]);
  const [loading, setLoading] = useState<boolean>(initialLoading);
  const [selectedFarmFilter, setSelectedFarmFilter] = useState<string>('all');
  const [error, setError] = useState<string | null>(null);

  // Verification state
  const [verifyingId, setVerifyingId] = useState<string | null>(null);
  const [verificationResults, setVerificationResults] = useState<
    Record<string, SnapshotIntegrityResult>
  >({});

  // Create Snapshot Modal state
  const [isCreateModalOpen, setIsCreateModalOpen] = useState<boolean>(false);
  const [createFarmFolder, setCreateFarmFolder] = useState<string>('');
  const [createLabel, setCreateLabel] = useState<string>('');
  const [creating, setCreating] = useState<boolean>(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // Rename Modal state
  const [renamingSnapshot, setRenamingSnapshot] = useState<VersionedSnapshotSummary | null>(null);
  const [newLabelText, setNewLabelText] = useState<string>('');
  const [renaming, setRenaming] = useState<boolean>(false);

  // Delete Modal state
  const [deletingSnapshot, setDeletingSnapshot] = useState<VersionedSnapshotSummary | null>(null);
  const [deleteConfirmInput, setDeleteConfirmInput] = useState<string>('');
  const [deleting, setDeleting] = useState<boolean>(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  // Compare state
  const [selectedForCompare, setSelectedForCompare] = useState<string[]>([]);
  const [comparisonResult, setComparisonResult] = useState<SnapshotComparison | null>(null);
  const [comparing, setComparing] = useState<boolean>(false);

  const loadSnapshots = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchVersionedSnapshots(
        selectedFarmFilter === 'all' ? undefined : selectedFarmFilter
      );
      setVersionedSnapshots(data);
    } catch (err: any) {
      console.error('Failed to load versioned snapshots:', err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadSnapshots();
  }, [selectedFarmFilter]);

  // Set default farm folder when opening create modal
  useEffect(() => {
    if (isCreateModalOpen && farms.length > 0 && !createFarmFolder) {
      if (selectedFarmFilter !== 'all') {
        setCreateFarmFolder(selectedFarmFilter);
      } else {
        setCreateFarmFolder(farms[0].folder_name);
      }
    }
  }, [isCreateModalOpen, farms, selectedFarmFilter]);

  const handleVerify = async (snapshotId: string) => {
    setVerifyingId(snapshotId);
    try {
      const res = await verifyVersionedSnapshot(snapshotId);
      setVerificationResults((prev) => ({ ...prev, [snapshotId]: res }));
    } catch (err: any) {
      console.error('Verification error:', err);
    } finally {
      setVerifyingId(null);
    }
  };

  const handleCreateSnapshot = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!createFarmFolder) return;

    setCreating(true);
    setCreateError(null);
    try {
      await createManualSnapshot(createFarmFolder, createLabel.trim() || undefined);
      setIsCreateModalOpen(false);
      setCreateLabel('');
      await loadSnapshots();
      if (onRefresh) onRefresh();
    } catch (err: any) {
      console.error('Snapshot creation failed:', err);
      setCreateError(String(err));
    } finally {
      setCreating(false);
    }
  };

  const handleOpenRename = (snap: VersionedSnapshotSummary) => {
    setRenamingSnapshot(snap);
    setNewLabelText(snap.user_label || '');
  };

  const handleSaveRename = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!renamingSnapshot) return;

    setRenaming(true);
    try {
      const updated = await renameVersionedSnapshotLabel(
        renamingSnapshot.snapshot_id,
        newLabelText.trim()
      );
      setVersionedSnapshots((prev) =>
        prev.map((s) => (s.snapshot_id === updated.snapshot_id ? updated : s))
      );
      setRenamingSnapshot(null);
    } catch (err: any) {
      console.error('Rename failed:', err);
    } finally {
      setRenaming(false);
    }
  };

  const handleOpenDelete = (snap: VersionedSnapshotSummary) => {
    setDeletingSnapshot(snap);
    setDeleteConfirmInput('');
    setDeleteError(null);
  };

  const handleConfirmDelete = async () => {
    if (!deletingSnapshot || deleteConfirmInput !== 'DELETE') return;

    setDeleting(true);
    setDeleteError(null);
    try {
      await deleteVersionedSnapshot(deletingSnapshot.snapshot_id, 'DELETE');
      setDeletingSnapshot(null);
      await loadSnapshots();
      if (onRefresh) onRefresh();
    } catch (err: any) {
      console.error('Delete failed:', err);
      setDeleteError(String(err));
    } finally {
      setDeleting(false);
    }
  };

  const handleToggleCompare = (id: string) => {
    if (selectedForCompare.includes(id)) {
      setSelectedForCompare((prev) => prev.filter((item) => item !== id));
    } else {
      if (selectedForCompare.length >= 2) {
        setSelectedForCompare([selectedForCompare[1], id]);
      } else {
        setSelectedForCompare((prev) => [...prev, id]);
      }
    }
  };

  const handleExecuteCompare = async () => {
    if (selectedForCompare.length !== 2) return;

    setComparing(true);
    try {
      const result = await compareVersionedSnapshots(
        selectedForCompare[0],
        selectedForCompare[1]
      );
      setComparisonResult(result);
    } catch (err: any) {
      console.error('Compare failed:', err);
    } finally {
      setComparing(false);
    }
  };

  const formatReasonBadge = (reason: string) => {
    switch (reason.toUpperCase()) {
      case 'MANUAL':
        return <span className="badge badge-gold">{t('backups.reasonManual')}</span>;
      case 'ORIGINAL_IMPORT':
        return <span className="badge badge-purple">{t('backups.reasonOriginalImport')}</span>;
      case 'PRE_SYNC':
        return <span className="badge badge-info">{t('backups.reasonPreSync')}</span>;
      case 'POST_SYNC':
        return <span className="badge badge-success">{t('backups.reasonPostSync')}</span>;
      case 'PRE_HOST_ROTATION':
        return <span className="badge badge-warning">{t('backups.reasonPreHostRotation')}</span>;
      case 'RECOVERY':
        return <span className="badge badge-danger">{t('backups.reasonRecovery')}</span>;
      default:
        return <span className="badge badge-secondary">{reason}</span>;
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

      {/* Control & Filter Bar */}
      <div
        className="card"
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexWrap: 'wrap',
          gap: '16px',
          padding: '16px 20px',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div
            style={{
              width: '36px',
              height: '36px',
              borderRadius: '8px',
              backgroundColor: 'var(--accent-primary-subtle)',
              color: 'var(--accent-primary)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <Archive size={20} />
          </div>
          <div>
            <h2 style={{ fontSize: '16px', fontWeight: 700, margin: 0, color: 'var(--text-primary)' }}>
              {t('backups.title')}
            </h2>
            <p style={{ fontSize: '12.5px', color: 'var(--text-secondary)', margin: 0 }}>
              {t('backups.subtitle')}
            </p>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flexWrap: 'wrap' }}>
          {/* Farm Filter Dropdown */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '12.5px', color: 'var(--text-muted)' }}>
              {t('backups.filterFarm')}:
            </span>
            <select
              id="snapshot-farm-filter"
              value={selectedFarmFilter}
              onChange={(e) => setSelectedFarmFilter(e.target.value)}
              style={{
                backgroundColor: 'var(--bg-input)',
                color: 'var(--text-primary)',
                border: '1px solid var(--border-medium)',
                borderRadius: 'var(--radius-sm)',
                padding: '6px 12px',
                fontSize: '13px',
                outline: 'none',
              }}
            >
              <option value="all">{t('backups.allFarms')}</option>
              {farms.map((f) => (
                <option key={f.folder_name} value={f.folder_name}>
                  {f.farm_name} ({f.folder_name})
                </option>
              ))}
            </select>
          </div>

          {/* Compare Button */}
          {selectedForCompare.length === 2 && (
            <button
              id="btn-run-compare"
              className="btn btn-secondary btn-sm"
              onClick={handleExecuteCompare}
              disabled={comparing}
            >
              <Scale size={13} />
              <span>{t('backups.compareButton')} (2)</span>
            </button>
          )}

          {/* Create Snapshot Button */}
          <button
            id="btn-create-snapshot"
            className="btn btn-primary btn-sm"
            onClick={() => setIsCreateModalOpen(true)}
          >
            <Plus size={14} />
            <span>{t('backups.createSnapshot')}</span>
          </button>

          {/* Refresh Button */}
          <button
            id="btn-refresh-snapshots"
            className="btn btn-secondary btn-sm"
            onClick={() => {
              loadSnapshots();
              if (onRefresh) onRefresh();
            }}
            disabled={loading}
          >
            <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
            <span>{t('backups.refresh')}</span>
          </button>
        </div>
      </div>

      {/* Error state */}
      {error && (
        <div className="banner banner-danger">
          <AlertTriangle size={18} />
          <div>{error}</div>
        </div>
      )}

      {/* Snapshot List */}
      {loading && versionedSnapshots.length === 0 ? (
        <div className="card" style={{ padding: '36px', textAlign: 'center', color: 'var(--text-muted)' }}>
          <RefreshCw size={24} className="animate-spin" style={{ margin: '0 auto 12px' }} />
          <div>{t('common.loading')}</div>
        </div>
      ) : versionedSnapshots.length === 0 ? (
        <div className="card" style={{ padding: '36px', textAlign: 'center', color: 'var(--text-muted)' }}>
          <Archive size={32} style={{ margin: '0 auto 12px', opacity: 0.5 }} />
          <div>{t('backups.noSnapshots')}</div>
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {versionedSnapshots.map((snap) => {
            const ver = verificationResults[snap.snapshot_id];
            const isCurrentlyVerifying = verifyingId === snap.snapshot_id;
            const isSelectedCompare = selectedForCompare.includes(snap.snapshot_id);

            return (
              <div
                key={snap.snapshot_id}
                id={`snapshot-card-${snap.snapshot_id}`}
                className="card"
                style={{
                  borderLeft: isSelectedCompare
                    ? '4px solid var(--accent-gold)'
                    : '4px solid var(--accent-primary)',
                  transition: 'all 0.15s ease',
                }}
              >
                {/* Header bar of card */}
                <div className="card-header" style={{ flexWrap: 'wrap', gap: '10px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexWrap: 'wrap' }}>
                    <Archive size={18} style={{ color: 'var(--accent-primary)' }} />
                    <span style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)' }}>
                      {snap.farm_name}
                    </span>
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                      ({snap.farm_folder_name})
                    </span>

                    {/* User label if present */}
                    {snap.user_label && (
                      <span
                        className="badge badge-gold"
                        style={{ display: 'flex', alignItems: 'center', gap: '4px' }}
                      >
                        <Tag size={10} />
                        <span>{snap.user_label}</span>
                      </span>
                    )}

                    {/* Snapshot Reason Badge */}
                    {formatReasonBadge(snap.snapshot_reason)}

                    {/* Protected badge */}
                    {snap.is_protected && (
                      <span className="badge badge-success">
                        <CheckCircle2 size={10} />
                        <span>{t('backups.protected')}</span>
                      </span>
                    )}
                  </div>

                  {/* Top right actions */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap' }}>
                    <button
                      id={`btn-toggle-compare-${snap.snapshot_id}`}
                      className={`btn btn-sm ${isSelectedCompare ? 'btn-gold' : 'btn-secondary'}`}
                      onClick={() => handleToggleCompare(snap.snapshot_id)}
                      title={isSelectedCompare ? 'Deselect from comparison' : 'Select for comparison'}
                    >
                      <Scale size={12} />
                      <span>{isSelectedCompare ? 'Selected' : t('backups.compareButton')}</span>
                    </button>

                    <button
                      id={`btn-verify-${snap.snapshot_id}`}
                      className="btn btn-secondary btn-sm"
                      onClick={() => handleVerify(snap.snapshot_id)}
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

                    <button
                      id={`btn-rename-${snap.snapshot_id}`}
                      className="btn btn-secondary btn-sm"
                      onClick={() => handleOpenRename(snap)}
                      title={t('backups.renameButton')}
                    >
                      <Edit3 size={12} />
                      <span>{t('backups.renameButton')}</span>
                    </button>

                    <button
                      id={`btn-delete-${snap.snapshot_id}`}
                      className="btn btn-danger btn-sm"
                      onClick={() => handleOpenDelete(snap)}
                      title={t('backups.deleteButton')}
                    >
                      <Trash2 size={12} />
                    </button>

                    {/* Disabled Restore Button satisfying Phase 3 Safety Boundary */}
                    <button
                      className="btn btn-secondary btn-sm"
                      disabled
                      style={{ opacity: 0.5, cursor: 'not-allowed' }}
                      title={t('backups.restoreDisabledTooltip')}
                    >
                      <Lock size={12} />
                      <span>{t('backups.restoreButton')}</span>
                    </button>
                  </div>
                </div>

                {/* Card Body */}
                <div className="card-body">
                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
                      gap: '12px',
                    }}
                  >
                    <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' }}>
                        <Clock size={11} /> {t('backups.timestamp')}
                      </span>
                      <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
                        {snap.created_at.slice(0, 19).replace('T', ' ')} UTC
                      </span>
                    </div>

                    <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' }}>
                        <Calendar size={11} /> {t('backups.inGameDate')}
                      </span>
                      <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
                        {snap.save_metadata?.in_game_date || 'N/A'}
                      </span>
                    </div>

                    <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' }}>
                        <Coins size={11} color="var(--accent-gold)" /> {t('characters.currentFunds')}
                      </span>
                      <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--accent-gold)' }}>
                        {snap.save_metadata?.money !== null && snap.save_metadata?.money !== undefined
                          ? `${snap.save_metadata.money.toLocaleString()}g`
                          : 'N/A'}
                      </span>
                    </div>

                    <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' }}>
                        <Users size={11} /> {t('characters.title')}
                      </span>
                      <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
                        {snap.save_metadata
                          ? t('backups.charactersCount', { count: snap.save_metadata.character_count })
                          : 'N/A'}
                      </span>
                    </div>

                    <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' }}>
                        <Archive size={11} /> {t('backups.size')}
                      </span>
                      <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
                        {(snap.total_size_bytes / (1024 * 1024)).toFixed(2)} MB ({snap.file_count} files)
                      </span>
                    </div>

                    <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase', display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' }}>
                        <ShieldCheck size={11} /> {t('backups.packageHash')}
                      </span>
                      <span
                        className="code-box"
                        style={{ fontSize: '11px', padding: '1px 4px', display: 'block', textOverflow: 'ellipsis', overflow: 'hidden', whiteSpace: 'nowrap' }}
                        title={snap.package_hash}
                      >
                        {snap.package_hash}
                      </span>
                    </div>
                  </div>

                  {/* Snapshot ID bar */}
                  <div style={{ marginTop: '10px', fontSize: '11px', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '6px' }}>
                    <span>Snapshot ID:</span>
                    <span className="code-box" style={{ padding: '1px 6px' }}>
                      {snap.snapshot_id}
                    </span>
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
                          Files Checked: {ver.files_checked} • Package Hash:{' '}
                          {ver.package_hash_matched ? 'VALID' : 'TAMPERED'} • XML Schema:{' '}
                          {ver.xml_parseable ? 'VALID' : 'CORRUPTED'}
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

      {/* CREATE SNAPSHOT MODAL */}
      {isCreateModalOpen && (
        <div className="modal-backdrop">
          <div className="modal card animate-fade-in" style={{ maxWidth: '500px', width: '100%' }}>
            <div className="modal-header">
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Archive size={18} style={{ color: 'var(--accent-primary)' }} />
                <h3 style={{ fontSize: '16px', fontWeight: 700, margin: 0 }}>
                  {t('backups.createSnapshotModalTitle')}
                </h3>
              </div>
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setIsCreateModalOpen(false)}
                style={{ padding: '4px 8px' }}
              >
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handleCreateSnapshot}>
              <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <p style={{ fontSize: '13px', color: 'var(--text-secondary)', margin: 0 }}>
                  {t('backups.createSnapshotDesc')}
                </p>

                {createError && (
                  <div className="banner banner-danger" style={{ margin: 0 }}>
                    <AlertTriangle size={16} />
                    <span style={{ fontSize: '12.5px' }}>{createError}</span>
                  </div>
                )}

                <div>
                  <label
                    htmlFor="create-snap-farm"
                    style={{ display: 'block', fontSize: '12px', fontWeight: 600, marginBottom: '6px', color: 'var(--text-primary)' }}
                  >
                    {t('backups.selectFarm')}
                  </label>
                  <select
                    id="create-snap-farm"
                    value={createFarmFolder}
                    onChange={(e) => setCreateFarmFolder(e.target.value)}
                    required
                    style={{
                      width: '100%',
                      backgroundColor: 'var(--bg-input)',
                      color: 'var(--text-primary)',
                      border: '1px solid var(--border-medium)',
                      borderRadius: 'var(--radius-sm)',
                      padding: '8px 12px',
                      fontSize: '13px',
                      outline: 'none',
                    }}
                  >
                    {farms.map((f) => (
                      <option key={f.folder_name} value={f.folder_name}>
                        {f.farm_name} ({f.folder_name})
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label
                    htmlFor="create-snap-label"
                    style={{ display: 'block', fontSize: '12px', fontWeight: 600, marginBottom: '6px', color: 'var(--text-primary)' }}
                  >
                    {t('backups.snapshotLabel')}
                  </label>
                  <input
                    id="create-snap-label"
                    type="text"
                    value={createLabel}
                    onChange={(e) => setCreateLabel(e.target.value)}
                    placeholder={t('backups.snapshotLabelPlaceholder')}
                    style={{
                      width: '100%',
                      backgroundColor: 'var(--bg-input)',
                      color: 'var(--text-primary)',
                      border: '1px solid var(--border-medium)',
                      borderRadius: 'var(--radius-sm)',
                      padding: '8px 12px',
                      fontSize: '13px',
                      outline: 'none',
                    }}
                  />
                </div>
              </div>

              <div className="modal-footer" style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px' }}>
                <button
                  type="button"
                  className="btn btn-secondary btn-sm"
                  onClick={() => setIsCreateModalOpen(false)}
                  disabled={creating}
                >
                  {t('common.close')}
                </button>
                <button
                  id="btn-confirm-create-snapshot"
                  type="submit"
                  className="btn btn-primary btn-sm"
                  disabled={creating || !createFarmFolder}
                >
                  {creating ? (
                    <>
                      <RefreshCw size={13} className="animate-spin" />
                      <span>{t('backups.creating')}</span>
                    </>
                  ) : (
                    <>
                      <Plus size={13} />
                      <span>{t('backups.createButton')}</span>
                    </>
                  )}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* RENAME LABEL MODAL */}
      {renamingSnapshot && (
        <div className="modal-backdrop">
          <div className="modal card animate-fade-in" style={{ maxWidth: '440px', width: '100%' }}>
            <div className="modal-header">
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Edit3 size={16} style={{ color: 'var(--accent-primary)' }} />
                <h3 style={{ fontSize: '15px', fontWeight: 700, margin: 0 }}>
                  {t('backups.renameModalTitle')}
                </h3>
              </div>
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setRenamingSnapshot(null)}
                style={{ padding: '4px 8px' }}
              >
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handleSaveRename}>
              <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div style={{ fontSize: '12.5px', color: 'var(--text-muted)' }}>
                  Snapshot: <span className="code-box">{renamingSnapshot.snapshot_id}</span>
                </div>

                <div>
                  <label
                    htmlFor="rename-snap-label"
                    style={{ display: 'block', fontSize: '12px', fontWeight: 600, marginBottom: '6px', color: 'var(--text-primary)' }}
                  >
                    {t('backups.userLabel')}
                  </label>
                  <input
                    id="rename-snap-label"
                    type="text"
                    value={newLabelText}
                    onChange={(e) => setNewLabelText(e.target.value)}
                    placeholder={t('backups.snapshotLabelPlaceholder')}
                    autoFocus
                    style={{
                      width: '100%',
                      backgroundColor: 'var(--bg-input)',
                      color: 'var(--text-primary)',
                      border: '1px solid var(--border-medium)',
                      borderRadius: 'var(--radius-sm)',
                      padding: '8px 12px',
                      fontSize: '13px',
                      outline: 'none',
                    }}
                  />
                </div>
              </div>

              <div className="modal-footer" style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px' }}>
                <button
                  type="button"
                  className="btn btn-secondary btn-sm"
                  onClick={() => setRenamingSnapshot(null)}
                  disabled={renaming}
                >
                  {t('common.close')}
                </button>
                <button
                  id="btn-confirm-rename-snapshot"
                  type="submit"
                  className="btn btn-primary btn-sm"
                  disabled={renaming}
                >
                  <Check size={13} />
                  <span>{t('backups.saveLabel')}</span>
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* EXPLICIT DELETE CONFIRMATION MODAL */}
      {deletingSnapshot && (
        <div className="modal-backdrop">
          <div className="modal card animate-fade-in" style={{ maxWidth: '480px', width: '100%' }}>
            <div className="modal-header" style={{ borderBottom: '1px solid var(--status-danger)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Trash2 size={18} style={{ color: 'var(--status-danger)' }} />
                <h3 style={{ fontSize: '16px', fontWeight: 700, margin: 0, color: 'var(--status-danger)' }}>
                  {t('backups.deleteModalTitle')}
                </h3>
              </div>
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setDeletingSnapshot(null)}
                style={{ padding: '4px 8px' }}
              >
                <X size={14} />
              </button>
            </div>

            <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
              <div className="banner banner-danger" style={{ margin: 0 }}>
                <AlertTriangle size={18} />
                <span style={{ fontSize: '12.5px' }}>{t('backups.deleteWarning')}</span>
              </div>

              <div style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                Target Snapshot: <strong style={{ color: 'var(--text-primary)' }}>{deletingSnapshot.snapshot_id}</strong>
                <br />
                Farm: {deletingSnapshot.farm_name} ({deletingSnapshot.farm_folder_name})
              </div>

              {deleteError && (
                <div style={{ color: 'var(--status-danger)', fontSize: '12px' }}>
                  {deleteError}
                </div>
              )}

              <div>
                <label
                  htmlFor="delete-confirm-input"
                  style={{ display: 'block', fontSize: '12.5px', fontWeight: 600, marginBottom: '6px', color: 'var(--text-primary)' }}
                >
                  {t('backups.deletePrompt')}
                </label>
                <input
                  id="delete-confirm-input"
                  type="text"
                  value={deleteConfirmInput}
                  onChange={(e) => setDeleteConfirmInput(e.target.value)}
                  placeholder="DELETE"
                  autoFocus
                  style={{
                    width: '100%',
                    backgroundColor: 'var(--bg-input)',
                    color: 'var(--status-danger)',
                    fontWeight: 700,
                    letterSpacing: '0.05em',
                    border: '1px solid var(--status-danger)',
                    borderRadius: 'var(--radius-sm)',
                    padding: '8px 12px',
                    fontSize: '13px',
                    outline: 'none',
                  }}
                />
              </div>
            </div>

            <div className="modal-footer" style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px' }}>
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                onClick={() => setDeletingSnapshot(null)}
                disabled={deleting}
              >
                {t('common.close')}
              </button>
              <button
                id="btn-confirm-delete-snapshot"
                type="button"
                className="btn btn-danger btn-sm"
                onClick={handleConfirmDelete}
                disabled={deleting || deleteConfirmInput !== 'DELETE'}
              >
                {deleting ? (
                  <RefreshCw size={13} className="animate-spin" />
                ) : (
                  <Trash2 size={13} />
                )}
                <span>{t('backups.deleteConfirmButton')}</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* SIDE-BY-SIDE COMPARISON MODAL */}
      {comparisonResult && (
        <div className="modal-backdrop">
          <div className="modal card animate-fade-in" style={{ maxWidth: '800px', width: '100%' }}>
            <div className="modal-header">
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Scale size={18} style={{ color: 'var(--accent-gold)' }} />
                <h3 style={{ fontSize: '16px', fontWeight: 700, margin: 0 }}>
                  {t('backups.compareModalTitle')}
                </h3>
              </div>
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setComparisonResult(null)}
                style={{ padding: '4px 8px' }}
              >
                <X size={14} />
              </button>
            </div>

            <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div className="banner banner-info" style={{ margin: 0 }}>
                <ShieldCheck size={16} />
                <span style={{ fontSize: '12px' }}>{t('backups.compareNotice')}</span>
              </div>

              {/* Hash match alert */}
              <div
                className={`banner ${comparisonResult.hashes_match ? 'banner-success' : 'banner-warning'}`}
                style={{ margin: 0 }}
              >
                {comparisonResult.hashes_match ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
                <span style={{ fontSize: '12.5px', fontWeight: 600 }}>
                  {comparisonResult.hashes_match
                    ? t('backups.identicalHashes')
                    : t('backups.differingHashes')}
                </span>
              </div>

              {/* Comparison Grid */}
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                {/* Snapshot A */}
                <div style={{ backgroundColor: 'var(--bg-input)', padding: '14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-medium)' }}>
                  <div style={{ fontSize: '13px', fontWeight: 700, color: 'var(--accent-primary)', marginBottom: '8px' }}>
                    {t('backups.snapshotA')}
                  </div>
                  <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)', marginBottom: '10px' }}>
                    {comparisonResult.snapshot_id_a}
                  </div>
                  <div className="kv-list" style={{ fontSize: '12.5px' }}>
                    <div className="kv-item">
                      <span className="kv-key">{t('backups.inGameDate')}</span>
                      <span className="kv-val">{comparisonResult.date_a || 'N/A'}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('characters.currentFunds')}</span>
                      <span className="kv-val" style={{ color: 'var(--accent-gold)' }}>
                        {comparisonResult.money_a !== null && comparisonResult.money_a !== undefined
                          ? `${comparisonResult.money_a.toLocaleString()}g`
                          : 'N/A'}
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('characters.playTime')}</span>
                      <span className="kv-val">{comparisonResult.playtime_a || 'N/A'}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('characters.title')}</span>
                      <span className="kv-val">{comparisonResult.characters_a}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('backups.size')}</span>
                      <span className="kv-val">
                        {(comparisonResult.total_size_a / (1024 * 1024)).toFixed(2)} MB
                      </span>
                    </div>
                  </div>
                </div>

                {/* Snapshot B */}
                <div style={{ backgroundColor: 'var(--bg-input)', padding: '14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-medium)' }}>
                  <div style={{ fontSize: '13px', fontWeight: 700, color: 'var(--accent-gold)', marginBottom: '8px' }}>
                    {t('backups.snapshotB')}
                  </div>
                  <div style={{ fontSize: '11px', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)', marginBottom: '10px' }}>
                    {comparisonResult.snapshot_id_b}
                  </div>
                  <div className="kv-list" style={{ fontSize: '12.5px' }}>
                    <div className="kv-item">
                      <span className="kv-key">{t('backups.inGameDate')}</span>
                      <span className="kv-val">{comparisonResult.date_b || 'N/A'}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('characters.currentFunds')}</span>
                      <span className="kv-val" style={{ color: 'var(--accent-gold)' }}>
                        {comparisonResult.money_b !== null && comparisonResult.money_b !== undefined
                          ? `${comparisonResult.money_b.toLocaleString()}g`
                          : 'N/A'}
                      </span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('characters.playTime')}</span>
                      <span className="kv-val">{comparisonResult.playtime_b || 'N/A'}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('characters.title')}</span>
                      <span className="kv-val">{comparisonResult.characters_b}</span>
                    </div>
                    <div className="kv-item">
                      <span className="kv-key">{t('backups.size')}</span>
                      <span className="kv-val">
                        {(comparisonResult.total_size_b / (1024 * 1024)).toFixed(2)} MB
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Changed files */}
              {comparisonResult.files_changed.length > 0 && (
                <div>
                  <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', display: 'block', marginBottom: '6px' }}>
                    {t('backups.filesChanged')} ({comparisonResult.files_changed.length})
                  </span>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                    {comparisonResult.files_changed.map((f, i) => (
                      <span key={i} className="badge badge-secondary" style={{ fontFamily: 'var(--font-mono)' }}>
                        {f}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>

            <div className="modal-footer" style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                onClick={() => setComparisonResult(null)}
              >
                {t('common.close')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
