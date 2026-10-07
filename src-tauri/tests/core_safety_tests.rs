mod test_fixtures;

use chrono::Utc;
use stardew_sync_core::{
    discover_saves, BackupManager, BackupType, MockProcessChecker, ProcessMonitor, RecoveryManager,
    RecoveryManifest, SafeReplacer, SaveSettleConfig, SaveSettleDetector, StagingArea,
    TransactionState,
};
use std::time::Duration;
use tempfile::tempdir;
use test_fixtures::{generate_test_save_game_info_xml, generate_test_save_xml};

#[test]
fn test_save_discovery_on_isolated_temp_directory() {
    let temp_root = tempdir().unwrap();
    let root_path = temp_root.path();

    // 1. Valid save A with all files
    let farm_a_dir = root_path.join("OakWood_12345");
    std::fs::create_dir_all(&farm_a_dir).unwrap();
    std::fs::write(farm_a_dir.join("OakWood_12345"), "data").unwrap();
    std::fs::write(farm_a_dir.join("SaveGameInfo"), "info").unwrap();
    std::fs::write(farm_a_dir.join("OakWood_12345_old"), "old_data").unwrap();
    std::fs::write(farm_a_dir.join("SaveGameInfo_old"), "old_info").unwrap();

    // 2. Valid save B with only required live files
    let farm_b_dir = root_path.join("PineFarm_67890");
    std::fs::create_dir_all(&farm_b_dir).unwrap();
    std::fs::write(farm_b_dir.join("PineFarm_67890"), "data").unwrap();
    std::fs::write(farm_b_dir.join("SaveGameInfo"), "info").unwrap();

    // 3. Incomplete directory (missing primary save file)
    let invalid_dir = root_path.join("Broken_99999");
    std::fs::create_dir_all(&invalid_dir).unwrap();
    std::fs::write(invalid_dir.join("SaveGameInfo"), "info").unwrap();

    // 4. Loose file in root
    std::fs::write(root_path.join("random.txt"), "noise").unwrap();

    let discovered = discover_saves(root_path).expect("Discovery should succeed");

    assert_eq!(discovered.len(), 2);
    assert_eq!(discovered[0].folder_name, "OakWood_12345");
    assert_eq!(discovered[0].farm_name, "OakWood");
    assert_eq!(discovered[0].game_id, "12345");
    assert!(discovered[0].has_old_save);
    assert!(discovered[0].has_old_save_info);

    assert_eq!(discovered[1].folder_name, "PineFarm_67890");
    assert_eq!(discovered[1].farm_name, "PineFarm");
    assert_eq!(discovered[1].game_id, "67890");
    assert!(!discovered[1].has_old_save);
    assert!(!discovered[1].has_old_save_info);
}

#[test]
fn test_backup_creation_verification_and_tamper_detection() {
    let temp_root = tempdir().unwrap();
    let save_dir = temp_root.path().join("Emerald_42");
    std::fs::create_dir_all(&save_dir).unwrap();
    std::fs::write(save_dir.join("Emerald_42"), generate_test_save_xml()).unwrap();
    std::fs::write(
        save_dir.join("SaveGameInfo"),
        generate_test_save_game_info_xml(),
    )
    .unwrap();

    let backups_root = temp_root.path().join("backups");

    // 1. Create verified backup
    let manifest =
        BackupManager::create_backup(&save_dir, &backups_root, BackupType::Manual, false)
            .expect("Backup creation should succeed");

    let backup_dir = backups_root.join("Emerald_42").join(&manifest.backup_id);
    assert!(backup_dir.is_dir());

    // 2. Verify backup integrity
    let verified = BackupManager::verify_backup(&backup_dir).expect("Verification must pass");
    assert_eq!(verified.files.len(), 2);

    // 3. Tamper test: Corrupt 1 byte in backup save file
    let save_file_path = backup_dir.join("Emerald_42");
    let mut bytes = std::fs::read(&save_file_path).unwrap();
    bytes[50] ^= 0xFF; // Flip bits
    std::fs::write(&save_file_path, bytes).unwrap();

    // 4. Verify should now detect the tamper and reject the backup
    let err = BackupManager::verify_backup(&backup_dir)
        .expect_err("Tampered backup must fail verification");
    assert!(err.to_string().contains("hash mismatch"));
}

#[test]
fn test_original_import_protection_from_retention() {
    let temp_root = tempdir().unwrap();
    let save_dir = temp_root.path().join("Farm_100");
    std::fs::create_dir_all(&save_dir).unwrap();
    std::fs::write(save_dir.join("Farm_100"), "content").unwrap();
    std::fs::write(save_dir.join("SaveGameInfo"), "info").unwrap();

    let backups_root = temp_root.path().join("backups");
    let farm_backups_dir = backups_root.join("Farm_100");

    // 1. Create Original Import backup (must be protected forever)
    let orig_import =
        BackupManager::create_backup(&save_dir, &backups_root, BackupType::OriginalImport, true)
            .unwrap();
    assert!(orig_import.is_protected);

    // 2. Create 5 regular unprotected backups
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(15));
        BackupManager::create_backup(&save_dir, &backups_root, BackupType::PreSync, false).unwrap();
    }

    // 3. Enforce retention to keep at most 2 unprotected backups
    let deleted = BackupManager::enforce_retention(&farm_backups_dir, 2)
        .expect("Retention enforcement should succeed");

    assert_eq!(
        deleted, 3,
        "Should have pruned 3 excess unprotected backups"
    );

    // 4. Assert the Original Import backup still exists and is untouched
    let orig_import_dir = farm_backups_dir.join(&orig_import.backup_id);
    assert!(
        orig_import_dir.is_dir(),
        "Original Import backup must NEVER be deleted"
    );
    BackupManager::verify_backup(&orig_import_dir)
        .expect("Original Import backup must still be valid");
}

