use stardew_sync_core::commands::*;
use xmltree::Element;

static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_get_app_status_structure() {
    let status = get_app_status().expect("get_app_status should succeed");
    assert_eq!(status.app_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(status.author, "bazq");
    assert!(!status.platform.is_empty());
    assert!(!status.arch.is_empty());
    assert!(status.saves_dir.contains("StardewValley"));
    assert!(status.sync_status.contains("Phase 5"));
}

#[test]
fn test_get_process_status_structure() {
    let status = get_process_status().expect("get_process_status should succeed");
    assert!(status.process_names_checked.contains(&"Stardew Valley.exe".to_string()));
    assert!(status.process_names_checked.contains(&"StardewModdingAPI.exe".to_string()));
    assert_eq!(status.can_safely_operate, !status.is_stardew_running);
}

#[test]
fn test_farm_metadata_rejects_path_traversal() {
    let traversal_attempts = vec![
        "../AnyFarm",
        "..\\AnyFarm",
        "../TXrk_450560341",
        "..\\TXrk_450560341",
        "sub/folder",
        "sub\\folder",
        "../../Windows",
    ];

    for attempt in traversal_attempts {
        let result = get_farm_metadata(attempt.to_string());
        assert!(
            result.is_err(),
            "Path traversal attempt '{}' should be rejected",
            attempt
        );
        let err = result.unwrap_err();
        assert!(err.contains("Security Violation"));
    }
}

#[test]
fn test_snapshot_integrity_rejects_path_traversal() {
    let traversal_attempts = vec![
        "../snapshot_hack",
        "..\\snapshot_hack",
        "dir/name",
        "dir\\name",
    ];

    for attempt in traversal_attempts {
        let result = verify_snapshot_integrity(attempt.to_string());
        assert!(
            result.is_err(),
            "Snapshot traversal attempt '{}' should be rejected",
            attempt
        );
        let err = result.unwrap_err();
        assert!(err.contains("Security Violation"));
    }
}

#[test]
fn test_check_for_updates_endpoint() {
    let update_info = check_for_updates().expect("check_for_updates should succeed");
    assert_eq!(update_info.current_version, env!("CARGO_PKG_VERSION"));
    assert!(update_info.endpoint.contains("github.com/bazq3D/stardew-sync"));
    assert!(update_info.public_key_configured);
}

#[test]
fn test_discover_farms_returns_vector_and_verifies_dynamic_fields() {
    let farms = discover_farms().expect("discover_farms should succeed");
    for farm in &farms {
        // Every farm is protected by generic default-deny model
        assert!(farm.is_protected, "All discovered farms must have is_protected = true");

        // If this is the developer's legacy production farm, verify accurate dynamic date parsing
        if farm.folder_name == "TXrk_450560341" {
            assert!(farm.is_legacy_production);
            assert_eq!(farm.farm_name, "Türk", "Farm name must be parsed from SaveGameInfo");
            assert_eq!(farm.host_name, "Kubilay");
            assert!(
                farm.date_summary.contains("Winter") || farm.date_summary.contains("Fall"),
                "Production farm date must be parsed accurately, got '{}'",
                farm.date_summary
            );
        }
    }
}

#[test]
fn test_list_snapshots_returns_vector() {
    let snapshots = list_snapshots().expect("list_snapshots should succeed");
    for snap in &snapshots {
        assert!(!snap.id.is_empty());
        assert!(!snap.path.is_empty());
    }
}

// =========================================================================
// Task C Regression Tests: Date Parsing & Season Transitions
// =========================================================================

#[test]
fn test_date_parsing_regression_spring_integer_and_string() {
    let xml_int = r#"<Farmer><seasonForSaveGame>0</seasonForSaveGame><dayOfMonthForSaveGame>5</dayOfMonthForSaveGame><yearForSaveGame>1</yearForSaveGame></Farmer>"#;
    let elem_int = Element::parse(xml_int.as_bytes()).unwrap();
    let (season, day, year, summary) = extract_date_from_xml_element(&elem_int);
    assert_eq!(season, "Spring");
    assert_eq!(day, 5);
    assert_eq!(year, 1);
    assert_eq!(summary, "Spring, Day 5 (Year 1)");

    let xml_str = r#"<Farmer><seasonForSaveGame>spring</seasonForSaveGame><dayOfMonthForSaveGame>1</dayOfMonthForSaveGame><yearForSaveGame>2</yearForSaveGame></Farmer>"#;
    let elem_str = Element::parse(xml_str.as_bytes()).unwrap();
    let (season, day, year, summary) = extract_date_from_xml_element(&elem_str);
    assert_eq!(season, "Spring");
    assert_eq!(day, 1);
    assert_eq!(year, 2);
    assert_eq!(summary, "Spring, Day 1 (Year 2)");
}

#[test]
fn test_date_parsing_regression_summer() {
    let xml = r#"<Farmer><seasonForSaveGame>1</seasonForSaveGame><dayOfMonthForSaveGame>21</dayOfMonthForSaveGame><yearForSaveGame>1</yearForSaveGame></Farmer>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let (season, day, year, summary) = extract_date_from_xml_element(&elem);
    assert_eq!(season, "Summer");
    assert_eq!(day, 21);
    assert_eq!(year, 1);
    assert_eq!(summary, "Summer, Day 21 (Year 1)");
}

#[test]
fn test_date_parsing_regression_fall_production_format() {
    // Exact format from live Stardew 1.6 SaveGameInfo (seasonForSaveGame = 2)
    let xml = r#"<Farmer><seasonForSaveGame>2</seasonForSaveGame><dayOfMonthForSaveGame>23</dayOfMonthForSaveGame><yearForSaveGame>1</yearForSaveGame></Farmer>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let (season, day, year, summary) = extract_date_from_xml_element(&elem);
    assert_eq!(season, "Fall");
    assert_eq!(day, 23);
    assert_eq!(year, 1);
    assert_eq!(summary, "Fall, Day 23 (Year 1)");
}

#[test]
fn test_date_parsing_regression_winter() {
    let xml = r#"<Farmer><seasonForSaveGame>3</seasonForSaveGame><dayOfMonthForSaveGame>28</dayOfMonthForSaveGame><yearForSaveGame>3</yearForSaveGame></Farmer>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let (season, day, year, summary) = extract_date_from_xml_element(&elem);
    assert_eq!(season, "Winter");
    assert_eq!(day, 28);
    assert_eq!(year, 3);
    assert_eq!(summary, "Winter, Day 28 (Year 3)");
}

#[test]
fn test_date_parsing_regression_year_transitions() {
    let xml = r#"<SaveGame><currentSeason>winter</currentSeason><dayOfMonth>28</dayOfMonth><year>10</year></SaveGame>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let (season, day, year, summary) = extract_date_from_xml_element(&elem);
    assert_eq!(season, "Winter");
    assert_eq!(day, 28);
    assert_eq!(year, 10);
    assert_eq!(summary, "Winter, Day 28 (Year 10)");
}

#[test]
fn test_date_parsing_regression_missing_fields_graceful_fallback() {
    let xml = r#"<Farmer><name>LonelyFarmer</name></Farmer>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let (season, day, year, summary) = extract_date_from_xml_element(&elem);
    assert_eq!(season, "Spring");
    assert_eq!(day, 1);
    assert_eq!(year, 1);
    assert_eq!(summary, "Spring, Day 1 (Year 1)");
}

#[test]
fn test_update_eligibility_independent_of_game_running() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    // 1. When game is NOT running and no save operation is active -> can update
    let res1 = check_update_eligibility(false, false);
    assert!(res1.can_update);
    assert!(!res1.is_game_running);
    assert!(!res1.is_save_operation_active);

    // 2. When game IS running and no save operation is active -> STILL can update (decoupled!)
    let res2 = check_update_eligibility(true, false);
    assert!(
        res2.can_update,
        "App updates must be allowed while Stardew Valley is running"
    );
    assert!(res2.is_game_running);
    assert!(!res2.is_save_operation_active);
    assert!(res2.reason.contains("never touches game saves"));

    // 3. When an active save operation IS in progress -> updates must be paused
    let res3 = check_update_eligibility(false, true);
    assert!(
        !res3.can_update,
        "Updates must pause during active save operations"
    );
    assert!(res3.is_save_operation_active);
    assert!(res3.reason.contains("paused to protect active operations"));

    // 4. When both game is running and save operation is active -> cannot update due to save op
    let res4 = check_update_eligibility(true, true);
    assert!(!res4.can_update);
    assert!(res4.is_save_operation_active);
}

