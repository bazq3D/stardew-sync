use stardew_sync_core::commands::*;

#[test]
fn test_get_app_status_structure() {
    let status = get_app_status().expect("get_app_status should succeed");
    assert_eq!(status.app_version, "0.1.0");
    assert_eq!(status.author, "bazq");
    assert!(!status.platform.is_empty());
    assert!(!status.arch.is_empty());
    assert!(status.saves_dir.contains("StardewValley"));
    assert!(status.sync_status.contains("Phase 5.0"));
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
    assert_eq!(update_info.current_version, "0.1.0");
    assert!(update_info.endpoint.contains("github.com/bazq3/stardew-sync-p2p"));
    assert!(update_info.public_key_configured);
}

#[test]
fn test_discover_farms_returns_vector() {
    let farms = discover_farms().expect("discover_farms should succeed");
    // If saves exist on this system, verify that if production farm is present, it is flagged
    for farm in &farms {
        if farm.folder_name == "TXrk_450560341" {
            assert!(farm.is_production, "Production farm must be flagged as production");
            assert!(!farm.is_disposable, "Production farm must not be flagged as disposable");
        }
        if farm.folder_name.to_lowercase().contains("test") {
            assert!(farm.is_disposable, "Test farm must be flagged as disposable");
            assert!(!farm.is_production, "Test farm must not be flagged as production");
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
