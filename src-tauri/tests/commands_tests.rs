use stardew_sync_core::commands::*;
use xmltree::Element;

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

        // If this is the developer's legacy production farm, verify accurate Fall date parsing
        if farm.folder_name == "TXrk_450560341" {
            assert!(farm.is_legacy_production);
            assert_eq!(farm.farm_name, "Türk", "Farm name must be parsed from SaveGameInfo");
            assert_eq!(farm.host_name, "Kubilay");
            assert!(
                farm.date_summary.contains("Fall") && farm.date_summary.contains("23"),
                "Production farm date must be parsed accurately as Fall 23, got '{}'",
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
