mod test_fixtures;

use sha2::{Digest, Sha256};
use stardew_sync_core::core::monitoring::{
    DetectionReason, FingerprintGenerator, ObservationJournalManager,
    SaveChangeClassification, SaveMonitoringService, SettleAndVerifyPipeline,
    SettleConfig, ValidationStatus,
};
use std::fs;
use std::sync::Mutex;
use std::time::Duration;
use tempfile::tempdir;
use test_fixtures::{generate_test_save_game_info_xml, generate_test_save_xml};

static MONITORING_TEST_MUTEX: Mutex<()> = Mutex::new(());

fn fast_test_settle_config() -> SettleConfig {
    SettleConfig {
        quiet_period: Duration::from_millis(30),
        max_retries: 3,
        retry_backoff: Duration::from_millis(15),
    }
}

fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:X}", hasher.finalize())
}

#[test]
fn test_stable_fingerprint_generation_and_initial_discovery() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "Emerald_100";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    let save_xml = generate_test_save_xml();
    let info_xml = generate_test_save_game_info_xml();
    fs::write(save_dir.join(farm_folder), &save_xml).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), &info_xml).unwrap();

    let fp = FingerprintGenerator::compute_fingerprint(
        farm_folder,
        &save_dir,
        "StandardAppdata",
    )
    .expect("Fingerprint generation must succeed");

    assert_eq!(fp.farm_id, farm_folder);
    assert_eq!(fp.files.len(), 2);
    assert!(!fp.aggregate_hash.is_empty());
    assert_eq!(fp.validation_status, ValidationStatus::Valid);

    // Verify canonical alphabetical sorting
    assert_eq!(fp.files[0].file_name, farm_folder);
    assert_eq!(fp.files[1].file_name, "SaveGameInfo");

    // In-game metadata safely extracted
    assert!(fp.in_game_metadata.is_some());
    let meta = fp.in_game_metadata.unwrap();
    assert_eq!(meta.host_name, Some("PlayerA".to_string()));
    assert_eq!(meta.character_count, 2);
}

#[test]
fn test_unchanged_save_classification() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "Emerald_100";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let config = fast_test_settle_config();

    // 1. Initial observation
    let (fp1, class1) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        None,
        &config,
    )
    .unwrap();

    assert_eq!(class1, SaveChangeClassification::LocalChangeDetected);

    // 2. Second observation with same hash -> Must classify as Unchanged
    let (fp2, class2) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        Some(&fp1.aggregate_hash),
        &config,
    )
    .unwrap();

    assert_eq!(class2, SaveChangeClassification::Unchanged);
    assert_eq!(fp1.aggregate_hash, fp2.aggregate_hash);
}

#[test]
fn test_changed_save_classification() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "Emerald_100";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    let save_xml_a = generate_test_save_xml();
    fs::write(save_dir.join(farm_folder), &save_xml_a).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let config = fast_test_settle_config();

    let (fp1, _) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        None,
        &config,
    )
    .unwrap();

    // Modify save: advance day or inventory
    let save_xml_b = save_xml_a.replace("<name>Parsnip Seeds</name>", "<name>Cauliflower Seeds</name>");
    fs::write(save_dir.join(farm_folder), &save_xml_b).unwrap();

    let (fp2, class2) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        Some(&fp1.aggregate_hash),
        &config,
    )
    .unwrap();

    assert_eq!(class2, SaveChangeClassification::LocalChangeDetected);
    assert_ne!(fp1.aggregate_hash, fp2.aggregate_hash);
}

#[test]
fn test_atomic_save_replacement_handling() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "Emerald_100";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let config = fast_test_settle_config();

    // Baseline
    let (fp_base, _) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        None,
        &config,
    )
    .unwrap();

    // Simulate in-game atomic write pattern:
    // 1. Rename existing save to _old
    let old_primary = save_dir.join(format!("{}_old", farm_folder));
    fs::rename(save_dir.join(farm_folder), &old_primary).unwrap();

    // 2. Write new save to temp buffer and atomically replace
    let temp_save = save_dir.join(".tmp_SaveGame");
    let new_xml = generate_test_save_xml().replace("<name>Parsnip Seeds</name>", "<name>Melon Seeds</name>");
    fs::write(&temp_save, new_xml).unwrap();
    fs::rename(&temp_save, save_dir.join(farm_folder)).unwrap();

    let (fp_new, class_new) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        Some(&fp_base.aggregate_hash),
        &config,
    )
    .unwrap();

    assert_eq!(class_new, SaveChangeClassification::LocalChangeDetected);
    assert_ne!(fp_base.aggregate_hash, fp_new.aggregate_hash);
    // Companion _old file is now tracked in canonical inventory
    assert!(fp_new.files.iter().any(|f| f.file_name.ends_with("_old")));
}

