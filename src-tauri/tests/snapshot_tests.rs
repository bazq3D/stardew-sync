mod test_fixtures;

use sha2::{Digest, Sha256};
use stardew_sync_core::core::errors::CoreError;
use stardew_sync_core::core::snapshot::{
    SnapshotReason, VersionedSnapshotManager,
};
use std::fs;
use std::sync::Mutex;
use tempfile::tempdir;
use test_fixtures::{generate_test_save_game_info_xml, generate_test_save_xml};

static SNAPSHOT_TEST_MUTEX: Mutex<()> = Mutex::new(());

fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:X}", hasher.finalize())
}

#[test]
fn test_versioned_snapshot_creation_and_integrity_verification() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "Emerald_100";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    let save_xml = generate_test_save_xml();
    let info_xml = generate_test_save_game_info_xml();
    fs::write(save_dir.join(farm_folder), &save_xml).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), &info_xml).unwrap();

    // 1. Create snapshot
    let manifest = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        Some("Pre-Skull Cavern Run".to_string()),
        SnapshotReason::Manual,
    )
    .expect("Snapshot creation must succeed");

    assert_eq!(manifest.farm_folder_name, farm_folder);
    assert_eq!(manifest.user_label, Some("Pre-Skull Cavern Run".to_string()));
    assert_eq!(manifest.snapshot_reason, SnapshotReason::Manual);
    assert!(manifest.is_protected);
    assert_eq!(manifest.files.len(), 2);

    let primary_entry = manifest.files.iter().find(|f| f.file_name == farm_folder).unwrap();
    assert_eq!(primary_entry.sha256_hash, compute_sha256(save_xml.as_bytes()));

    let info_entry = manifest.files.iter().find(|f| f.file_name == "SaveGameInfo").unwrap();
    assert_eq!(info_entry.sha256_hash, compute_sha256(info_xml.as_bytes()));

    // 2. Verify snapshot integrity
    let verify_res = VersionedSnapshotManager::verify_integrity(&manifest.snapshot_id)
        .expect("Verification must succeed");
    assert!(verify_res.is_valid);
    assert_eq!(verify_res.files_checked, 2);
    assert!(verify_res.primary_hash_matched);
    assert!(verify_res.savegameinfo_hash_matched);
    assert!(verify_res.package_hash_matched);
    assert!(verify_res.xml_parseable);

    // 3. Verify listing
    let list = VersionedSnapshotManager::list_snapshots(Some(farm_folder))
        .expect("Listing must succeed");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].snapshot_id, manifest.snapshot_id);
    assert_eq!(list[0].user_label, Some("Pre-Skull Cavern Run".to_string()));
}

#[test]
fn test_corrupted_snapshot_tamper_detection() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "Emerald_100";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let manifest = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        None,
        SnapshotReason::Manual,
    )
    .unwrap();

    // Tamper with the primary file in the snapshot directory
    let snapshot_file = temp_snapshots
        .path()
        .join(farm_folder)
        .join(&manifest.snapshot_id)
        .join(farm_folder);
    fs::write(&snapshot_file, "TAMPERED_CONTENT_HERE").unwrap();

    // Verify integrity: MUST detect tampering!
    let verify_res = VersionedSnapshotManager::verify_integrity(&manifest.snapshot_id)
        .expect("Verification call should return result");
    assert!(!verify_res.is_valid, "Tampered snapshot MUST be detected as invalid");
    assert!(verify_res.message.contains("Hash mismatch"));
}

#[test]
fn test_missing_file_integrity_detection() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "Emerald_100";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let manifest = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        None,
        SnapshotReason::Manual,
    )
    .unwrap();

    // Delete SaveGameInfo from the snapshot folder
    let snapshot_info = temp_snapshots
        .path()
        .join(farm_folder)
        .join(&manifest.snapshot_id)
        .join("SaveGameInfo");
    fs::remove_file(snapshot_info).unwrap();

    let verify_res = VersionedSnapshotManager::verify_integrity(&manifest.snapshot_id)
        .expect("Verification call should return result");
    assert!(!verify_res.is_valid, "Missing file MUST be detected as invalid");
    assert!(verify_res.message.contains("Missing snapshot file"));
}

