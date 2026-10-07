mod test_fixtures;

use stardew_sync_core::{
    DisposableIdentity, DisposableSaveManager, MockProcessChecker, PostRuntimeAnalyzer,
    ProcessMonitor, ProductionGuard, SaveValidator, PRODUCTION_FARM_FOLDER, PRODUCTION_GAME_ID,
};
use std::path::Path;
use test_fixtures::{
    generate_test_save_1_6_xml, generate_test_save_game_info_xml, PLAYER_A_ID, PLAYER_B_ID,
};

#[test]
fn test_production_farm_folder_identity_strictly_rejected() {
    // Exact match
    let exact = Path::new("C:\\Users\\bazq3\\AppData\\Roaming\\StardewValley\\Saves")
        .join(PRODUCTION_FARM_FOLDER);
    assert!(ProductionGuard::validate_target_path(&exact).is_err());

    // Case-insensitive match
    let lower = Path::new("C:\\Saves").join("txrk_450560341");
    assert!(ProductionGuard::validate_target_path(&lower).is_err());

    // Subpath match
    let inside = Path::new("C:\\Saves\\TXrk_450560341\\TXrk_450560341");
    assert!(ProductionGuard::validate_target_path(inside).is_err());

    // Contains production game ID
    let with_id = Path::new("C:\\Saves\\AnyFarm_450560341");
    assert!(ProductionGuard::validate_target_path(with_id).is_err());

    // Valid disposable target path accepted
    let valid_test = Path::new("C:\\Saves\\TXrkTest_999450560");
    assert!(ProductionGuard::validate_target_path(valid_test).is_ok());
}

#[test]
fn test_production_save_content_strictly_rejected() {
    let prod_xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <uniqueIDForThisGame>{}</uniqueIDForThisGame>
  <player><name>Kubilay</name><UniqueMultiplayerID>100</UniqueMultiplayerID></player>
  <locations><GameLocation xsi:type="Farm"><name>Farm</name></GameLocation></locations>
</SaveGame>"#,
        PRODUCTION_GAME_ID
    );
    assert!(ProductionGuard::validate_save_content(&prod_xml).is_err());

    let safe_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <uniqueIDForThisGame>999450560</uniqueIDForThisGame>
  <player><name>Kubilay</name><UniqueMultiplayerID>100</UniqueMultiplayerID></player>
  <locations><GameLocation xsi:type="Farm"><name>Farm</name></GameLocation></locations>
</SaveGame>"#;
    assert!(ProductionGuard::validate_save_content(safe_xml).is_ok());
}

#[test]
fn test_stardew_running_state_blocks_installation() {
    let mock = MockProcessChecker::new();
    mock.set_running(vec!["Stardew Valley.exe"]);
    let monitor = ProcessMonitor::with_checker(mock);

    let target_path = Path::new("C:\\Saves\\TXrkTest_999450560");
    let safe_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <uniqueIDForThisGame>999450560</uniqueIDForThisGame>
  <player><name>Kubilay</name><UniqueMultiplayerID>100</UniqueMultiplayerID></player>
  <locations><GameLocation xsi:type="Farm"><name>Farm</name></GameLocation></locations>
</SaveGame>"#;

    let res = ProductionGuard::validate_safe_for_install(&monitor, target_path, safe_xml);
    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("Game process is currently running"));
}

#[test]
fn test_disposable_save_preparation_and_baseline_manifest_creation() {
    let source_save = generate_test_save_1_6_xml();
    let source_info = generate_test_save_game_info_xml();

    let identity = DisposableIdentity::default();
    assert_eq!(identity.folder_name(), "TXrkTest_999450560");
    assert_eq!(identity.farm_name, "TürkTest");

    let (state_a, state_b) = DisposableSaveManager::prepare_both_states(
        &source_save,
        &source_info,
        &identity,
        PLAYER_B_ID,
    )
    .expect("Preparation of both states must succeed");

    // State A: Host is PlayerA
    assert_eq!(state_a.manifest.host_id, PLAYER_A_ID);
    assert_eq!(state_a.manifest.host_name, "PlayerA");
    assert_eq!(state_a.manifest.game_id, 999450560);
    assert_eq!(state_a.manifest.folder_name, "TXrkTest_999450560");

    // State B: Host is PlayerB
    assert_eq!(state_b.manifest.host_id, PLAYER_B_ID);
    assert_eq!(state_b.manifest.host_name, "PlayerB");
    assert_eq!(state_b.manifest.game_id, 999450560);
    assert_eq!(state_b.manifest.folder_name, "TXrkTest_999450560");

    // Both pass production guard
    assert!(ProductionGuard::validate_save_content(&state_a.save_xml).is_ok());
    assert!(ProductionGuard::validate_save_content(&state_b.save_xml).is_ok());

    // Both manifests contain valid sha256
    assert_eq!(state_a.manifest.primary_save_sha256.len(), 64);
    assert_eq!(state_b.manifest.primary_save_sha256.len(), 64);
}

