import React from 'react';
import { ActiveTab } from '../types';
import {
  LayoutDashboard,
  Sprout,
  Archive,
  Settings,
  Sparkles,
  Users,
  WifiOff,
} from 'lucide-react';

interface NavigationProps {
  activeTab: ActiveTab;
  onSelectTab: (tab: ActiveTab) => void;
  appVersion: string;
}

export const Navigation: React.FC<NavigationProps> = ({
  activeTab,
  onSelectTab,
  appVersion,
}) => {
  const navItems: { id: ActiveTab; label: string; icon: React.ReactNode }[] = [
    {
      id: 'dashboard',
      label: 'Dashboard',
      icon: <LayoutDashboard size={18} />,
    },
    {
      id: 'farms',
      label: 'Farms',
      icon: <Sprout size={18} />,
    },
    {
      id: 'backups',
      label: 'Backups',
      icon: <Archive size={18} />,
    },
    {
      id: 'settings',
      label: 'Settings',
      icon: <Settings size={18} />,
    },
    {
      id: 'updates',
      label: 'Updates',
      icon: <Sparkles size={18} />,
    },
  ];

  return (
    <nav className="sidebar" aria-label="Main Navigation">
      <div>
        {/* Brand Section */}
        <div className="brand-section">
          <img
            src="/app-icon.png"
            alt="Stardew Sync"
            className="brand-logo"
          />
          <div className="brand-title-wrap">
            <span className="brand-title">Stardew Sync</span>
            <span className="brand-subtitle">v{appVersion} • by bazq</span>
          </div>
        </div>

        {/* Navigation Items */}
        <div className="nav-links">
          {navItems.map((item) => (
            <button
              key={item.id}
              id={`nav-${item.id}`}
              className={`nav-item ${activeTab === item.id ? 'active' : ''}`}
              onClick={() => onSelectTab(item.id)}
            >
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Sidebar Footer */}
      <div className="sidebar-footer">
        <div className="system-status-indicator">
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Users size={13} style={{ color: 'var(--accent-primary)' }} />
            <span style={{ fontWeight: 600 }}>Local Co-Op</span>
          </div>
          <span className="badge badge-gold" style={{ fontSize: '9.5px', padding: '1px 5px' }}>
            READY
          </span>
        </div>

        <div className="system-status-indicator">
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <WifiOff size={13} style={{ color: 'var(--text-muted)' }} />
            <span>P2P Sync</span>
          </div>
          <span style={{ color: 'var(--text-muted)', fontSize: '10.5px' }}>
            Offline
          </span>
        </div>
      </div>
    </nav>
  );
};
