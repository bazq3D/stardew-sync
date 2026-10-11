import { invoke } from "@tauri-apps/api/core";
import {
  AppStatus,
  ProcessStatus,
  FarmInfo,
  FarmDetailedMetadata,
  SnapshotInfo,
  SnapshotVerification,
  SnapshotIntegrityResult,
  SnapshotComparison,
  VersionedSnapshotManifest,
  VersionedSnapshotSummary,
  UpdateCheckResult,
  UpdateEligibility,
  AppSettings,
  BackgroundUpdateStatus,
  CharacterProfile,
  FarmMonitoringStatus,
  ObservationJournalEntry,
  PublicDeviceInfo,
  PairedDeviceWithStatus,
  DiscoveredPeer,
  PairingInvitation,
  ActivePairingSessionView,
  UnpairResult,
} from "../types";

// Helper to detect if running inside Tauri webview
export const isTauriEnvironment = (): boolean => {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
};

export async function fetchAppStatus(): Promise<AppStatus> {
  if (!isTauriEnvironment()) {
    return {
      app_version: "0.1.1",
      author: "bazq",
      platform: "windows",
      arch: "x86_64",
      saves_dir: "%APPDATA%\\StardewValley\\Saves",
      saves_dir_exists: true,
      wgs_dir_exists: false,
      wgs_dir: null,
      sync_status: "Offline (Phase 5.1 Public Foundation)",
      stardew_installed: true,
    };
  }
  return await invoke<AppStatus>("get_app_status");
}

export async function fetchProcessStatus(): Promise<ProcessStatus> {
  if (!isTauriEnvironment()) {
    return {
      is_stardew_running: false,
      process_names_checked: [
        "Stardew Valley.exe",
        "Stardew Valley",
        "StardewModdingAPI.exe",
        "StardewModdingAPI",
      ],
      can_safely_operate: true,
      checked_at: new Date().toLocaleTimeString(),
    };
  }
  return await invoke<ProcessStatus>("get_process_status");
}

export async function fetchDiscoveredFarms(): Promise<FarmInfo[]> {
  if (!isTauriEnvironment()) {
    return [
      {
        folder_name: "Valley_100200300",
        farm_name: "Valley",
        game_id: "100200300",
        is_legacy_production: false,
        is_test_fixture: false,
        is_protected: true,
        host_name: "Farmer",
        farmhands: ["Partner"],
        date_summary: "Fall, Day 23 (Year 1)",
        money: 18450,
        last_modified: new Date().toISOString(),
        total_size_bytes: 4317484,
        primary_save_exists: true,
        savegameinfo_exists: true,
        is_production: false,
        is_disposable: false,
      },
      {
        folder_name: "Riverside_400500600",
        farm_name: "Riverside",
        game_id: "400500600",
        is_legacy_production: false,
        is_test_fixture: false,
        is_protected: true,
        host_name: "RiverHost",
        farmhands: [],
        date_summary: "Summer, Day 14 (Year 2)",
        money: 52100,
        last_modified: new Date().toISOString(),
        total_size_bytes: 4120000,
        primary_save_exists: true,
        savegameinfo_exists: true,
        is_production: false,
        is_disposable: false,
      },
    ];
  }
  return await invoke<FarmInfo[]>("discover_farms");
}

export async function fetchFarmMetadata(folderName: string): Promise<FarmDetailedMetadata> {
  if (!isTauriEnvironment()) {
    return {
      folder_name: folderName,
      farm_name: "Valley",
      game_id: "100200300",
      is_legacy_production: false,
      is_test_fixture: false,
      is_protected: true,
      host: {
        name: "Farmer",
        unique_multiplayer_id: 100200301,
        home_location: "FarmHouse",
        house_upgrade_level: 1,
        is_host: true,
        cabin_indoors_name: null,
      },
      farmhands: [
        {
          name: "Partner",
          unique_multiplayer_id: 100200302,
          home_location: "Cabin",
          house_upgrade_level: 0,
          is_host: false,
          cabin_indoors_name: "Cabin1",
        },
      ],
      cabins: [
        {
          building_type: "Stone Cabin",
          tile_x: 62,
          tile_y: 14,
          indoors_name: "Cabin1",
          upgrade_level: 0,
          farmhand: null,
          farmhand_ref: 100200302,
        },
      ],
      in_game_date: "Fall, Day 23 (Year 1)",
      play_time_hours: 42.6,
      play_time_formatted: "42h 36m (42.6 hours)",
      game_version: "1.6",
      sha256_primary: "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      sha256_savegameinfo: "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      is_production: false,
      is_disposable: false,
    };
  }
  return await invoke<FarmDetailedMetadata>("get_farm_metadata", { folderName });
}