#[test]
fn test_backend_authoritative_active_operation_guard() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();
    assert!(!ActiveOperationGuard::is_active());

    // Acquire the backend guard
    {
        let guard = ActiveOperationGuard::acquire().expect("Should acquire active operation guard");
        assert!(ActiveOperationGuard::is_active());

        // Even if frontend claims `false`, backend authoritative guard forces `can_update: false`
        let res = check_update_eligibility(false, false);
        assert!(!res.can_update, "Backend operation guard must block updates even if frontend says false");
        assert!(res.is_save_operation_active);
        assert!(res.reason.contains("paused to protect active operations"));

        // Second acquire attempt should fail while first is held
        let second = ActiveOperationGuard::acquire();
        assert!(second.is_err(), "Concurrent save operations must be rejected");

        drop(guard);
    }

    // After guard is dropped, lock is released
    assert!(!ActiveOperationGuard::is_active());
    let res_after = check_update_eligibility(false, false);
    assert!(res_after.can_update, "Updates must be re-enabled after save operation completes");
}

#[test]
fn test_updater_reservation_mutual_exclusion_with_save_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    // 1. Acquire update reservation using RAII guard
    {
        let update_guard = ActiveUpdateGuard::acquire_guard().expect("Should acquire update guard");
        assert!(ActiveUpdateGuard::is_active());

        // While update is reserved, save operations MUST be rejected
        let save_op = ActiveOperationGuard::acquire();
        assert!(save_op.is_err(), "Save operations must be blocked during update installation");
        assert!(save_op.unwrap_err().contains("update installation is in progress"));

        // Dropping update guard releases the reservation
        drop(update_guard);
    }
    assert!(!ActiveUpdateGuard::is_active());

    // 2. Now save operations can proceed
    let save_op = ActiveOperationGuard::acquire();
    assert!(save_op.is_ok(), "Save operations can proceed after update completes");

    // While save operation is running, update reservation MUST be rejected
    let update_res = ActiveUpdateGuard::acquire();
    assert!(update_res.is_err(), "Update reservation must be rejected during active save operation");
    assert!(update_res.unwrap_err().contains("save-critical operation is in progress"));

    drop(save_op);
    assert!(!ActiveOperationGuard::is_active());
}

