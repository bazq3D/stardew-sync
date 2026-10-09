import { invoke } from "@tauri-apps/api/core";
import {
  AppStatus,
  ProcessStatus,
  FarmInfo,
  FarmDetailedMetadata,
  SnapshotInfo,
  SnapshotVerification,
  UpdateCheckResult,
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