export async function fetchSnapshots(): Promise<SnapshotInfo[]> {
  if (!isTauriEnvironment()) {
    return [];
  }
  return await invoke<SnapshotInfo[]>("list_snapshots");
}

export async function verifySnapshot(snapshotFolder: string): Promise<SnapshotVerification> {
  if (!isTauriEnvironment()) {
    return {
      snapshot_id: snapshotFolder,
      is_valid: true,
      files_checked: 2,
      primary_hash: "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      savegameinfo_hash: "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      xml_parseable: true,
      message: "Snapshot verified successfully: Cryptographic SHA-256 and XML schema valid.",
    };
  }
  return await invoke<SnapshotVerification>("verify_snapshot_integrity", {
    snapshotFolder,
  });
}

export async function createManualSnapshot(
  folderName: string,
  label?: string
): Promise<VersionedSnapshotManifest> {
  if (!isTauriEnvironment()) {
    const now = new Date();
    const id = `${now.toISOString().slice(0, 10).replace(/-/g, '')}_${now.toTimeString().slice(0, 8).replace(/:/g, '')}_MANUAL_mock123`;
    return {
      schema_version: 1,
      snapshot_id: id,
      created_at: now.toISOString(),
      snapshot_reason: "MANUAL",
      farm_folder_name: folderName,
      farm_name: "Mock Farm",
      game_id: "12345678",
      source_type: "StandardAppdata",
      source_directory: "%APPDATA%\\StardewValley\\Saves\\" + folderName,
      user_label: label || null,
      is_protected: true,
      files: [
        { file_name: folderName, sha256_hash: "MOCK_HASH_PRIMARY", size_bytes: 4000000 },
        { file_name: "SaveGameInfo", sha256_hash: "MOCK_HASH_INFO", size_bytes: 32000 },
      ],
      package_hash: "MOCK_PACKAGE_HASH",
      save_metadata: {
        in_game_date: "Fall, Day 23 (Year 1)",
        money: 18450,
        play_time_hours: 42.6,
        play_time_formatted: "42h 36m (42.6 hours)",
        host_name: "Farmer",
        character_count: 2,
      },
    };
  }
  return await invoke<VersionedSnapshotManifest>("create_manual_snapshot", {
    folderName,
    label: label || null,
  });
}