#[test]
fn test_commands_updater_reservation_lifecycle_and_owner_release() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    // 1. Acquire reservation command returns unguessable token
    let res1 = acquire_update_reservation();
    assert!(res1.is_ok(), "acquire_update_reservation should succeed when idle");
    let token = res1.unwrap();
    assert!(!token.trim().is_empty(), "Token must not be empty");
    assert!(ActiveUpdateGuard::is_active());

    // 2. Second concurrent call fails immediately
    let res2 = acquire_update_reservation();
    assert!(res2.is_err(), "Concurrent update reservation must fail");
    assert!(res2.unwrap_err().contains("already in progress"));

    // 3. Release reservation command with matching owner token succeeds
    let res3 = release_update_reservation(token.clone());
    assert_eq!(res3, Ok(true));
    assert!(!ActiveUpdateGuard::is_active());

    // 4. Can acquire again with fresh new token
    let res4 = acquire_update_reservation();
    assert!(res4.is_ok());
    let token2 = res4.unwrap();
    assert_ne!(token, token2, "Subsequent reservation must receive a distinct token");
    let _ = release_update_reservation(token2);
    assert!(!ActiveUpdateGuard::is_active());
}

#[test]
fn test_incorrect_token_release_rejected() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    let token = acquire_update_reservation().expect("Should acquire reservation");
    assert!(ActiveUpdateGuard::is_active());

    // Releasing with an incorrect token must fail
    let rel_wrong = release_update_reservation("invalid-token-xyz".to_string());
    assert!(rel_wrong.is_err(), "Incorrect token release must fail");
    assert!(rel_wrong.unwrap_err().contains("mismatch"));

    // The reservation MUST remain active and protected!
    assert!(ActiveUpdateGuard::is_active(), "Reservation must remain active after bad token attempt");
    let save_attempt = ActiveOperationGuard::acquire();
    assert!(save_attempt.is_err(), "Save operations must remain blocked");

    // Releasing with empty or whitespace token must fail
    let rel_empty = release_update_reservation("   ".to_string());
    assert!(rel_empty.is_err());
    assert!(ActiveUpdateGuard::is_active());

    // Genuine owner releases successfully
    let rel_correct = release_update_reservation(token);
    assert_eq!(rel_correct, Ok(true));
    assert!(!ActiveUpdateGuard::is_active());
}

#[test]
fn test_stale_token_cannot_release_new_reservation() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    // 1. Old operation acquires and releases
    let token_old = acquire_update_reservation().expect("Old reservation should succeed");
    assert_eq!(release_update_reservation(token_old.clone()), Ok(true));
    assert!(!ActiveUpdateGuard::is_active());

    // 2. New operation acquires reservation
    let token_new = acquire_update_reservation().expect("New reservation should succeed");
    assert!(ActiveUpdateGuard::is_active());

    // 3. Delayed/stale release from old operation arrives -> MUST be rejected!
    let stale_rel = release_update_reservation(token_old);
    assert!(stale_rel.is_err(), "Stale token from prior operation must be rejected");
    assert!(stale_rel.unwrap_err().contains("mismatch"));

    // The new reservation MUST still be active and protected!
    assert!(ActiveUpdateGuard::is_active(), "New reservation must remain intact after stale release attempt");

    // Clean up with new token
    assert_eq!(release_update_reservation(token_new), Ok(true));
    assert!(!ActiveUpdateGuard::is_active());
}

#[test]
fn test_duplicate_release_fails_safely() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    let token = acquire_update_reservation().expect("Should acquire");
    // First release succeeds
    assert_eq!(release_update_reservation(token.clone()), Ok(true));
    assert!(!ActiveUpdateGuard::is_active());

    // Duplicate release fails safely without corrupting idle state
    let dup_rel = release_update_reservation(token);
    assert!(dup_rel.is_err());
    assert!(!ActiveUpdateGuard::is_active());

    // Save operations can proceed normally
    let save_guard = ActiveOperationGuard::acquire();
    assert!(save_guard.is_ok());
    drop(save_guard);
}

#[test]
fn test_targeted_release_does_not_clear_active_save_operation() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    // Acquire save operation
    let save_guard = ActiveOperationGuard::acquire().expect("Should acquire save guard");
    assert!(ActiveOperationGuard::is_active());

    // Calling release_update_reservation while in save state must error and NOT release save op!
    let rel = release_update_reservation("some-random-token".to_string());
    assert!(rel.is_err(), "Release update must error when no update reservation is active");
    assert!(ActiveOperationGuard::is_active(), "Save operation must remain active");

    drop(save_guard);
    assert!(!ActiveOperationGuard::is_active());
}