#[test]
fn test_path_traversal_rejection_in_snapshots() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let malicious_names = vec![
        "../Secret",
        "..\\Secret",
        "../../Windows",
        "sub/folder",
        "sub\\folder",
        ".hidden",
        "",
    ];

    for name in malicious_names {
        let create_res = VersionedSnapshotManager::create_snapshot(
            name,
            temp_saves.path(),
            None,
            SnapshotReason::Manual,
        );
        assert!(create_res.is_err(), "Should reject traversal in create: {}", name);

        let delete_res = VersionedSnapshotManager::delete_snapshot(name, "DELETE");
        assert!(delete_res.is_err(), "Should reject traversal in delete: {}", name);

        let verify_res = VersionedSnapshotManager::verify_integrity(name);
        assert!(verify_res.is_err(), "Should reject traversal in verify: {}", name);
    }
}

#[test]
fn test_explicit_deletion_confirmation() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "DeleteFarm_001";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let manifest = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        None,
        SnapshotReason::Manual,
    )
    .unwrap();

    let snap_dir = temp_snapshots
        .path()
        .join(farm_folder)
        .join(&manifest.snapshot_id);
    assert!(snap_dir.exists());

    // 1. Deletion with wrong confirmation string MUST fail
    let bad_delete = VersionedSnapshotManager::delete_snapshot(&manifest.snapshot_id, "CONFIRM");
    assert!(bad_delete.is_err());
    assert!(snap_dir.exists(), "Snapshot must remain intact after bad confirmation");

    // 2. Deletion with explicit "DELETE" MUST succeed
    let good_delete = VersionedSnapshotManager::delete_snapshot(&manifest.snapshot_id, "DELETE");
    assert!(good_delete.is_ok());
    assert!(!snap_dir.exists(), "Snapshot folder must be removed after successful deletion");
}

#[test]
fn test_production_path_write_protection() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "ProtectedFarm_999";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    let primary_bytes = generate_test_save_xml().into_bytes();
    let info_bytes = generate_test_save_game_info_xml().into_bytes();
    fs::write(save_dir.join(farm_folder), &primary_bytes).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), &info_bytes).unwrap();

    let hash_primary_before = compute_sha256(&primary_bytes);
    let hash_info_before = compute_sha256(&info_bytes);

    // Create multiple snapshots
    let _snap1 = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        Some("Backup 1".into()),
        SnapshotReason::Manual,
    )
    .unwrap();

    let _snap2 = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        Some("Backup 2".into()),
        SnapshotReason::Manual,
    )
    .unwrap();

    // Verify source save files are 100% UNMODIFIED byte-for-byte
    let primary_after = fs::read(save_dir.join(farm_folder)).unwrap();
    let info_after = fs::read(save_dir.join("SaveGameInfo")).unwrap();

    assert_eq!(compute_sha256(&primary_after), hash_primary_before);
    assert_eq!(compute_sha256(&info_after), hash_info_before);
    assert_eq!(primary_after, primary_bytes);
    assert_eq!(info_after, info_bytes);
}

#[test]
fn test_rename_snapshot_user_label() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "RenameFarm_111";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let manifest = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        Some("Initial Label".into()),
        SnapshotReason::Manual,
    )
    .unwrap();

    assert_eq!(manifest.user_label, Some("Initial Label".to_string()));

    // Rename to new label
    let updated = VersionedSnapshotManager::rename_label(&manifest.snapshot_id, "Updated Label".into())
        .expect("Rename must succeed");
    assert_eq!(updated.user_label, Some("Updated Label".to_string()));

    // Verify persisted manifest
    let reloaded = VersionedSnapshotManager::get_manifest(&manifest.snapshot_id).unwrap();
    assert_eq!(reloaded.user_label, Some("Updated Label".to_string()));

    // Clear label
    let cleared = VersionedSnapshotManager::rename_label(&manifest.snapshot_id, "   ".into()).unwrap();
    assert_eq!(cleared.user_label, None);
}

