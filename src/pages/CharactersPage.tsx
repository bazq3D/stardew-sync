import React, { useState, useEffect, useMemo } from 'react';
import { CharacterProfile, FarmInfo } from '../types';
import { fetchAllCharacters } from '../api/tauri';
import { useTranslation } from '../i18n/LanguageContext';
import {
  User,
  Heart,
  Coins,
  Shield,
  Zap,
  Sword,
  Pickaxe,
  Fish,
  Sprout,
  TreePine,
  Briefcase,
  AlertCircle,
  RefreshCw,
  Home,
  Clock,
  Sparkles,
} from 'lucide-react';

interface CharactersPageProps {
  farms: FarmInfo[];
}

export const CharactersPage: React.FC<CharactersPageProps> = ({ farms }) => {
  const { t } = useTranslation();

  const [characters, setCharacters] = useState<CharacterProfile[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [selectedFarm, setSelectedFarm] = useState<string>('all');
  const [error, setError] = useState<string | null>(null);

  const loadCharacters = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchAllCharacters();
      setCharacters(data);
    } catch (err: any) {
      console.error('Failed to load characters:', err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadCharacters();
  }, []);

  const filteredCharacters = useMemo(() => {
    if (selectedFarm === 'all') {
      return characters;
    }
    return characters.filter((c) => c.folder_name === selectedFarm);
  }, [characters, selectedFarm]);

  const uniqueFarms = useMemo(() => {
    const map = new Map<string, string>();
    for (const c of characters) {
      if (!map.has(c.folder_name)) {
        map.set(c.folder_name, c.farm_name);
      }
    }
    for (const f of farms) {
      if (!map.has(f.folder_name)) {
        map.set(f.folder_name, f.farm_name);
      }
    }
    return Array.from(map.entries()).map(([folder, name]) => ({ folder, name }));
  }, [characters, farms]);

  const renderHeartMeter = (hearts: number) => {
    const clamped = Math.min(Math.max(hearts, 0), 14);
    return (
      <div style={{ display: 'inline-flex', alignItems: 'center', gap: '2px' }}>
        {Array.from({ length: Math.min(clamped, 10) }).map((_, i) => (
          <Heart key={i} size={11} fill="#e53e3e" color="#e53e3e" />
        ))}
        {clamped > 10 && (
          <span style={{ fontSize: '11px', fontWeight: 600, color: '#e53e3e', marginLeft: '4px' }}>
            +{clamped - 10}
          </span>
        )}
      </div>
    );
  };

  return (
    <div className="animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Top Filter and Controls Bar */}
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
              backgroundColor: 'var(--accent-gold-subtle)',
              color: 'var(--accent-gold)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <User size={20} />
          </div>
          <div>
            <h2 style={{ fontSize: '16px', fontWeight: 700, margin: 0, color: 'var(--text-primary)' }}>
              {t('characters.title')}
            </h2>
            <p style={{ fontSize: '12.5px', color: 'var(--text-secondary)', margin: 0 }}>
              {t('characters.subtitle')}
            </p>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '12.5px', color: 'var(--text-muted)' }}>
              {t('characters.farmLabel')}:
            </span>
            <select
              id="char-farm-filter"
              value={selectedFarm}
              onChange={(e) => setSelectedFarm(e.target.value)}
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
              <option value="all">{t('characters.filterAll')}</option>
              {uniqueFarms.map((f) => (
                <option key={f.folder} value={f.folder}>
                  {f.name} ({f.folder})
                </option>
              ))}
            </select>
          </div>

          <button
            id="char-btn-refresh"
            className="btn btn-sm btn-secondary"
            onClick={loadCharacters}
            disabled={loading}
          >
            <RefreshCw size={14} className={loading ? 'spin' : ''} />
            <span>{t('common.refresh')}</span>
          </button>
        </div>
      </div>

      {/* Character Transfer Feasibility Study Notice */}
      <div className="banner banner-info" style={{ margin: 0 }}>
        <AlertCircle size={18} style={{ flexShrink: 0, marginTop: '2px' }} />
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <span style={{ fontWeight: 600 }}>{t('characters.transferStudyTitle')}</span>
          <span style={{ fontSize: '12.5px', opacity: 0.9 }}>
            {t('characters.transferStudyText')}
          </span>
        </div>
      </div>

      {error && (
        <div className="banner banner-warning">
          <AlertCircle size={16} />
          <span>{error}</span>
        </div>
      )}

      {/* Characters List Grid */}
      {loading ? (
        <div className="card" style={{ textAlign: 'center', padding: '40px 20px' }}>
          <RefreshCw size={24} className="spin" style={{ margin: '0 auto 12px auto', color: 'var(--accent-primary)' }} />
          <p style={{ color: 'var(--text-muted)' }}>{t('common.loading')}</p>
        </div>
      ) : filteredCharacters.length === 0 ? (
        <div className="card" style={{ textAlign: 'center', padding: '40px 20px' }}>
          <User size={32} style={{ margin: '0 auto 12px auto', opacity: 0.4 }} />
          <p style={{ color: 'var(--text-muted)' }}>{t('characters.noCharacters')}</p>
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
          {filteredCharacters.map((c) => (
            <div key={`${c.folder_name}-${c.unique_multiplayer_id}`} className="card">
              {/* Character Header Bar */}
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  borderBottom: '1px solid var(--border-subtle)',
                  paddingBottom: '14px',
                  marginBottom: '16px',
                  flexWrap: 'wrap',
                  gap: '12px',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
                  <div
                    style={{
                      width: '44px',
                      height: '44px',
                      borderRadius: '50%',
                      backgroundColor: c.is_host ? 'var(--status-success-bg)' : 'var(--status-info-bg)',
                      border: `2px solid ${c.is_host ? 'var(--status-success)' : 'var(--status-info)'}`,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      fontWeight: 700,
                      fontSize: '16px',
                      color: c.is_host ? 'var(--status-success)' : 'var(--status-info)',
                    }}
                  >
                    {c.name.slice(0, 2).toUpperCase()}
                  </div>

                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                      <span style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-primary)' }}>
                        {c.name}
                      </span>
                      <span className={`badge ${c.is_host ? 'badge-success' : 'badge-info'}`}>
                        {c.is_host ? t('characters.roleHost') : t('characters.roleFarmhand')}
                      </span>
                      {c.spouse && (
                        <span className="badge badge-gold" style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                          <Heart size={10} fill="currentColor" />
                          <span>{c.spouse}</span>
                        </span>
                      )}
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginTop: '4px', fontSize: '12px', color: 'var(--text-muted)' }}>
                      <span>
                        {t('characters.farmLabel')}: <strong style={{ color: 'var(--text-secondary)' }}>{c.farm_name}</strong>
                      </span>
                      <span>•</span>
                      <span>
                        {t('characters.genderLabel')}: <span style={{ color: 'var(--text-secondary)' }}>
                          {c.gender === 'Female' ? t('characters.genderFemale') : c.gender === 'Male' ? t('characters.genderMale') : c.gender === 'Unknown' ? t('characters.genderUnknown') : c.gender}
                        </span>
                      </span>
                      {c.cabin_name && (
                        <>
                          <span>•</span>
                          <span
                            style={{ display: 'flex', alignItems: 'center', gap: '4px' }}
                            title={c.raw_cabin_id ? `${t('characters.internalCabinId')}: ${c.raw_cabin_id}` : undefined}
                          >
                            <Home size={12} />
                            <span>{c.cabin_name}</span>
                            {c.raw_cabin_id && (
                              <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                                ({c.raw_cabin_id.length > 18 ? `${c.raw_cabin_id.slice(0, 9)}…` : c.raw_cabin_id})
                              </span>
                            )}
                          </span>
                        </>
                      )}
                    </div>
                  </div>
                </div>

                <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12.5px', color: 'var(--text-secondary)' }}>
                    <Clock size={14} color="var(--text-muted)" />
                    <span>{c.play_time_formatted}</span>
                  </div>
                  <span className="code-box" style={{ padding: '2px 8px' }}>
                    ID: {c.unique_multiplayer_id}
                  </span>
                </div>
              </div>

              {/* Vitals, Stats & Wealth Row */}
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: 'repeat(auto-fit, minmax(140px, 1fr))',
                  gap: '12px',
                  marginBottom: '16px',
                }}
              >
                <div style={{ backgroundColor: 'var(--bg-input)', padding: '10px 14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                  <div style={{ fontSize: '11px', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '4px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <Shield size={12} color="#48bb78" />
                    <span>{t('characters.health')}</span>
                  </div>
                  <div style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)' }}>
                    {c.max_health} HP
                  </div>
                </div>

                <div style={{ backgroundColor: 'var(--bg-input)', padding: '10px 14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                  <div style={{ fontSize: '11px', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '4px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <Zap size={12} color="#4299e1" />
                    <span>{t('characters.stamina')}</span>
                  </div>
                  <div style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)' }}>
                    {c.max_stamina} SP
                  </div>
                </div>

                <div style={{ backgroundColor: 'var(--bg-input)', padding: '10px 14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                  <div style={{ fontSize: '11px', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '4px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                      <Coins size={12} color="var(--accent-gold)" />
                      <span>{t('characters.currentFunds')}</span>
                    </div>
                    {c.is_wallet_shared ? (
                      <span className="badge badge-info" style={{ fontSize: '9px', padding: '1px 5px' }} title={t('characters.sharedFundsTooltip')}>
                        {t('characters.sharedFunds')}
                      </span>
                    ) : (
                      <span className="badge badge-secondary" style={{ fontSize: '9px', padding: '1px 5px' }} title={t('characters.separateFundsTooltip')}>
                        {t('characters.separateFunds')}
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: '15px', fontWeight: 700, color: 'var(--accent-gold)' }}>
                    {c.money.toLocaleString()}g
                  </div>
                </div>

                <div style={{ backgroundColor: 'var(--bg-input)', padding: '10px 14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                  <div style={{ fontSize: '11px', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '4px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                      <Sparkles size={12} color="var(--text-muted)" />
                      <span>{t('characters.totalEarned')}</span>
                    </div>
                    {c.is_wallet_shared && (
                      <span className="badge badge-info" style={{ fontSize: '9px', padding: '1px 5px' }} title={t('characters.sharedFundsTooltip')}>
                        {t('characters.sharedFunds')}
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)' }}>
                    {c.total_money_earned.toLocaleString()}g
                  </div>
                </div>
              </div>

              {/* Skills Section */}
              <div style={{ marginBottom: '16px' }}>
                <span style={{ fontSize: '12px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.04em', color: 'var(--text-muted)', display: 'block', marginBottom: '8px' }}>
                  {t('characters.skillsTitle')}
                </span>

                <div
                  style={{
                    display: 'grid',
                    gridTemplateColumns: 'repeat(auto-fit, minmax(130px, 1fr))',
                    gap: '10px',
                  }}
                >
                  {/* Farming */}
                  <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                      <span style={{ fontSize: '12px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <Sprout size={13} color="#48bb78" /> {t('characters.skillFarming')}
                      </span>
                      <strong style={{ fontSize: '12px' }}>{t('characters.level', { level: c.skills.farming })}</strong>
                    </div>
                    <div style={{ height: '4px', width: '100%', backgroundColor: 'rgba(255,255,255,0.1)', borderRadius: '2px', overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${(c.skills.farming / 10) * 100}%`, backgroundColor: '#48bb78' }} />
                    </div>
                  </div>

                  {/* Mining */}
                  <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                      <span style={{ fontSize: '12px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <Pickaxe size={13} color="#ed8936" /> {t('characters.skillMining')}
                      </span>
                      <strong style={{ fontSize: '12px' }}>{t('characters.level', { level: c.skills.mining })}</strong>
                    </div>
                    <div style={{ height: '4px', width: '100%', backgroundColor: 'rgba(255,255,255,0.1)', borderRadius: '2px', overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${(c.skills.mining / 10) * 100}%`, backgroundColor: '#ed8936' }} />
                    </div>
                  </div>

                  {/* Foraging */}
                  <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                      <span style={{ fontSize: '12px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <TreePine size={13} color="#38a169" /> {t('characters.skillForaging')}
                      </span>
                      <strong style={{ fontSize: '12px' }}>{t('characters.level', { level: c.skills.foraging })}</strong>
                    </div>
                    <div style={{ height: '4px', width: '100%', backgroundColor: 'rgba(255,255,255,0.1)', borderRadius: '2px', overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${(c.skills.foraging / 10) * 100}%`, backgroundColor: '#38a169' }} />
                    </div>
                  </div>

                  {/* Fishing */}
                  <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                      <span style={{ fontSize: '12px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <Fish size={13} color="#3182ce" /> {t('characters.skillFishing')}
                      </span>
                      <strong style={{ fontSize: '12px' }}>{t('characters.level', { level: c.skills.fishing })}</strong>
                    </div>
                    <div style={{ height: '4px', width: '100%', backgroundColor: 'rgba(255,255,255,0.1)', borderRadius: '2px', overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${(c.skills.fishing / 10) * 100}%`, backgroundColor: '#3182ce' }} />
                    </div>
                  </div>

                  {/* Combat */}
                  <div style={{ backgroundColor: 'var(--bg-input)', padding: '8px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                      <span style={{ fontSize: '12px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <Sword size={13} color="#e53e3e" /> {t('characters.skillCombat')}
                      </span>
                      <strong style={{ fontSize: '12px' }}>{t('characters.level', { level: c.skills.combat })}</strong>
                    </div>
                    <div style={{ height: '4px', width: '100%', backgroundColor: 'rgba(255,255,255,0.1)', borderRadius: '2px', overflow: 'hidden' }}>
                      <div style={{ height: '100%', width: `${(c.skills.combat / 10) * 100}%`, backgroundColor: '#e53e3e' }} />
                    </div>
                  </div>
                </div>
              </div>

              {/* Equipped Gear & Inventory Split */}
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
                  gap: '16px',
                  marginBottom: '16px',
                }}
              >
                {/* Equipped Gear Card */}
                <div style={{ backgroundColor: 'var(--bg-input)', padding: '12px 14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                  <span style={{ fontSize: '12px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.04em', color: 'var(--text-muted)', display: 'block', marginBottom: '8px' }}>
                    {t('characters.gearTitle')}
                  </span>
                  <div className="kv-list" style={{ gap: '6px' }}>
                    <div className="kv-item" style={{ padding: '3px 0' }}>
                      <span className="kv-key">{t('characters.gearWeapon')}</span>
                      <span className="kv-val">{c.equipped_gear.weapon || t('characters.noneEquipped')}</span>
                    </div>
                    <div className="kv-item" style={{ padding: '3px 0' }}>
                      <span className="kv-key">{t('characters.gearBoots')}</span>
                      <span className="kv-val">{c.equipped_gear.boots || t('characters.noneEquipped')}</span>
                    </div>
                    <div className="kv-item" style={{ padding: '3px 0' }}>
                      <span className="kv-key">{t('characters.gearHat')}</span>
                      <span className="kv-val">{c.equipped_gear.hat || t('characters.noneEquipped')}</span>
                    </div>
                    <div className="kv-item" style={{ padding: '3px 0' }}>
                      <span className="kv-key">{t('characters.gearRings')}</span>
                      <span className="kv-val">
                        {[c.equipped_gear.left_ring, c.equipped_gear.right_ring].filter(Boolean).join(' • ') || t('characters.noneEquipped')}
                      </span>
                    </div>
                  </div>
                </div>

                {/* Inventory / Backpack Summary */}
                <div style={{ backgroundColor: 'var(--bg-input)', padding: '12px 14px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                    <span style={{ fontSize: '12px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.04em', color: 'var(--text-muted)' }}>
                      <Briefcase size={12} style={{ display: 'inline', marginRight: '4px' }} />
                      {t('characters.backpackTitle', { count: c.inventory_item_count, capacity: c.backpack_capacity })}
                    </span>
                  </div>

                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', maxHeight: '100px', overflowY: 'auto' }}>
                    {c.items.slice(0, 10).map((item, idx) => (
                      <span
                        key={idx}
                        style={{
                          backgroundColor: 'var(--bg-card)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-xs)',
                          padding: '3px 8px',
                          fontSize: '11.5px',
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '4px',
                        }}
                      >
                        <span style={{ color: 'var(--text-primary)' }}>{item.name}</span>
                        {item.stack > 1 && (
                          <span style={{ color: 'var(--text-muted)', fontSize: '10px' }}>x{item.stack}</span>
                        )}
                      </span>
                    ))}
                    {c.items.length > 10 && (
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', alignSelf: 'center', paddingLeft: '4px' }}>
                        +{c.items.length - 10} more
                      </span>
                    )}
                  </div>
                </div>
              </div>

              {/* Relationships Section */}
              {c.friendships.length > 0 && (
                <div>
                  <span style={{ fontSize: '12px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.04em', color: 'var(--text-muted)', display: 'block', marginBottom: '8px' }}>
                    {t('characters.socialTitle')} ({c.friendships.length})
                  </span>

                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
                      gap: '8px',
                    }}
                  >
                    {c.friendships.slice(0, 8).map((f) => (
                      <div
                        key={f.npc_name}
                        style={{
                          backgroundColor: 'var(--bg-input)',
                          padding: '6px 10px',
                          borderRadius: 'var(--radius-xs)',
                          border: '1px solid var(--border-subtle)',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          fontSize: '12px',
                        }}
                      >
                        <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>{f.npc_name}</span>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                          {renderHeartMeter(f.hearts)}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