#[test]
fn test_long_running_update_reservation_remains_protected() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    let token = acquire_update_reservation().expect("Should acquire reservation");

    // Even if checked repeatedly or active for a long duration, no auto-eviction occurs!
    for _ in 0..100 {
        assert!(ActiveUpdateGuard::is_active(), "Update reservation must never auto-expire");
    }

    // Save operation is strictly blocked
    assert!(ActiveOperationGuard::acquire().is_err());

    // Released cleanly by owner
    assert_eq!(release_update_reservation(token), Ok(true));
    assert!(!ActiveUpdateGuard::is_active());
}

#[test]
fn test_recovery_behavior_fail_closed_after_interruption() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();
    ActiveUpdateGuard::reset_for_test();

    // Simulate update reservation acquired before an unexpected webview crash/reload
    let _token = acquire_update_reservation().expect("Acquire");
    assert!(ActiveUpdateGuard::is_active());

    // If frontend crashes without calling release: save operations remain blocked (fail-closed!)
    let save_attempt = ActiveOperationGuard::acquire();
    assert!(save_attempt.is_err());
    let err_msg = save_attempt.unwrap_err();
    assert!(err_msg.contains("restart Stardew Sync"), "Fail-closed message instructs user to restart if interrupted");

    // On simulated application restart: clean idle state is re-established
    ActiveOperationGuard::reset_for_test();
    assert!(!ActiveUpdateGuard::is_active());
    assert!(!ActiveOperationGuard::is_active());

    let clean_save = ActiveOperationGuard::acquire();
    assert!(clean_save.is_ok(), "Post-restart state allows normal operations");
    drop(clean_save);
}

#[test]
fn test_playtime_extraction_realistic_primary_save() {
    let xml = r#"<SaveGame><player><name>Kubilay</name><millisecondsPlayed>36803712</millisecondsPlayed></player></SaveGame>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let ms = extract_milliseconds_played(&elem);
    assert_eq!(ms, Some(36803712));
    let hours = ms.map(ms_to_hours).unwrap();
    assert!((hours - 10.223253).abs() < 0.001);
    let summary = format_playtime_summary(ms);
    assert_eq!(summary, "10h 13m (10.2 hours)");
}

#[test]
fn test_playtime_extraction_savegameinfo_root() {
    let xml = r#"<Farmer><name>Kubilay</name><millisecondsPlayed>68062848</millisecondsPlayed></Farmer>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let ms = extract_milliseconds_played(&elem);
    assert_eq!(ms, Some(68062848));
    let hours = ms.map(ms_to_hours).unwrap();
    assert!((hours - 18.906346).abs() < 0.001);
    let summary = format_playtime_summary(ms);
    assert_eq!(summary, "18h 54m (18.9 hours)");
}

#[test]
fn test_playtime_extraction_missing_tag_returns_none() {
    let xml = r#"<SaveGame><player><name>Kubilay</name></player></SaveGame>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let ms = extract_milliseconds_played(&elem);
    assert_eq!(ms, None);
    let summary = format_playtime_summary(ms);
    assert_eq!(summary, "Unknown");
}

#[test]
fn test_playtime_extraction_corrupt_non_numeric() {
    let xml = r#"<SaveGame><player><name>Kubilay</name><millisecondsPlayed>invalid_time</millisecondsPlayed></player></SaveGame>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let ms = extract_milliseconds_played(&elem);
    assert_eq!(ms, None);
    let summary = format_playtime_summary(ms);
    assert_eq!(summary, "Unknown");
}

#[test]
fn test_playtime_extraction_zero_value() {
    let xml = r#"<SaveGame><player><name>NewFarmer</name><millisecondsPlayed>0</millisecondsPlayed></player></SaveGame>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let ms = extract_milliseconds_played(&elem);
    assert_eq!(ms, Some(0));
    assert_eq!(ms_to_hours(0), 0.0);
    assert_eq!(format_playtime_summary(ms), "0h 0m (0.0 hours)");
}

#[test]
fn test_playtime_conversion_boundaries() {
    // Exactly 1 hour
    assert_eq!(ms_to_hours(3_600_000), 1.0);
    assert_eq!(format_playtime_summary(Some(3_600_000)), "1h 0m (1.0 hours)");

    // Exactly 30 minutes
    assert_eq!(ms_to_hours(1_800_000), 0.5);
    assert_eq!(format_playtime_summary(Some(1_800_000)), "0h 30m (0.5 hours)");

    // 2.5 hours
    assert_eq!(ms_to_hours(9_000_000), 2.5);
    assert_eq!(format_playtime_summary(Some(9_000_000)), "2h 30m (2.5 hours)");
}