#[test]
fn test_post_runtime_analyzer_handles_gameplay_progression_and_serialization_changes() {
    let source_save = generate_test_save_1_6_xml();
    let source_info = generate_test_save_game_info_xml();
    let identity = DisposableIdentity::default();

    let (state_a, _) = DisposableSaveManager::prepare_both_states(
        &source_save,
        &source_info,
        &identity,
        PLAYER_B_ID,
    )
    .unwrap();

    // Simulate normal gameplay save rewrite by Stardew:
    // 1. Time advances to 1420 (2:20 PM)
    // 2. Player earned 500 gold
    // 3. XML has slightly different formatting/ordering
    let simulated_gameplay_save = state_a
        .save_xml
        .replace(
            "<currentSeason>summer</currentSeason>",
            "<currentSeason>summer</currentSeason>\n  <timeOfDay>1420</timeOfDay>",
        )
        .replace(
            "<name>PlayerA</name>",
            "<name>PlayerA</name>\n    <money>500</money>",
        );

    let report = PostRuntimeAnalyzer::analyze_xml(
        &state_a.manifest,
        &simulated_gameplay_save,
        &state_a.save_game_info_xml,
    )
    .expect("Analyzer should successfully process save");

    assert!(
        report.is_healthy,
        "Save should be healthy despite normal gameplay changes"
    );
    assert!(
        report.serialization_rewrite_detected,
        "Serialization rewrite should be detected"
    );
    assert!(
        !report.byte_identical_to_baseline,
        "Byte identity should differ after save"
    );
    assert!(
        report.world.gameplay_progression_observed,
        "Progression to 1420 should be observed"
    );
    assert_eq!(report.world.time_of_day, Some(1420));
    assert!(
        report.unexpected_destructive_changes.is_empty(),
        "Zero destructive changes expected"
    );
}

#[test]
fn test_post_runtime_analyzer_detects_destructive_anomalies() {
    let source_save = generate_test_save_1_6_xml();
    let source_info = generate_test_save_game_info_xml();
    let identity = DisposableIdentity::default();

    let (state_a, _) = DisposableSaveManager::prepare_both_states(
        &source_save,
        &source_info,
        &identity,
        PLAYER_B_ID,
    )
    .unwrap();

    // Corrupt the save: replace host player with a completely different ID
    let corrupted_save = state_a.save_xml.replace(
        &format!("<UniqueMultiplayerID>{}</UniqueMultiplayerID>", PLAYER_A_ID),
        "<UniqueMultiplayerID>9999999</UniqueMultiplayerID>",
    );

    let report = PostRuntimeAnalyzer::analyze_xml(
        &state_a.manifest,
        &corrupted_save,
        &state_a.save_game_info_xml,
    )
    .unwrap();

    assert!(
        !report.is_healthy,
        "Corrupted host ID must cause report.is_healthy = false"
    );
    assert!(
        !report.unexpected_destructive_changes.is_empty(),
        "Destructive changes must be recorded"
    );
}

#[test]
fn test_validator_rejects_duplicate_multiplayer_ids() {
    let duplicate_id_save = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <player>
    <name>PlayerA</name>
    <UniqueMultiplayerID>{id}</UniqueMultiplayerID>
    <homeLocation>FarmHouse</homeLocation>
  </player>
  <farmhands>
    <Farmer>
      <name>PlayerB</name>
      <UniqueMultiplayerID>{id}</UniqueMultiplayerID>
      <homeLocation>Cabin</homeLocation>
    </Farmer>
  </farmhands>
  <locations>
    <GameLocation xsi:type="Farm"><name>Farm</name></GameLocation>
  </locations>
</SaveGame>"#,
        id = 123456789
    );

    let parsed = stardew_sync_core::ParsedSave::parse(&duplicate_id_save).unwrap();
    let res = SaveValidator::validate_multiplayer_for_migration(&parsed, 123456789);
    assert!(res.is_err());
}