#[test]
fn test_snapshot_comparison() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "DiffFarm_222";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    let save_xml_a = generate_test_save_xml();
    fs::write(save_dir.join(farm_folder), &save_xml_a).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let snap_a = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        Some("Day 1".into()),
        SnapshotReason::Manual,
    )
    .unwrap();

    // Advance save: modify save content so hashes differ
    let save_xml_b = save_xml_a.replace("<name>Parsnip Seeds</name>", "<name>Cauliflower Seeds</name>");
    assert_ne!(save_xml_a, save_xml_b);
    fs::write(save_dir.join(farm_folder), &save_xml_b).unwrap();

    let snap_b = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        Some("Day 2".into()),
        SnapshotReason::Manual,
    )
    .unwrap();

    let comparison = VersionedSnapshotManager::compare_snapshots(&snap_a.snapshot_id, &snap_b.snapshot_id)
        .expect("Comparison must succeed");

    assert_eq!(comparison.snapshot_id_a, snap_a.snapshot_id);
    assert_eq!(comparison.snapshot_id_b, snap_b.snapshot_id);
    assert!(!comparison.hashes_match, "Different saves must produce different package hashes");
    assert!(comparison.files_changed.iter().any(|f| f.contains(farm_folder)));
}

#[test]
fn test_xbox_wgs_snapshot_denial() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    // Creating snapshot targeting WGS path must be rejected with SnapshotWgsUnsupported
    let wgs_dir = temp_saves.path().join("SystemAppData").join("wgs");
    fs::create_dir_all(&wgs_dir).unwrap();

    let result = VersionedSnapshotManager::create_snapshot(
        "wgs",
        temp_saves.path().join("SystemAppData").as_path(),
        None,
        SnapshotReason::Manual,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::SnapshotWgsUnsupported => (),
        other => panic!("Expected SnapshotWgsUnsupported, got: {:?}", other),
    }
}

#[test]
fn test_interrupted_staging_snapshot_is_ignored_by_listing() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "StagingFarm_333";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    // 1. Create a valid snapshot
    let valid_snap = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        Some("Valid Snapshot".into()),
        SnapshotReason::Manual,
    )
    .unwrap();

    // 2. Simulate an interrupted/abandoned staging directory in the snapshots root
    let farm_snap_dir = temp_snapshots.path().join(farm_folder);
    let abandoned_staging = farm_snap_dir.join(".staging_interrupted_snap_id_9999");
    fs::create_dir_all(&abandoned_staging).unwrap();
    fs::write(abandoned_staging.join("partial_data.bin"), b"corrupted bytes").unwrap();

    // 3. List snapshots: MUST ignore .staging directory completely
    let snapshots = VersionedSnapshotManager::list_snapshots(Some(farm_folder)).unwrap();
    assert_eq!(snapshots.len(), 1, "Listing must only return the valid published snapshot");
    assert_eq!(snapshots[0].snapshot_id, valid_snap.snapshot_id);
    assert!(!snapshots.iter().any(|s| s.snapshot_id.contains(".staging")));
}

#[test]
fn test_invalid_manifest_tamper_detected() {
    let _lock = SNAPSHOT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_snapshots = tempdir().unwrap();
    std::env::set_var("STARDEW_SNAPSHOTS_DIR", temp_snapshots.path().to_str().unwrap());

    let farm_folder = "TamperedManifestFarm_444";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let snap = VersionedSnapshotManager::create_snapshot(
        farm_folder,
        temp_saves.path(),
        None,
        SnapshotReason::Manual,
    )
    .unwrap();

    // Tamper with the snapshot_manifest.json to make it invalid JSON
    let manifest_file = temp_snapshots
        .path()
        .join(farm_folder)
        .join(&snap.snapshot_id)
        .join("snapshot_manifest.json");
    fs::write(&manifest_file, "{ invalid_json: true").unwrap();

    // Verification must fail safely
    let verify_res = VersionedSnapshotManager::verify_integrity(&snap.snapshot_id);
    assert!(verify_res.is_err(), "Invalid manifest JSON must return an error");
}