#[test]
fn test_app_settings_default_and_roundtrip() {
    use stardew_sync_core::core::settings::{AppSettings, SettingsManager};

    let defaults = AppSettings::default();
    assert!(!defaults.autostart_enabled, "Autostart must be disabled by default");
    assert!(!defaults.start_minimized, "Start minimized must be disabled by default");
    assert!(!defaults.minimize_to_tray_on_close, "Minimize to tray on close must be disabled by default");
    assert!(defaults.auto_check_updates, "Auto check updates should be enabled by default");
    assert_eq!(defaults.last_update_check, None);
    assert_eq!(defaults.last_notified_version, None);

    // Test serialization roundtrip via isolated temp directory
    let temp_dir = tempfile::tempdir().expect("tempdir creation should succeed");
    let test_file = temp_dir.path().join("test_settings.json");

    let custom_settings = AppSettings {
        autostart_enabled: true,
        start_minimized: true,
        minimize_to_tray_on_close: true,
        auto_check_updates: false,
        last_update_check: Some("2026-10-10T00:00:00Z".to_string()),
        last_notified_version: Some("0.2.0".to_string()),
    };

    SettingsManager::save_to_path(&test_file, &custom_settings).expect("save_to_path should succeed");
    assert!(test_file.exists());

    let loaded = SettingsManager::load_from_path(&test_file);
    assert_eq!(loaded, custom_settings);
}

#[test]
fn test_app_settings_cache_and_update_lifecycle() {
    let _lock = TEST_MUTEX.lock().unwrap();
    use stardew_sync_core::core::settings::SettingsManager;

    let mut current = get_app_settings().expect("get_app_settings should succeed");
    current.minimize_to_tray_on_close = !current.minimize_to_tray_on_close;

    let saved = save_app_settings(current.clone()).expect("save_app_settings should succeed");
    assert_eq!(saved.minimize_to_tray_on_close, current.minimize_to_tray_on_close);

    let retrieved = get_app_settings().expect("get_app_settings should succeed");
    assert_eq!(retrieved.minimize_to_tray_on_close, current.minimize_to_tray_on_close);

    // Test record_update_check
    let test_time = "2026-10-10T12:34:56Z".to_string();
    let test_ver = "0.2.0-rc1".to_string();
    SettingsManager::record_update_check(test_time.clone(), Some(test_ver.clone()));

    let updated = get_app_settings().expect("get_app_settings should succeed");
    assert_eq!(updated.last_update_check, Some(test_time));
    assert_eq!(updated.last_notified_version, Some(test_ver));
}

#[test]
fn test_background_update_poller_invariants() {
    let _lock = TEST_MUTEX.lock().unwrap();
    use stardew_sync_core::core::updater::BackgroundUpdatePoller;

    let status = BackgroundUpdatePoller::get_latest_status();
    assert_eq!(status.current_version, env!("CARGO_PKG_VERSION"));
    assert!(!status.is_checking);

    // Invariant: Updater inspection never acquires or holds an ActiveUpdateGuard reservation
    assert!(
        !ActiveUpdateGuard::is_active(),
        "Checking update status must not acquire exclusive update reservation"
    );
}

#[test]
fn test_tray_quit_guard_invariants() {
    let _lock = TEST_MUTEX.lock().unwrap();
    ActiveOperationGuard::reset_for_test();

    // Invariant 1: Idle state allows safe quit
    assert!(!ActiveUpdateGuard::is_active() && !ActiveOperationGuard::is_active());

    // Invariant 2: Active update reservation blocks tray quit
    let update_token = ActiveUpdateGuard::acquire().expect("acquire update guard should succeed");
    let is_busy_with_update = ActiveUpdateGuard::is_active() || ActiveOperationGuard::is_active();
    assert!(is_busy_with_update, "Active update reservation must report busy to tray quit guard");

    ActiveUpdateGuard::release_with_token(&update_token).expect("release update guard should succeed");
    assert!(!ActiveUpdateGuard::is_active());

    // Invariant 3: Active save operation blocks tray quit
    let save_guard = ActiveOperationGuard::acquire().expect("acquire save guard should succeed");
    let is_busy_with_save = ActiveUpdateGuard::is_active() || ActiveOperationGuard::is_active();
    assert!(is_busy_with_save, "Active save operation must report busy to tray quit guard");

    drop(save_guard);
    assert!(!ActiveOperationGuard::is_active());
}

