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
      app_version: "0.1.0",
      author: "bazq",
      platform: "windows",
      arch: "x86_64",
      saves_dir: "C:\\Users\\bazq3\\AppData\\Roaming\\StardewValley\\Saves",
      saves_dir_exists: true,
      wgs_dir_exists: true,
      wgs_dir: "C:\\Users\\bazq3\\AppData\\Local\\Packages\\ConcernedApe.StardewValleyPC_0c8vynj4cqe4e\\SystemAppData\\wgs",
      sync_status: "Offline (Phase 5.0 Foundation)",
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
        folder_name: "TXrk_450560341",
        farm_name: "Türk",
        game_id: "450560341",
        is_production: true,
        is_disposable: false,
        host_name: "Kubilay",
        farmhands: ["elbi"],
        date_summary: "Fall, Day 23 (Year 1)",
        money: 18450,
        last_modified: "2026-10-09 18:00:41",
        total_size_bytes: 4317484,
        primary_save_exists: true,
        savegameinfo_exists: true,
      },
      {
        folder_name: "TXrkTest_999450560",
        farm_name: "TürkTest",
        game_id: "999450560",
        is_production: false,
        is_disposable: true,
        host_name: "Kubilay",
        farmhands: ["elbi"],
        date_summary: "Summer, Day 21 (Year 1)",
        money: 15200,
        last_modified: "2026-10-08 01:35:09",
        total_size_bytes: 4210000,
        primary_save_exists: true,
        savegameinfo_exists: true,
      },
      {
        folder_name: "TXrkTestB_999450561",
        farm_name: "TürkTestB",
        game_id: "999450561",
        is_production: false,
        is_disposable: true,
        host_name: "elbi",
        farmhands: ["Kubilay"],
        date_summary: "Summer, Day 21 (Year 1)",
        money: 15200,
        last_modified: "2026-10-09 17:40:28",
        total_size_bytes: 4215000,
        primary_save_exists: true,
        savegameinfo_exists: true,
      },
    ];
  }
  return await invoke<FarmInfo[]>("discover_farms");
}

export async function fetchFarmMetadata(folderName: string): Promise<FarmDetailedMetadata> {
  if (!isTauriEnvironment()) {
    const isProd = folderName === "TXrk_450560341";
    return {
      folder_name: folderName,
      farm_name: isProd ? "Türk" : "TürkTestB",
      game_id: isProd ? "450560341" : "999450561",
      is_production: isProd,
      is_disposable: !isProd,
      host: {
        name: isProd ? "Kubilay" : "elbi",
        unique_multiplayer_id: isProd ? -4392182049182 : 8847291048291,
        home_location: "FarmHouse",
        house_upgrade_level: 1,
        is_host: true,
        cabin_indoors_name: null,
      },
      farmhands: [
        {
          name: isProd ? "elbi" : "Kubilay",
          unique_multiplayer_id: isProd ? 8847291048291 : -4392182049182,
          home_location: "Cabin",
          house_upgrade_level: 0,
          is_host: false,
          cabin_indoors_name: "Cabin2",
        },
      ],
      cabins: [
        {
          building_type: "Stone Cabin",
          tile_x: 62,
          tile_y: 14,
          indoors_name: "Cabin2",
          upgrade_level: 0,
          farmhand: null,
          farmhand_ref: isProd ? 8847291048291 : -4392182049182,
        },
      ],
      in_game_date: isProd ? "Fall, Day 23 (Year 1)" : "Summer, Day 21 (Year 1)",
      play_time_hours: isProd ? 42.6 : 38.2,
      game_version: "1.6.15",
      sha256_primary: "C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1",
      sha256_savegameinfo: "ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299",
    };
  }
  return await invoke<FarmDetailedMetadata>("get_farm_metadata", { folderName });
}

export async function fetchSnapshots(): Promise<SnapshotInfo[]> {
  if (!isTauriEnvironment()) {
    return [
      {
        id: "TXrk_450560341_2026-10-09_180041",
        folder_name: "TXrk_450560341_2026-10-09_180041",
        path: "C:\\Users\\bazq3\\Desktop\\stardew-sync-test\\production-snapshots\\TXrk_450560341_2026-10-09_180041",
        timestamp: "2026-10-09 18:00:41",
        total_bytes: 8650775,
        files: [
          "SaveGameInfo",
          "SaveGameInfo_old",
          "TXrk_450560341",
          "TXrk_450560341_old",
        ],
        is_verified: true,
      },
    ];
  }
  return await invoke<SnapshotInfo[]>("list_snapshots");
}

export async function verifySnapshot(snapshotFolder: string): Promise<SnapshotVerification> {
  if (!isTauriEnvironment()) {
    return {
      snapshot_id: snapshotFolder,
      is_valid: true,
      files_checked: 4,
      primary_hash: "C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1",
      savegameinfo_hash: "ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299",
      xml_parseable: true,
      message: "Snapshot verified successfully: 4 files checked. Cryptographic SHA-256 and XML schema valid.",
    };
  }
  return await invoke<SnapshotVerification>("verify_snapshot_integrity", {
    snapshotFolder,
  });
}

export async function checkAppUpdates(): Promise<UpdateCheckResult> {
  if (!isTauriEnvironment()) {
    return {
      current_version: "0.1.0",
      endpoint: "https://github.com/bazq3/stardew-sync-p2p/releases/latest/download/latest.json",
      public_key_configured: true,
      update_available: false,
      latest_version: "0.1.0",
      release_notes: "Phase 5.0 Desktop Application Foundation release with secure read-only commands and verified snapshot management.",
      status_message: "You are running the latest version of Stardew Sync (v0.1.0).",
    };
  }
  return await invoke<UpdateCheckResult>("check_for_updates");
}