#[test]
fn test_malformed_xml_detection() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "CorruptFarm_666";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    // Write incomplete/corrupt XML tag
    fs::write(save_dir.join(farm_folder), "<SaveGame><corrupted_unclosed").unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let config = fast_test_settle_config();

    let (fp, class) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        None,
        &config,
    )
    .unwrap();

    assert_eq!(class, SaveChangeClassification::InvalidOrIncompleteSave);
    match fp.validation_status {
        ValidationStatus::MalformedXml(_) => (),
        other => panic!("Expected MalformedXml, got {:?}", other),
    }
}

#[test]
fn test_missing_companion_file_detection() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "MissingInfo_777";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    // Write primary save, but OMIT companion SaveGameInfo
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();

    let config = fast_test_settle_config();

    let (fp, class) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        None,
        &config,
    )
    .unwrap();

    assert_eq!(class, SaveChangeClassification::InvalidOrIncompleteSave);
    match fp.validation_status {
        ValidationStatus::MissingFiles(msg) => assert!(msg.contains("SaveGameInfo")),
        other => panic!("Expected MissingFiles, got {:?}", other),
    }
}

#[test]
fn test_directory_disappearance_and_recreation() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "GhostFarm_888";
    let save_dir = temp_saves.path().join(farm_folder);

    let config = fast_test_settle_config();

    // 1. Directory does not exist -> SourceMissing
    let (fp_missing, class_missing) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        None,
        &config,
    )
    .unwrap();

    assert_eq!(class_missing, SaveChangeClassification::SourceMissing);
    assert!(fp_missing.files.is_empty());

    // 2. Directory created with valid saves -> Detected
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let (_, class_recreated) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        false,
        None,
        &config,
    )
    .unwrap();

    assert_eq!(class_recreated, SaveChangeClassification::LocalChangeDetected);
}

#[test]
fn test_game_running_deferral() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "ActiveFarm_999";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();
    fs::write(save_dir.join(farm_folder), generate_test_save_xml()).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let config = fast_test_settle_config();

    // When game is running: pipeline MUST defer observation
    let (_, class) = SettleAndVerifyPipeline::execute(
        farm_folder,
        &save_dir,
        "StandardAppdata",
        true, // is_game_running = true
        None,
        &config,
    )
    .unwrap();

    assert_eq!(class, SaveChangeClassification::WaitingForGameClosure);
}

#[test]
fn test_observation_sequence_and_journal_persistence() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_monitoring = tempdir().unwrap();
    std::env::set_var("STARDEW_MONITORING_DIR", temp_monitoring.path().to_str().unwrap());

    let temp_saves = tempdir().unwrap();
    let farm_folder = "JournalFarm_001";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    let save_xml = generate_test_save_xml();
    fs::write(save_dir.join(farm_folder), &save_xml).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();

    let fp1 = FingerprintGenerator::compute_fingerprint(farm_folder, &save_dir, "StandardAppdata").unwrap();

    // 1. Initial entry: Sequence #1
    let entry1 = ObservationJournalManager::record_observation(
        farm_folder,
        &fp1,
        SaveChangeClassification::LocalChangeDetected,
        DetectionReason::InitialScan,
        false,
    )
    .unwrap();

    assert_eq!(entry1.sequence_number, 1);
    assert_eq!(entry1.current_fingerprint, fp1.aggregate_hash);
    assert_eq!(entry1.previous_fingerprint, None);

    // 2. Unchanged observation: Sequence remains #1, updates last_seen
    let entry1_repeat = ObservationJournalManager::record_observation(
        farm_folder,
        &fp1,
        SaveChangeClassification::LocalChangeDetected,
        DetectionReason::PeriodicReconcile,
        false,
    )
    .unwrap();

    assert_eq!(entry1_repeat.sequence_number, 1);
    let journal = ObservationJournalManager::load_journal(farm_folder).unwrap();
    assert_eq!(journal.len(), 1, "Unchanged observation should not inflate journal length");

    // 3. New state / change detected: Sequence monotonically increments to #2
    let new_save_xml = save_xml.replace("<name>Parsnip Seeds</name>", "<name>Ancient Seeds</name>");
    fs::write(save_dir.join(farm_folder), &new_save_xml).unwrap();

    let fp2 = FingerprintGenerator::compute_fingerprint(farm_folder, &save_dir, "StandardAppdata").unwrap();
    let entry2 = ObservationJournalManager::record_observation(
        farm_folder,
        &fp2,
        SaveChangeClassification::LocalChangeDetected,
        DetectionReason::FilesystemEvent,
        false,
    )
    .unwrap();

    assert_eq!(entry2.sequence_number, 2);
    assert_eq!(entry2.previous_fingerprint, Some(fp1.aggregate_hash));
    assert_eq!(entry2.current_fingerprint, fp2.aggregate_hash);

    let journal_final = ObservationJournalManager::load_journal(farm_folder).unwrap();
    assert_eq!(journal_final.len(), 2);
    assert_eq!(journal_final[0].sequence_number, 1);
    assert_eq!(journal_final[1].sequence_number, 2);
}