export async function fetchVersionedSnapshots(
  farmFilter?: string
): Promise<VersionedSnapshotSummary[]> {
  if (!isTauriEnvironment()) {
    const mockSnapshots: VersionedSnapshotSummary[] = [
      {
        snapshot_id: "20261010_143000_MANUAL_b7a2e190",
        farm_folder_name: "TXrk_450560341",
        farm_name: "Türk",
        game_id: "450560341",
        created_at: "2026-10-10T14:30:00Z",
        snapshot_reason: "MANUAL",
        source_type: "StandardAppdata",
        user_label: "Pre-Skull Cavern Run",
        is_protected: true,
        file_count: 2,
        total_size_bytes: 4194304,
        package_hash: "A7B4C89D01E2F3A4B5C6D7E8F9012345",
        save_metadata: {
          in_game_date: "Winter, Day 12 (Year 1)",
          money: 48250,
          play_time_hours: 18.9,
          play_time_formatted: "18h 54m (18.9 hours)",
          host_name: "Kubilay",
          character_count: 2,
        },
      },
      {
        snapshot_id: "20261007_120000_ORIGINAL_IMPORT_0001",
        farm_folder_name: "TXrk_450560341",
        farm_name: "Türk",
        game_id: "450560341",
        created_at: "2026-10-07T12:00:00Z",
        snapshot_reason: "ORIGINAL_IMPORT",
        source_type: "StandardAppdata",
        user_label: "Original Import Baseline",
        is_protected: true,
        file_count: 2,
        total_size_bytes: 4180000,
        package_hash: "B8C5D90E12F3A4B5C6D7E8F901234567",
        save_metadata: {
          in_game_date: "Fall, Day 28 (Year 1)",
          money: 32100,
          play_time_hours: 14.2,
          play_time_formatted: "14h 12m (14.2 hours)",
          host_name: "Kubilay",
          character_count: 2,
        },
      },
    ];
    return mockSnapshots.filter(
      (s) => !farmFilter || farmFilter === "all" || s.farm_folder_name === farmFilter
    );
  }
  return await invoke<VersionedSnapshotSummary[]>("list_versioned_snapshots", {
    farmFilter: farmFilter && farmFilter !== "all" ? farmFilter : null,
  });
}

export async function verifyVersionedSnapshot(
  snapshotId: string
): Promise<SnapshotIntegrityResult> {
  if (!isTauriEnvironment()) {
    return {
      snapshot_id: snapshotId,
      is_valid: true,
      files_checked: 2,
      primary_hash_matched: true,
      savegameinfo_hash_matched: true,
      package_hash_matched: true,
      xml_parseable: true,
      message: "Snapshot verified successfully (2 files, package SHA-256 and XML schema valid)",
    };
  }
  return await invoke<SnapshotIntegrityResult>("verify_versioned_snapshot", {
    snapshotId,
  });
}

export async function renameVersionedSnapshotLabel(
  snapshotId: string,
  newLabel: string
): Promise<VersionedSnapshotSummary> {
  if (!isTauriEnvironment()) {
    return {
      snapshot_id: snapshotId,
      farm_folder_name: "TXrk_450560341",
      farm_name: "Türk",
      game_id: "450560341",
      created_at: new Date().toISOString(),
      snapshot_reason: "MANUAL",
      source_type: "StandardAppdata",
      user_label: newLabel,
      is_protected: true,
      file_count: 2,
      total_size_bytes: 4194304,
      package_hash: "MOCK_HASH",
      save_metadata: null,
    };
  }
  return await invoke<VersionedSnapshotSummary>("rename_versioned_snapshot_label", {
    snapshotId,
    newLabel,
  });
}

export async function deleteVersionedSnapshot(
  snapshotId: string,
  confirmation: string
): Promise<void> {
  if (!isTauriEnvironment()) {
    return;
  }
  return await invoke<void>("delete_versioned_snapshot", {
    snapshotId,
    confirmation,
  });
}

export async function compareVersionedSnapshots(
  snapshotIdA: string,
  snapshotIdB: string
): Promise<SnapshotComparison> {
  if (!isTauriEnvironment()) {
    return {
      snapshot_id_a: snapshotIdA,
      snapshot_id_b: snapshotIdB,
      farm_folder_name: "TXrk_450560341",
      created_at_a: "2026-10-07T12:00:00Z",
      created_at_b: "2026-10-10T14:30:00Z",
      date_a: "Fall, Day 28 (Year 1)",
      date_b: "Winter, Day 12 (Year 1)",
      money_a: 32100,
      money_b: 48250,
      playtime_a: "14h 12m (14.2 hours)",
      playtime_b: "18h 54m (18.9 hours)",
      characters_a: 2,
      characters_b: 2,
      total_size_a: 4180000,
      total_size_b: 4194304,
      package_hash_a: "B8C5D90E12F3A4B5C6D7E8F901234567",
      package_hash_b: "A7B4C89D01E2F3A4B5C6D7E8F9012345",
      hashes_match: false,
      files_changed: ["TXrk_450560341 (modified)", "SaveGameInfo (modified)"],
    };
  }
  return await invoke<SnapshotComparison>("compare_versioned_snapshots", {
    snapshotIdA,
    snapshotIdB,
  });
}