#[test]
fn test_staging_area_and_safe_transactional_replacement() {
    let temp_root = tempdir().unwrap();
    let work_dir = temp_root.path().join("work");
    let backups_dir = temp_root.path().join("backups");
    let live_dir = temp_root.path().join("Farm_200");
    let manifest_path = temp_root.path().join("tx_manifest.json");

    std::fs::create_dir_all(&live_dir).unwrap();
    std::fs::write(live_dir.join("Farm_200"), generate_test_save_xml()).unwrap();
    std::fs::write(
        live_dir.join("SaveGameInfo"),
        generate_test_save_game_info_xml(),
    )
    .unwrap();

    // 1. Create staging area and populate
    let staging = StagingArea::new(&work_dir).unwrap();
    staging.populate_from(&live_dir).unwrap();

    // Modify a value in staging
    let new_save_content = generate_test_save_xml().replace("summer", "fall");
    std::fs::write(staging.path.join("Farm_200"), new_save_content).unwrap();

    let staged_path = staging.release_for_commit();

    // 2. Execute safe replacement
    SafeReplacer::replace_save_directory(&live_dir, &staged_path, &backups_dir, &manifest_path)
        .expect("Safe replacement should succeed");

    // 3. Verify live folder now has the new content
    let live_content = std::fs::read_to_string(live_dir.join("Farm_200")).unwrap();
    assert!(
        live_content.contains("fall"),
        "Live folder must have updated staged content"
    );

    // 4. Verify safety backup of pre-replacement state was created
    let farm_backups = backups_dir.join("Farm_200");
    assert!(
        farm_backups.is_dir(),
        "A safety backup must be created before replacing"
    );
}

#[test]
fn test_process_monitor_mock() {
    let mock = MockProcessChecker::new();
    let monitor = ProcessMonitor::with_checker(mock);

    assert!(!monitor.is_stardew_running());

    monitor
        .checker
        .set_running(vec!["notepad.exe", "chrome.exe"]);
    assert!(!monitor.is_stardew_running());

    monitor.checker.set_running(vec!["Stardew Valley.exe"]);
    assert!(monitor.is_stardew_running());

    monitor.checker.set_running(vec!["StardewModdingAPI.exe"]);
    assert!(monitor.is_stardew_running());
}

#[test]
fn test_save_settle_detector() {
    let temp_root = tempdir().unwrap();
    let save_dir = temp_root.path().join("SettleTest_300");
    std::fs::create_dir_all(&save_dir).unwrap();
    std::fs::write(save_dir.join("file1"), "hello").unwrap();

    let config = SaveSettleConfig {
        required_stable_observations: 2,
        check_interval: Duration::from_millis(50),
        timeout: Duration::from_secs(2),
    };

    // Since file is untouched, settle detector should succeed quickly
    SaveSettleDetector::wait_for_settled(&save_dir, &config)
        .expect("Settle detection should succeed for stable files");
}

#[test]
fn test_transaction_crash_recovery_restores_live_save() {
    let temp_root = tempdir().unwrap();
    let live_dir = temp_root.path().join("Farm_Crashed");
    let rollback_dir = live_dir.with_extension("rollback_tmp");
    let staging_dir = temp_root.path().join("staging_crashed");
    let manifest_path = temp_root.path().join("tx_crash_manifest.json");

    // Simulate scenario: power loss / kill during swapping.
    // Live dir was renamed to rollback_dir, staging dir exists, but live dir is missing.
    std::fs::create_dir_all(&rollback_dir).unwrap();
    std::fs::write(rollback_dir.join("Farm_Crashed"), "original_pre_swap_data").unwrap();
    std::fs::write(rollback_dir.join("SaveGameInfo"), "original_info").unwrap();

    std::fs::create_dir_all(&staging_dir).unwrap();
    std::fs::write(staging_dir.join("Farm_Crashed"), "staged_data").unwrap();

    let manifest = RecoveryManifest {
        tx_id: "crash-test-id".to_string(),
        created_at: Utc::now(),
        state: TransactionState::Swapping,
        live_path: live_dir.clone(),
        staging_path: staging_dir.clone(),
        rollback_path: Some(rollback_dir.clone()),
    };
    RecoveryManager::write_manifest(&manifest_path, &manifest).unwrap();

    // Run recovery
    let recovered = RecoveryManager::recover_interrupted_transaction(&manifest_path)
        .expect("Recovery should succeed");

    assert!(recovered, "Must report recovered = true");
    assert!(live_dir.is_dir(), "Live directory must be restored");
    assert_eq!(
        std::fs::read_to_string(live_dir.join("Farm_Crashed")).unwrap(),
        "original_pre_swap_data",
        "Live directory must have original data restored from rollback folder"
    );
    assert!(
        !rollback_dir.exists(),
        "Rollback directory must be cleaned up"
    );
    assert!(
        !staging_dir.exists(),
        "Staging directory must be cleaned up"
    );
    assert!(
        !manifest_path.exists(),
        "Crash manifest must be deleted after recovery"
    );
}