#[test]
fn test_path_traversal_and_security_rejections() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
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
        let fp_res = FingerprintGenerator::compute_fingerprint(
            name,
            &temp_saves.path().join(name),
            "StandardAppdata",
        );
        assert!(fp_res.is_err(), "Should reject malicious farm id in compute_fingerprint: {}", name);

        let journal_res = ObservationJournalManager::get_journal_file_path(name);
        assert!(journal_res.is_err(), "Should reject malicious farm id in journal: {}", name);
    }
}

#[test]
fn test_production_save_write_protection_during_monitoring() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_monitoring = tempdir().unwrap();
    std::env::set_var("STARDEW_MONITORING_DIR", temp_monitoring.path().to_str().unwrap());

    let farm_folder = "ProtectedMonitor_111";
    let save_dir = temp_saves.path().join(farm_folder);
    fs::create_dir_all(&save_dir).unwrap();

    let primary_bytes = generate_test_save_xml().into_bytes();
    let info_bytes = generate_test_save_game_info_xml().into_bytes();
    fs::write(save_dir.join(farm_folder), &primary_bytes).unwrap();
    fs::write(save_dir.join("SaveGameInfo"), &info_bytes).unwrap();

    let hash_primary_before = compute_sha256(&primary_bytes);
    let hash_info_before = compute_sha256(&info_bytes);

    let service = SaveMonitoringService::new(temp_saves.path().to_path_buf());
    service.set_settle_config(fast_test_settle_config());

    // Execute multiple monitoring checks
    let _status1 = service.check_farm_now(farm_folder, DetectionReason::InitialScan).unwrap();
    let _status2 = service.check_farm_now(farm_folder, DetectionReason::ManualCheck).unwrap();
    let _reconcile = service.reconcile_all(DetectionReason::PeriodicReconcile);

    // Verify monitored source files remain 100% UNMODIFIED byte-for-byte
    let primary_after = fs::read(save_dir.join(farm_folder)).unwrap();
    let info_after = fs::read(save_dir.join("SaveGameInfo")).unwrap();

    assert_eq!(compute_sha256(&primary_after), hash_primary_before);
    assert_eq!(compute_sha256(&info_after), hash_info_before);
    assert_eq!(primary_after, primary_bytes);
    assert_eq!(info_after, info_bytes);
}

#[test]
fn test_service_reconciliation_and_toggle() {
    let _lock = MONITORING_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let temp_saves = tempdir().unwrap();
    let temp_monitoring = tempdir().unwrap();
    std::env::set_var("STARDEW_MONITORING_DIR", temp_monitoring.path().to_str().unwrap());

    // Setup 2 farms
    let farm1 = "FarmOne_111";
    let farm2 = "FarmTwo_222";
    for f in &[farm1, farm2] {
        let dir = temp_saves.path().join(f);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(f), generate_test_save_xml()).unwrap();
        fs::write(dir.join("SaveGameInfo"), generate_test_save_game_info_xml()).unwrap();
    }

    let service = SaveMonitoringService::new(temp_saves.path().to_path_buf());
    service.set_settle_config(fast_test_settle_config());

    // Reconcile all
    let results = service.reconcile_all(DetectionReason::InitialScan);
    assert_eq!(results.len(), 2);

    let s1 = service.get_status(farm1).expect("FarmOne status must exist");
    assert_eq!(s1.classification, SaveChangeClassification::LocalChangeDetected);

    // Toggle service disabled
    service.set_enabled(false);
    assert!(!service.is_enabled());

    let check_when_disabled = service.check_farm_now(farm1, DetectionReason::ManualCheck);
    assert!(check_when_disabled.is_err(), "Check must be rejected when service is disabled");

    // Toggle service re-enabled
    service.set_enabled(true);
    assert!(service.is_enabled());
    let check_when_enabled = service.check_farm_now(farm1, DetectionReason::ManualCheck);
    assert!(check_when_enabled.is_ok());
}