export async function checkAppUpdates(): Promise<UpdateCheckResult> {
  if (!isTauriEnvironment()) {
    return {
      current_version: "0.1.1",
      endpoint: "https://github.com/bazq3D/stardew-sync/releases/latest/download/latest.json",
      public_key_configured: true,
      update_available: false,
      latest_version: "0.1.1",
      release_notes: "Phase 5.2 Release-Ready Desktop Foundation with secure signed auto-updater.",
      status_message: "Running Stardew Sync v0.1.1. Native updater configured for GitHub Releases.",
    };
  }
  return await invoke<UpdateCheckResult>("check_for_updates");
}

export async function fetchUpdateEligibility(
  isSaveOperationActive = false
): Promise<UpdateEligibility> {
  if (!isTauriEnvironment()) {
    return {
      can_update: !isSaveOperationActive,
      is_game_running: false,
      is_save_operation_active: isSaveOperationActive,
      reason: isSaveOperationActive
        ? "Active save operation in progress."
        : "Browser preview mode: updates allowed.",
    };
  }
  return await invoke<UpdateEligibility>("get_update_eligibility", {
    isSaveOperationActive,
  });
}

export async function reserveUpdateSlot(): Promise<string> {
  if (!isTauriEnvironment()) {
    return "mock-browser-token-" + Date.now();
  }
  return await invoke<string>("acquire_update_reservation");
}

export async function releaseUpdateSlot(token: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    return true;
  }
  return await invoke<boolean>("release_update_reservation", { token });
}

// In-memory fallback for non-Tauri preview/test environments
let mockSettings: AppSettings = {
  autostart_enabled: false,
  start_minimized: false,
  minimize_to_tray_on_close: false,
  auto_check_updates: true,
  last_update_check: null,
  last_notified_version: null,
};

export async function fetchAppSettings(): Promise<AppSettings> {
  if (!isTauriEnvironment()) {
    return { ...mockSettings };
  }
  return await invoke<AppSettings>("get_app_settings");
}

export async function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  if (!isTauriEnvironment()) {
    mockSettings = { ...settings };
    return { ...mockSettings };
  }
  return await invoke<AppSettings>("save_app_settings", { settings });
}

export async function fetchAutostartStatus(): Promise<boolean> {
  if (!isTauriEnvironment()) {
    return mockSettings.autostart_enabled;
  }
  return await invoke<boolean>("get_autostart_status");
}

export async function setAutostartStatus(enable: boolean): Promise<boolean> {
  if (!isTauriEnvironment()) {
    mockSettings.autostart_enabled = enable;
    return enable;
  }
  return await invoke<boolean>("set_autostart_status", { enable });
}

export async function triggerBackgroundUpdateCheck(): Promise<BackgroundUpdateStatus> {
  if (!isTauriEnvironment()) {
    const now = new Date().toISOString();
    mockSettings.last_update_check = now;
    return {
      is_checking: false,
      update_available: false,
      latest_version: null,
      current_version: "0.1.4",
      last_check_timestamp: now,
      release_notes: null,
      error: null,
    };
  }
  return await invoke<BackgroundUpdateStatus>("trigger_background_update_check");
}

export async function fetchBackgroundUpdateStatus(): Promise<BackgroundUpdateStatus> {
  if (!isTauriEnvironment()) {
    return {
      is_checking: false,
      update_available: false,
      latest_version: null,
      current_version: "0.1.4",
      last_check_timestamp: mockSettings.last_update_check,
      release_notes: null,
      error: null,
    };
  }
  return await invoke<BackgroundUpdateStatus>("get_background_update_status");
}

