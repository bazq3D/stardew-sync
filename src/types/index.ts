export interface AppStatus {
  app_version: string;
  author: string;
  platform: string;
  arch: string;
  saves_dir: string;
  saves_dir_exists: boolean;
  wgs_dir_exists: boolean;
  wgs_dir: string | null;
  sync_status: string;
  stardew_installed: boolean;
}

export interface ProcessStatus {
  is_stardew_running: boolean;
  process_names_checked: string[];
  can_safely_operate: boolean;
  checked_at: string;
}

export interface FarmInfo {
  folder_name: string;
  farm_name: string;
  game_id: string;
  is_production: bool;
  is_disposable: bool;
  host_name: string;
  farmhands: string[];
  date_summary: string;
  money: number;
  last_modified: string;
  total_size_bytes: number;
  primary_save_exists: boolean;
  savegameinfo_exists: boolean;
}

type bool = boolean;

export interface PlayerSummary {
  name: string;
  unique_multiplayer_id: number;
  home_location: string;
  house_upgrade_level: number;
  is_host: boolean;
  cabin_indoors_name: string | null;
}

export interface CabinSummary {
  building_type: string;
  tile_x: number;
  tile_y: number;
  indoors_name: string;
  upgrade_level: number;
  farmhand: PlayerSummary | null;
  farmhand_ref: number | null;
}

export interface FarmDetailedMetadata {
  folder_name: string;
  farm_name: string;
  game_id: string;
  is_production: boolean;
  is_disposable: boolean;
  host: PlayerSummary | null;
  farmhands: PlayerSummary[];
  cabins: CabinSummary[];
  in_game_date: string;
  play_time_hours: number;
  game_version: string;
  sha256_primary: string | null;
  sha256_savegameinfo: string | null;
}

export interface SnapshotInfo {
  id: string;
  folder_name: string;
  path: string;
  timestamp: string;
  total_bytes: number;
  files: string[];
  is_verified: boolean;
}

export interface SnapshotVerification {
  snapshot_id: string;
  is_valid: boolean;
  files_checked: number;
  primary_hash: string | null;
  savegameinfo_hash: string | null;
  xml_parseable: boolean;
  message: string;
}

export interface UpdateCheckResult {
  current_version: string;
  endpoint: string;
  public_key_configured: boolean;
  update_available: boolean;
  latest_version: string | null;
  release_notes: string | null;
  status_message: string;
}

export type ThemeMode = "dark" | "light" | "system";

export type ActiveTab = "dashboard" | "farms" | "backups" | "settings" | "updates";