#[test]
fn test_character_extraction_from_host_and_cabin_farmhand() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <player>
    <name>PlayerA</name>
    <UniqueMultiplayerID>1000000000001</UniqueMultiplayerID>
    <homeLocation>FarmHouse</homeLocation>
    <houseUpgradeLevel>2</houseUpgradeLevel>
    <farmName>Emerald</farmName>
    <isMale>true</isMale>
    <money>45000</money>
    <totalMoneyEarned>120000</totalMoneyEarned>
    <millisecondsPlayed>72000000</millisecondsPlayed>
    <maxHealth>120</maxHealth>
    <maxStamina>320</maxStamina>
    <maxItems>24</maxItems>
    <items>
      <Item xsi:type="Tool">
        <name>Iridium Pickaxe</name>
        <upgradeLevel>4</upgradeLevel>
      </Item>
      <Item xsi:type="Object">
        <name>Parsnip Seeds</name>
        <stack>50</stack>
        <quality>0</quality>
      </Item>
    </items>
    <experiencePoints>
      <int>1500</int>
      <int>800</int>
      <int>0</int>
      <int>0</int>
      <int>0</int>
    </experiencePoints>
    <friendshipData>
      <item>
        <key><string>Abigail</string></key>
        <value><Friendship><Points>2500</Points><Status>Married</Status></Friendship></value>
      </item>
    </friendshipData>
    <spouse>Abigail</spouse>
  </player>
  <locations>
    <GameLocation xsi:type="Farm">
      <name>Farm</name>
      <buildings>
        <Building>
          <buildingType>Stone Cabin</buildingType>
          <tileX>42</tileX>
          <tileY>18</tileY>
          <indoors xsi:type="Cabin">
            <uniqueName>Cabin_stone_1</uniqueName>
            <farmhand>
              <name>PlayerB</name>
              <UniqueMultiplayerID>2000000000002</UniqueMultiplayerID>
              <homeLocation>Cabin_stone_1</homeLocation>
              <farmName>Emerald</farmName>
              <isMale>false</isMale>
              <money>5000</money>
              <totalMoneyEarned>15000</totalMoneyEarned>
              <maxHealth>100</maxHealth>
              <maxStamina>270</maxStamina>
              <maxItems>12</maxItems>
              <items>
                <Item xsi:type="Tool">
                  <name>Fiberglass Rod</name>
                  <upgradeLevel>2</upgradeLevel>
                </Item>
              </items>
              <experiencePoints>
                <int>0</int>
                <int>0</int>
                <int>2200</int>
                <int>0</int>
                <int>0</int>
              </experiencePoints>
              <friendshipData>
                <item>
                  <key><string>Leah</string></key>
                  <value><Friendship><Points>2000</Points><Status>Friendly</Status></Friendship></value>
                </item>
              </friendshipData>
            </farmhand>
          </indoors>
        </Building>
      </buildings>
    </GameLocation>
  </locations>
</SaveGame>"#;

    let elem = Element::parse(xml.as_bytes()).expect("XML should parse cleanly");
    let characters = stardew_sync_core::core::character::extract_all_characters_from_save(&elem, "Emerald_100");
    assert_eq!(characters.len(), 2, "Must extract both host and farmhand");

    // 1. Verify Host (PlayerA)
    let host = characters.iter().find(|c| c.is_host).expect("Host must exist");
    assert_eq!(host.name, "PlayerA");
    assert_eq!(host.unique_multiplayer_id, 1000000000001);
    assert_eq!(host.farm_name, "Emerald");
    assert_eq!(host.gender, "Male");
    assert_eq!(host.money, 45000);
    assert_eq!(host.total_money_earned, 120000);
    assert_eq!(host.max_health, 120);
    assert_eq!(host.max_stamina, 320);
    assert_eq!(host.backpack_capacity, 24);
    assert_eq!(host.skills.farming, 4); // 1500 exp => lvl 4
    assert_eq!(host.skills.fishing, 3); // 800 exp => lvl 3
    assert_eq!(host.items.len(), 2);
    assert_eq!(host.items[0].name, "Iridium Pickaxe");
    assert_eq!(host.items[1].name, "Parsnip Seeds");
    assert_eq!(host.items[1].stack, 50);
    assert_eq!(host.spouse, Some("Abigail".to_string()));
    assert_eq!(host.friendships.len(), 1);
    assert_eq!(host.friendships[0].npc_name, "Abigail");
    assert_eq!(host.friendships[0].hearts, 10);
    assert_eq!(host.friendships[0].status, Some("Married".to_string()));
    assert_eq!(host.play_time_formatted, "20h 0m (20.0 hours)");

    // 2. Verify Farmhand (PlayerB)
    let farmhand = characters.iter().find(|c| !c.is_host).expect("Farmhand must exist");
    assert_eq!(farmhand.name, "PlayerB");
    assert_eq!(farmhand.unique_multiplayer_id, 2000000000002);
    assert_eq!(farmhand.cabin_name, Some("Stone Cabin".to_string()));
    assert_eq!(farmhand.raw_cabin_id, Some("Cabin_stone_1".to_string()));
    assert_eq!(farmhand.gender, "Female");
    assert_eq!(farmhand.skills.foraging, 5); // 2200 exp => lvl 5
    assert_eq!(farmhand.items.len(), 1);
    assert_eq!(farmhand.items[0].name, "Fiberglass Rod");
    assert_eq!(farmhand.friendships.len(), 1);
    assert_eq!(farmhand.friendships[0].npc_name, "Leah");
    assert_eq!(farmhand.friendships[0].hearts, 8);
}

#[test]
fn test_character_extraction_stardew_1_6_root_farmhands() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <player>
    <name>RootHost</name>
    <UniqueMultiplayerID>999111</UniqueMultiplayerID>
  </player>
  <farmhands>
    <Farmer>
      <name>RootFarmhand</name>
      <UniqueMultiplayerID>999222</UniqueMultiplayerID>
      <farmingLevel>8</farmingLevel>
      <miningLevel>7</miningLevel>
      <foragingLevel>6</foragingLevel>
      <fishingLevel>5</fishingLevel>
      <combatLevel>4</combatLevel>
    </Farmer>
  </farmhands>