const mockCharacters: CharacterProfile[] = [
  {
    name: "Kubilay",
    unique_multiplayer_id: 1000000000001,
    is_host: true,
    farm_name: "Türk",
    folder_name: "TXrk_450560341",
    game_id: "450560341",
    cabin_name: null,
    raw_cabin_id: null,
    skills: {
      farming: 7,
      mining: 6,
      foraging: 5,
      fishing: 4,
      combat: 6,
    },
    experience_points: [5200, 2200, 2400, 3600, 3400, 0],
    max_health: 125,
    max_stamina: 304,
    money: 24500,
    total_money_earned: 98600,
    is_wallet_shared: true,
    play_time_hours: 18.9,
    play_time_formatted: "18h 54m (18.9 hours)",
    backpack_capacity: 24,
    inventory_item_count: 14,
    items: [
      { name: "Obsidian Edge", stack: 1, quality: 0, item_type: "MeleeWeapon" },
      { name: "Steel Pickaxe", stack: 1, quality: 0, item_type: "Tool" },
      { name: "Steel Axe", stack: 1, quality: 0, item_type: "Tool" },
      { name: "Fiberglass Rod", stack: 1, quality: 0, item_type: "Tool" },
      { name: "Bait", stack: 45, quality: 0, item_type: "Object" },
      { name: "Gold Ore", stack: 18, quality: 0, item_type: "Object" },
    ],
    equipped_gear: {
      hat: "Straw Hat",
      boots: "Space Boots",
      left_ring: "Glow Ring",
      right_ring: "Magnet Ring",
      weapon: "Obsidian Edge",
    },
    spouse: "Abigail",
    friendships: [
      { npc_name: "Abigail", points: 2500, hearts: 10, status: "Married" },
      { npc_name: "Linus", points: 1500, hearts: 6, status: "Friendly" },
      { npc_name: "Robin", points: 1250, hearts: 5, status: "Friendly" },
      { npc_name: "Gus", points: 1000, hearts: 4, status: "Friendly" },
    ],
    gender: "Male",
    hair_style: 4,
    skin_color: 2,
  },
  {
    name: "Partner",
    unique_multiplayer_id: 2000000000002,
    is_host: false,
    farm_name: "Türk",
    folder_name: "TXrk_450560341",
    game_id: "450560341",
    cabin_name: "Stone Cabin",
    raw_cabin_id: "FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8",
    skills: {
      farming: 4,
      mining: 3,
      foraging: 6,
      fishing: 8,
      combat: 3,
    },
    experience_points: [1600, 7200, 3500, 950, 850, 0],
    max_health: 105,
    max_stamina: 270,
    money: 24500,
    total_money_earned: 98600,
    is_wallet_shared: true,
    play_time_hours: 12.4,
    play_time_formatted: "12h 24m (12.4 hours)",
    backpack_capacity: 24,
    inventory_item_count: 8,
    items: [
      { name: "Iridium Rod", stack: 1, quality: 0, item_type: "Tool" },
      { name: "Iron Axe", stack: 1, quality: 0, item_type: "Tool" },
      { name: "Trout Soup", stack: 3, quality: 0, item_type: "Object" },
    ],
    equipped_gear: {
      hat: null,
      boots: "Tundra Boots",
      left_ring: "Small Glow Ring",
      right_ring: null,
      weapon: "Iron Edge",
    },
    spouse: null,
    friendships: [
      { npc_name: "Willy", points: 2000, hearts: 8, status: "Friendly" },
      { npc_name: "Leah", points: 1250, hearts: 5, status: "Friendly" },
    ],
    gender: "Female",
    hair_style: 12,
    skin_color: 1,
  },
];

export async function fetchAllCharacters(): Promise<CharacterProfile[]> {
  if (!isTauriEnvironment()) {
    return [...mockCharacters];
  }
  return await invoke<CharacterProfile[]>("get_all_characters");
}

export async function fetchFarmCharacters(folderName: string): Promise<CharacterProfile[]> {
  if (!isTauriEnvironment()) {
    return mockCharacters.filter((c) => c.folder_name === folderName);
  }
  return await invoke<CharacterProfile[]>("get_farm_characters", { folderName });
}

