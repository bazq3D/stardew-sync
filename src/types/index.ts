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
  is_legacy_production: boolean;
  is_test_fixture: boolean;
  is_protected: boolean;
  host_name: string;
  farmhands: string[];
  date_summary: string;
  money: number;
  last_modified: string;
  total_size_bytes: number;
  primary_save_exists: boolean;
  savegameinfo_exists: boolean;
  is_production: boolean;
  is_disposable: boolean;
}

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
  is_legacy_production: boolean;
  is_test_fixture: boolean;
  is_protected: boolean;
  host: PlayerSummary | null;
  farmhands: PlayerSummary[];
  cabins: CabinSummary[];
  in_game_date: string;
  play_time_hours: number | null;
  play_time_formatted?: string;
  game_version: string;
  sha256_primary: string | null;
  sha256_savegameinfo: string | null;
  is_production: boolean;
  is_disposable: boolean;
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

export interface SnapshotFileEntry {
  file_name: string;
  sha256_hash: string;
  size_bytes: number;
}

export interface SnapshotSaveMetadata {
  in_game_date: string | null;
  money: number | null;
  play_time_hours: number | null;
  play_time_formatted: string | null;
  host_name: string | null;
  character_count: number;
}

export interface VersionedSnapshotManifest {
  schema_version: number;
  snapshot_id: string;
  created_at: string;
  snapshot_reason: string;
  farm_folder_name: string;
  farm_name: string;
  game_id: string;
  source_type: string;
  source_directory: string;
  user_label: string | null;
  is_protected: boolean;
  files: SnapshotFileEntry[];
  package_hash: string;
  save_metadata: SnapshotSaveMetadata | null;
}

export interface VersionedSnapshotSummary {
  snapshot_id: string;
  farm_folder_name: string;
  farm_name: string;
  game_id: string;
  created_at: string;
  snapshot_reason: string;
  source_type: string;
  user_label: string | null;
  is_protected: boolean;
  file_count: number;
  total_size_bytes: number;
  package_hash: string;
  save_metadata: SnapshotSaveMetadata | null;
}

export interface SnapshotIntegrityResult {
  snapshot_id: string;
  is_valid: boolean;
  files_checked: number;
  primary_hash_matched: boolean;
  savegameinfo_hash_matched: boolean;
  package_hash_matched: boolean;
  xml_parseable: boolean;
  message: string;
}

export interface SnapshotComparison {
  snapshot_id_a: string;
  snapshot_id_b: string;
  farm_folder_name: string;
  created_at_a: string;
  created_at_b: string;
  date_a: string | null;
  date_b: string | null;
  money_a: number | null;
  money_b: number | null;
  playtime_a: string | null;
  playtime_b: string | null;
  characters_a: number;
  characters_b: number;
  total_size_a: number;
  total_size_b: number;
  package_hash_a: string;
  package_hash_b: string;
  hashes_match: boolean;
  files_changed: string[];
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

export interface UpdateEligibility {
  can_update: boolean;
  is_game_running: boolean;
  is_save_operation_active: boolean;
  reason: string;
}

export type ThemeMode = "dark" | "light" | "system";

export type ActiveTab = "dashboard" | "farms" | "characters" | "backups" | "devices" | "settings" | "updates";

export interface SkillLevels {
  farming: number;
  mining: number;
  foraging: number;
  fishing: number;
  combat: number;
}

export interface EquippedGear {
  hat: string | null;
  boots: string | null;
  left_ring: string | null;
  right_ring: string | null;
  weapon: string | null;
}

export interface FriendshipSummary {
  npc_name: string;
  points: number;
  hearts: number;
  status: string | null;
}

export interface InventoryItemSummary {
  name: string;
  stack: number;
  quality: number;
  item_type: string;
}

export interface CharacterProfile {
  name: string;
  unique_multiplayer_id: number;
  is_host: boolean;
  farm_name: string;
  folder_name: string;
  game_id: string;
  cabin_name: string | null;
  raw_cabin_id: string | null;