</SaveGame>"#;

    let elem = Element::parse(xml.as_bytes()).unwrap();
    let characters = stardew_sync_core::core::character::extract_all_characters_from_save(&elem, "TestFarm_100");
    assert_eq!(characters.len(), 2);

    let farmhand = characters.iter().find(|c| !c.is_host).unwrap();
    assert_eq!(farmhand.name, "RootFarmhand");
    assert_eq!(farmhand.unique_multiplayer_id, 999222);
    assert_eq!(farmhand.skills.farming, 8);
    assert_eq!(farmhand.skills.mining, 7);
    assert_eq!(farmhand.skills.foraging, 6);
    assert_eq!(farmhand.skills.fishing, 5);
    assert_eq!(farmhand.skills.combat, 4);
}

#[test]
fn test_character_extraction_missing_tags_graceful_fallbacks() {
    let xml = r#"<SaveGame><player><name>MinimalFarmer</name></player></SaveGame>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let characters = stardew_sync_core::core::character::extract_all_characters_from_save(&elem, "Minimal_001");
    assert_eq!(characters.len(), 1);

    let character = &characters[0];
    assert_eq!(character.name, "MinimalFarmer");
    assert_eq!(character.unique_multiplayer_id, 0);
    assert_eq!(character.max_health, 100);
    assert_eq!(character.max_stamina, 270);
    assert_eq!(character.money, 0);
    assert_eq!(character.backpack_capacity, 12);
    assert!(character.items.is_empty());
    assert!(character.friendships.is_empty());
    assert_eq!(character.spouse, None);
    assert_eq!(character.play_time_formatted, "Unknown");
    assert_eq!(character.gender, "Unknown");
}

#[test]
fn test_get_farm_characters_path_traversal_rejection() {
    let bad_paths = vec!["../Secret", "..\\Secret", "sub/folder", "sub\\folder"];
    for path in bad_paths {
        let res = get_farm_characters(path.to_string());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Security Violation"));
    }
}

#[test]
fn test_gender_extraction_regression_male_female_missing_malformed() {
    use stardew_sync_core::core::character::extract_gender;

    // Case 1: Standard male
    let xml_male = r#"<player><name>Bob</name><isMale>true</isMale></player>"#;
    let elem_male = Element::parse(xml_male.as_bytes()).unwrap();
    assert_eq!(extract_gender(&elem_male), "Male");

    // Case 2: Standard female
    let xml_female = r#"<player><name>Alice</name><isMale>false</isMale></player>"#;
    let elem_female = Element::parse(xml_female.as_bytes()).unwrap();
    assert_eq!(extract_gender(&elem_female), "Female");

    // Case 3: Stardew 1.6+ explicit <Gender>Female</Gender>
    let xml_16_female = r#"<player><name>Penny</name><Gender>Female</Gender><isMale xsi:nil="true" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"/></player>"#;
    let elem_16_female = Element::parse(xml_16_female.as_bytes()).unwrap();
    assert_eq!(extract_gender(&elem_16_female), "Female");

    // Case 4: Stardew 1.6+ explicit <Gender>Male</Gender>
    let xml_16_male = r#"<player><name>Sam</name><Gender>Male</Gender><isMale xsi:nil="true" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"/></player>"#;
    let elem_16_male = Element::parse(xml_16_male.as_bytes()).unwrap();
    assert_eq!(extract_gender(&elem_16_male), "Male");

    // Case 5: Missing gender tags entirely -> MUST NOT default to Male
    let xml_missing = r#"<player><name>Ghost</name></player>"#;
    let elem_missing = Element::parse(xml_missing.as_bytes()).unwrap();
    assert_eq!(extract_gender(&elem_missing), "Unknown");

    // Case 6: Malformed / non-boolean isMale tag -> MUST NOT default to Male
    let xml_malformed = r#"<player><name>Corrupt</name><isMale>not_a_boolean</isMale></player>"#;
    let elem_malformed = Element::parse(xml_malformed.as_bytes()).unwrap();
    assert_eq!(extract_gender(&elem_malformed), "Unknown");

    // Case 7: isMale with xsi:nil="true" but without Gender tag -> MUST NOT default to Male
    let xml_nil = r#"<player><name>NilFarmer</name><isMale xsi:nil="true" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"/></player>"#;
    let elem_nil = Element::parse(xml_nil.as_bytes()).unwrap();
    assert_eq!(extract_gender(&elem_nil), "Unknown");
}

