import React, { useState, useEffect, useCallback } from 'react';
import {
  AppStatus,
  ProcessStatus,
  FarmInfo,
  SnapshotInfo,
  FarmDetailedMetadata,
  ActiveTab,
  ThemeMode,
} from './types';
import {
  fetchAppStatus,
  fetchProcessStatus,
  fetchDiscoveredFarms,
  fetchSnapshots,
  fetchFarmMetadata,
} from './api/tauri';
import { Navigation } from './components/Navigation';
import { Header } from './components/Header';
import { Dashboard } from './pages/Dashboard';
import { FarmsPage } from './pages/FarmsPage';
import { BackupsPage } from './pages/BackupsPage';
import { SettingsPage } from './pages/SettingsPage';
import { UpdatesPage } from './pages/UpdatesPage';
import { FarmModal } from './components/FarmModal';
import './styles/main.css';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<ActiveTab>('dashboard');
  const [theme, setTheme] = useState<ThemeMode>(() => {
    return (localStorage.getItem('theme') as ThemeMode) || 'dark';
  });

  const [appStatus, setAppStatus] = useState<AppStatus | null>(null);
  const [processStatus, setProcessStatus] = useState<ProcessStatus | null>(null);
  const [farms, setFarms] = useState<FarmInfo[]>([]);
  const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([]);

  const [loadingProcess, setLoadingProcess] = useState(false);
  const [loadingFarms, setLoadingFarms] = useState(false);
  const [loadingSnapshots, setLoadingSnapshots] = useState(false);

  // Inspector modal state
  const [selectedFarmMeta, setSelectedFarmMeta] = useState<FarmDetailedMetadata | null>(null);
  const [loadingModal, setLoadingModal] = useState(false);

  // Synchronize theme with HTML document
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('theme', theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme((prev) => (prev === 'dark' ? 'light' : 'dark'));
  };

  // Data fetching functions
  const loadProcess = useCallback(async () => {
    setLoadingProcess(true);
    try {
      const res = await fetchProcessStatus();
      setProcessStatus(res);
    } catch (err) {
      console.error('Failed to fetch process status:', err);
    } finally {
      setLoadingProcess(false);
    }
  }, []);

  const loadFarms = useCallback(async () => {
    setLoadingFarms(true);
    try {
      const res = await fetchDiscoveredFarms();
      setFarms(res);
    } catch (err) {
      console.error('Failed to discover farms:', err);
    } finally {
      setLoadingFarms(false);
    }
  }, []);

  const loadSnapshots = useCallback(async () => {
    setLoadingSnapshots(true);
    try {
      const res = await fetchSnapshots();
      setSnapshots(res);
    } catch (err) {
      console.error('Failed to list snapshots:', err);
    } finally {
      setLoadingSnapshots(false);
    }
  }, []);

  useEffect(() => {
    // Initial loads
    fetchAppStatus()
      .then(setAppStatus)
      .catch((err) => console.error('Failed to get app status:', err));

    loadProcess();
    loadFarms();
    loadSnapshots();

    // Auto-poll process status every 8 seconds to detect game start/exit
    const interval = setInterval(loadProcess, 8000);
    return () => clearInterval(interval);
  }, [loadProcess, loadFarms, loadSnapshots]);

  const handleInspectFarm = async (folderName: string) => {
    setLoadingModal(true);
    try {
      const meta = await fetchFarmMetadata(folderName);
      setSelectedFarmMeta(meta);
    } catch (err) {
      console.error('Failed to inspect farm metadata:', err);
    } finally {
      setLoadingModal(false);
    }
  };

  const getPageTitle = (tab: ActiveTab): string => {
    switch (tab) {
      case 'dashboard':
        return 'Overview Dashboard';
      case 'farms':
        return 'Farm Saves Discovery';
      case 'backups':
        return 'Snapshot Management';
      case 'settings':
        return 'Application Settings';
      case 'updates':
        return 'Software Updates';
    }
  };

  return (
    <div className="app-layout">
      {/* Navigation Sidebar */}
      <Navigation
        activeTab={activeTab}
        onSelectTab={setActiveTab}
        appVersion={appStatus?.app_version || '0.1.0'}
      />

      {/* Main Content Area */}
      <div className="content-wrapper">
        <Header
          title={getPageTitle(activeTab)}
          processStatus={processStatus}
          loadingProcess={loadingProcess}
          onRefreshProcess={loadProcess}
          theme={theme}
          onToggleTheme={toggleTheme}
        />

        <main className="main-view">
          {activeTab === 'dashboard' && (
            <Dashboard
              appStatus={appStatus}
              processStatus={processStatus}
              farms={farms}
              snapshots={snapshots}
              onNavigate={setActiveTab}
              onInspectFarm={handleInspectFarm}
            />
          )}

          {activeTab === 'farms' && (
            <FarmsPage
              farms={farms}
              loading={loadingFarms}
              onRefresh={loadFarms}
              onInspectFarm={handleInspectFarm}
            />
          )}

          {activeTab === 'backups' && (
            <BackupsPage
              snapshots={snapshots}
              loading={loadingSnapshots}
              onRefresh={loadSnapshots}
            />
          )}

          {activeTab === 'settings' && (
            <SettingsPage
              appStatus={appStatus}
              processStatus={processStatus}
              theme={theme}
              onSetTheme={setTheme}
            />
          )}

          {activeTab === 'updates' && (
            <UpdatesPage
              appStatus={appStatus}
              processStatus={processStatus}
            />
          )}
        </main>
      </div>

      {/* Deep Inspection Modal */}
      {(selectedFarmMeta || loadingModal) && (
        <FarmModal
          metadata={selectedFarmMeta}
          loading={loadingModal}
          onClose={() => setSelectedFarmMeta(null)}
        />
      )}
    </div>
  );
};