export async function fetchFarmMonitoringStatus(folderName: string): Promise<FarmMonitoringStatus> {
  if (!isTauriEnvironment()) {
    return {
      farm_id: folderName,
      is_enabled: true,
      classification: "local_change_detected",
      classification_label: "Local Change Detected",
      current_fingerprint: "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      short_fingerprint: "E3B0C442",
      last_observed_at: new Date().toISOString(),
      last_verified_change_at: new Date(Date.now() - 3600000).toISOString(),
      observation_sequence: 3,
      is_game_running: false,
      is_settling: false,
      validation_status: "valid",
      error_message: null,
      sync_indicator: "Sync Status Unknown",
    };
  }
  return await invoke<FarmMonitoringStatus>("get_farm_monitoring_status", { folderName });
}

export async function triggerMonitoringCheck(folderName: string): Promise<FarmMonitoringStatus> {
  if (!isTauriEnvironment()) {
    return {
      farm_id: folderName,
      is_enabled: true,
      classification: "unchanged",
      classification_label: "Unchanged",
      current_fingerprint: "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      short_fingerprint: "E3B0C442",
      last_observed_at: new Date().toISOString(),
      last_verified_change_at: new Date(Date.now() - 3600000).toISOString(),
      observation_sequence: 4,
      is_game_running: false,
      is_settling: false,
      validation_status: "valid",
      error_message: null,
      sync_indicator: "Sync Status Unknown",
    };
  }
  return await invoke<FarmMonitoringStatus>("trigger_monitoring_check", { folderName });
}

export async function fetchFarmObservationJournal(folderName: string): Promise<ObservationJournalEntry[]> {
  if (!isTauriEnvironment()) {
    return [
      {
        entry_id: "mock-journal-1",
        farm_id: folderName,
        sequence_number: 1,
        previous_fingerprint: null,
        current_fingerprint: "A1B2C3D4E5F67890123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0",
        first_seen: new Date(Date.now() - 86400000).toISOString(),
        last_seen: new Date(Date.now() - 86400000).toISOString(),
        detection_reason: "initial_scan",
        classification: "local_change_detected",
        game_process_running: false,
        validation_status: "valid",
        in_game_metadata: {
          in_game_date: "Fall, Day 23 (Year 1)",
          money: 45000,
          play_time_hours: 14.2,
          play_time_formatted: "14h 12m (14.2 hours)",
          host_name: "Kubilay",
          character_count: 2,
        },
      },
      {
        entry_id: "mock-journal-2",
        farm_id: folderName,
        sequence_number: 2,
        previous_fingerprint: "A1B2C3D4E5F67890123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0",
        current_fingerprint: "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
        first_seen: new Date(Date.now() - 3600000).toISOString(),
        last_seen: new Date().toISOString(),
        detection_reason: "post_game_exit",
        classification: "local_change_detected",
        game_process_running: false,
        validation_status: "valid",
        in_game_metadata: {
          in_game_date: "Winter, Day 1 (Year 1)",
          money: 52000,
          play_time_hours: 16.5,
          play_time_formatted: "16h 30m (16.5 hours)",
          host_name: "Kubilay",
          character_count: 2,
        },
      },
    ];
  }
  return await invoke<ObservationJournalEntry[]>("get_farm_observation_journal", { folderName });
}

export async function fetchAllMonitoringStatuses(): Promise<FarmMonitoringStatus[]> {
  if (!isTauriEnvironment()) {
    return [];
  }
  return await invoke<FarmMonitoringStatus[]>("get_all_monitoring_statuses");
}

export async function setMonitoringEnabled(enabled: boolean): Promise<boolean> {
  if (!isTauriEnvironment()) {
    return enabled;
  }
  return await invoke<boolean>("set_monitoring_enabled", { enabled });
}

// -----------------------------------------------------------------------------
// Phase 5A: Secure Device Identity, Pairing & Discovery API
// -----------------------------------------------------------------------------

export async function fetchLocalDeviceInfo(): Promise<PublicDeviceInfo> {
  if (!isTauriEnvironment()) {
    return {
      device_id: "a1b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef0",
      fingerprint: "a1b2-c3d4-e5f6-7890",
      device_name: "Kubilay-PC",
      public_key_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      created_at: new Date().toISOString(),
    };
  }
  return await invoke<PublicDeviceInfo>("get_local_device_info");
}