#[test]
fn test_turk_farmhand_elbi_female_gender_regression() {
    // Regression test matching the real-world Türk farm structure where farmhand elbi was misidentified as Male
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <player>
    <name>Kubilay</name>
    <UniqueMultiplayerID>1001</UniqueMultiplayerID>
    <farmName>Türk</farmName>
    <isMale>true</isMale>
    <money>48250</money>
    <totalMoneyEarned>150000</totalMoneyEarned>
  </player>
  <locations>
    <GameLocation xsi:type="Farm">
      <buildings>
        <Building>
          <buildingType>Stone Cabin</buildingType>
          <indoors xsi:type="Cabin">
            <uniqueName>FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8</uniqueName>
            <name>Cabin</name>
            <farmhandReference>1002</farmhandReference>
          </indoors>
        </Building>
      </buildings>
    </GameLocation>
  </locations>
  <farmhands>
    <Farmer>
      <name>elbi</name>
      <UniqueMultiplayerID>1002</UniqueMultiplayerID>
      <Gender>Female</Gender>
      <isMale xsi:nil="true" />
      <money>48250</money>
      <totalMoneyEarned>150000</totalMoneyEarned>
    </Farmer>
  </farmhands>
  <farmerTeam>
    <useSeparateWallets>false</useSeparateWallets>
  </farmerTeam>
</SaveGame>"#;

    let elem = Element::parse(xml.as_bytes()).unwrap();
    let characters = stardew_sync_core::core::character::extract_all_characters_from_save(&elem, "TXrk_450560341");
    assert_eq!(characters.len(), 2);

    let host = characters.iter().find(|c| c.is_host).unwrap();
    assert_eq!(host.name, "Kubilay");
    assert_eq!(host.gender, "Male");
    assert!(host.is_wallet_shared);
    assert_eq!(host.money, 48250);

    let farmhand = characters.iter().find(|c| !c.is_host).unwrap();
    assert_eq!(farmhand.name, "elbi");
    // CRITICAL: elbi must be detected as Female, NOT Male!
    assert_eq!(farmhand.gender, "Female");
    assert!(farmhand.is_wallet_shared);
    assert_eq!(farmhand.money, 48250);
    // Friendly cabin name must be "Stone Cabin", not the raw FarmHouse UUID
    assert_eq!(farmhand.cabin_name, Some("Stone Cabin".to_string()));
    assert_eq!(
        farmhand.raw_cabin_id,
        Some("FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8".to_string())
    );
}

#[test]
fn test_separate_wallets_detection() {
    let xml = r#"<SaveGame><player><name>Solo</name><money>500</money></player><farmerTeam><useSeparateWallets>true</useSeparateWallets></farmerTeam></SaveGame>"#;
    let elem = Element::parse(xml.as_bytes()).unwrap();
    let characters = stardew_sync_core::core::character::extract_all_characters_from_save(&elem, "Solo_001");
    assert_eq!(characters.len(), 1);
    assert!(!characters[0].is_wallet_shared);
}

#[test]
fn test_discover_farms_farmhands_extraction_regression_turk_elbi() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let temp_saves = tempfile::tempdir().unwrap();
    let farm_folder = "TXrk_450560341";
    let farm_dir = temp_saves.path().join(farm_folder);
    std::fs::create_dir_all(&farm_dir).unwrap();

    // Primary save has host Kubilay + farmhand elbi in Stone Cabin
    let primary_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <player>
    <name>Kubilay</name>
    <UniqueMultiplayerID>1001</UniqueMultiplayerID>
    <farmName>Türk</farmName>
  </player>
  <locations>
    <GameLocation xsi:type="Farm">
      <buildings>
        <Building>
          <buildingType>Stone Cabin</buildingType>
          <indoors xsi:type="Cabin">
            <farmhandReference>1002</farmhandReference>
          </indoors>
        </Building>
      </buildings>
    </GameLocation>
  </locations>
  <farmhands>
    <Farmer>
      <name>elbi</name>
      <UniqueMultiplayerID>1002</UniqueMultiplayerID>
    </Farmer>
  </farmhands>
</SaveGame>"#;

    // SaveGameInfo does NOT contain farmhands tag (standard in Stardew Valley)
    let info_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Farmer xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <name>Kubilay</name>
  <farmName>Türk</farmName>
  <money>48250</money>
  <currentSeason>winter</currentSeason>
  <dayOfMonth>12</dayOfMonth>
  <year>1</year>
</Farmer>"#;

    std::fs::write(farm_dir.join(farm_folder), primary_xml).unwrap();
    std::fs::write(farm_dir.join("SaveGameInfo"), info_xml).unwrap();

    // Discover saves in this directory directly
    let discovered = stardew_sync_core::core::save::discovery::discover_saves(temp_saves.path()).unwrap();
    assert_eq!(discovered.len(), 1);

    // Verify extract_all_characters_from_save finds both host and elbi
    let primary_content = std::fs::read_to_string(&discovered[0].primary_save_path).unwrap();
    let primary_elem = xmltree::Element::parse(primary_content.as_bytes()).unwrap();
    let characters = stardew_sync_core::core::character::extract_all_characters_from_save(&primary_elem, farm_folder);

    let mut farmhands = Vec::new();
    for c in characters {
        if !c.is_host && !c.name.trim().is_empty() {
            farmhands.push(c.name);
        }
    }

    assert_eq!(farmhands, vec!["elbi".to_string()], "Farmhands must contain 'elbi', not be empty!");
}