  // Progression & Stats
  skills: SkillLevels;
  experience_points: number[];
  max_health: number;
  max_stamina: number;
  money: number;
  total_money_earned: number;
  is_wallet_shared: boolean;
  play_time_hours: number | null;
  play_time_formatted: string;

  // Inventory & Equipment Summary
  backpack_capacity: number;
  inventory_item_count: number;
  items: InventoryItemSummary[];
  equipped_gear: EquippedGear;

  // Social & Relationships
  spouse: string | null;
  friendships: FriendshipSummary[];

  // Visual Appearance
  gender: string;
  hair_style: number;
  skin_color: number;
}

export interface AppSettings {
  autostart_enabled: boolean;
  start_minimized: boolean;
  minimize_to_tray_on_close: boolean;
  auto_check_updates: boolean;
  last_update_check: string | null;
  last_notified_version: string | null;
}

export interface BackgroundUpdateStatus {
  is_checking: boolean;
  update_available: boolean;
  latest_version: string | null;
  current_version: string;
  last_check_timestamp: string | null;
  release_notes: string | null;
  error: string | null;
}

export type SaveChangeClassification =
  | 'unchanged'
  | 'local_change_detected'
  | 'waiting_for_game_closure'
  | 'waiting_for_file_stability'
  | 'invalid_or_incomplete_save'
  | 'source_missing'
  | 'monitoring_error'
  | 'unknown_needs_review';

export type DetectionReason =
  | 'initial_scan'
  | 'filesystem_event'
  | 'periodic_reconcile'
  | 'manual_check'
  | 'post_game_exit';

export interface ObservationJournalEntry {
  entry_id: string;
  farm_id: string;
  sequence_number: number;
  previous_fingerprint: string | null;
  current_fingerprint: string;
  first_seen: string;
  last_seen: string;
  detection_reason: DetectionReason;
  classification: SaveChangeClassification;
  game_process_running: boolean;
  validation_status: any;
  in_game_metadata: SnapshotSaveMetadata | null;
}

export interface FarmMonitoringStatus {
  farm_id: string;
  is_enabled: boolean;
  classification: SaveChangeClassification;
  classification_label: string;
  current_fingerprint: string | null;
  short_fingerprint: string | null;
  last_observed_at: string | null;
  last_verified_change_at: string | null;
  observation_sequence: number;
  is_game_running: boolean;
  is_settling: boolean;
  validation_status: any;
  error_message: string | null;
  sync_indicator: string;
}

export interface PublicDeviceInfo {
  device_id: string;
  fingerprint: string;
  device_name: string;
  public_key_hex: string;
  created_at: string;
}

export interface PairedDeviceWithStatus {
  device_id: string;
  device_name: string;
  public_key_hex: string;
  fingerprint: string;
  paired_at: string;
  last_seen_at: string | null;
  last_ip: string | null;
  connection_status: 'connected' | 'online' | 'offline' | 'disconnected' | 'connecting' | 'error';
}

export interface UnpairResult {
  local_removed: boolean;
  remote_notified: boolean;
  message: string;
}

export interface DiscoveredPeer {
  device_id: string;
  device_name: string;
  fingerprint: string;
  address: string;
  is_inviting: boolean;
  is_paired: boolean;
  last_seen_secs_ago: number;
}

export interface PairingInvitation {
  code: string;
  inviter_device_id: string;
  inviter_device_name: string;
  inviter_public_key_hex: string;
  inviter_nonce_hex: string;
  tcp_port: number;
  expires_in_secs: number;
}

export type PairingStatus =
  | 'Idle'
  | 'Inviting'
  | 'Connecting'
  | 'AwaitingDualConfirmation'
  | 'Completed'
  | 'Rejected'
  | 'Expired';

export interface ActivePairingSessionView {
  status: PairingStatus;
  session_id: string | null;
  invitation_code: string | null;
  peer_device: PublicDeviceInfo | null;
  sas_code: string | null;
  our_confirmed: boolean;
  peer_confirmed: boolean;
  seconds_remaining: number;
}