export async function setLocalDeviceName(name: string): Promise<PublicDeviceInfo> {
  if (!isTauriEnvironment()) {
    return {
      device_id: "a1b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef0",
      fingerprint: "a1b2-c3d4-e5f6-7890",
      device_name: name,
      public_key_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      created_at: new Date().toISOString(),
    };
  }
  return await invoke<PublicDeviceInfo>("set_local_device_name", { name });
}

export async function fetchPairedDevices(): Promise<PairedDeviceWithStatus[]> {
  if (!isTauriEnvironment()) {
    return [
      {
        device_id: "e5f6a1b2c3d47890123456789abcdef0123456789abcdef0123456789abcdef0",
        device_name: "Elbi-Laptop",
        public_key_hex: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        fingerprint: "e5f6-a1b2-c3d4-7890",
        paired_at: "2026-10-09T18:30:00Z",
        last_seen_at: new Date().toISOString(),
        last_ip: "192.168.1.105",
        connection_status: "connected",
      },
    ];
  }
  return await invoke<PairedDeviceWithStatus[]>("list_paired_devices");
}

export async function unpairDevice(deviceId: string): Promise<UnpairResult> {
  if (!isTauriEnvironment()) {
    return {
      local_removed: true,
      remote_notified: true,
      message: "Device successfully unpaired.",
    };
  }
  return await invoke<UnpairResult>("unpair_device", { deviceId });
}

export async function startPairingInvitation(tcpPort?: number): Promise<PairingInvitation> {
  if (!isTauriEnvironment()) {
    return {
      code: "SD-8492",
      inviter_device_id: "a1b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef0",
      inviter_device_name: "Kubilay-PC",
      inviter_public_key_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      inviter_nonce_hex: "11112222333344445555666677778888",
      tcp_port: tcpPort || 42890,
      expires_in_secs: 300,
    };
  }
  return await invoke<PairingInvitation>("start_pairing_invitation", { tcpPort });
}

export async function cancelPairingInvitation(): Promise<void> {
  if (!isTauriEnvironment()) {
    return;
  }
  await invoke("cancel_pairing_invitation");
}

export async function fetchActivePairingSession(): Promise<ActivePairingSessionView> {
  if (!isTauriEnvironment()) {
    return {
      status: "Idle",
      session_id: null,
      invitation_code: null,
      peer_device: null,
      sas_code: null,
      our_confirmed: false,
      peer_confirmed: false,
      seconds_remaining: 0,
    };
  }
  return await invoke<ActivePairingSessionView>("get_active_pairing_session");
}

export async function confirmPairingCode(code: string): Promise<any> {
  if (!isTauriEnvironment()) {
    return { device_id: "mock", device_name: "Mock Device" };
  }
  return await invoke("confirm_pairing_code", { code });
}

export async function rejectPairing(): Promise<void> {
  if (!isTauriEnvironment()) {
    return;
  }
  await invoke("reject_pairing");
}

export async function connectToPeer(address: string, invitationCode: string): Promise<string> {
  if (!isTauriEnvironment()) {
    return "481 920";
  }
  return await invoke<string>("connect_to_peer", { address, invitationCode });
}

export async function fetchDiscoveredPeers(): Promise<DiscoveredPeer[]> {
  if (!isTauriEnvironment()) {
    return [
      {
        device_id: "peer-99",
        device_name: "LivingRoom-PC",
        fingerprint: "9988-7766-5544-3322",
        address: "192.168.1.112:42890",
        is_inviting: false,
        is_paired: false,
        last_seen_secs_ago: 2,
      },
    ];
  }
  return await invoke<DiscoveredPeer[]>("get_discovered_peers");
}

export async function broadcastDiscoveryBeacon(): Promise<void> {
  if (!isTauriEnvironment()) {
    return;
  }
  await invoke("broadcast_discovery_beacon");
}




