mod test_fixtures;

use test_fixtures::{generate_test_save_xml, generate_test_save_game_info_xml, CABIN_NAME, PLAYER_A_ID, PLAYER_B_ID};
use stardew_sync_core::{
    compute_migration_stable_fingerprint, compute_full_farmer_fingerprint,
    HostMigrator, ParsedSave,
};

#[test]
fn test_host_migration_a_to_b() {
    let save_xml = generate_test_save_xml();
    let info_xml = generate_test_save_game_info_xml();

    let result = HostMigrator::migrate(&save_xml, &info_xml, PLAYER_B_ID)
        .expect("Migration A -> B should succeed");

    assert_eq!(result.previous_host_id, PLAYER_A_ID);
    assert_eq!(result.new_host_id, PLAYER_B_ID);
    assert_eq!(result.target_cabin_name, CABIN_NAME);

    // Verify parsed structure of transformed save
    let transformed = ParsedSave::parse(&result.transformed_save_xml)
        .expect("Transformed save must be valid XML");

    // 1. Host assertions
    let host = &transformed.metadata.host_player;
    assert_eq!(host.name, "PlayerB");
    assert_eq!(host.unique_multiplayer_id, PLAYER_B_ID);
    assert_eq!(host.home_location, "FarmHouse");
    assert_eq!(host.house_upgrade_level, 2, "New host must inherit Farmhouse upgrade level");

    // 2. Cabin Farmhand assertions
    assert_eq!(transformed.metadata.cabins.len(), 1);
    let cabin = &transformed.metadata.cabins[0];
    let farmhand = cabin.farmhand.as_ref().expect("Cabin must contain farmhand");
    assert_eq!(farmhand.name, "PlayerA");
    assert_eq!(farmhand.unique_multiplayer_id, PLAYER_A_ID);
    assert_eq!(farmhand.home_location, CABIN_NAME);
    assert_eq!(farmhand.house_upgrade_level, 1, "New farmhand must inherit Cabin upgrade level");

    // 3. SaveGameInfo assertions
    assert!(result.transformed_save_game_info_xml.contains("PlayerB"));
    assert!(result.transformed_save_game_info_xml.contains(&PLAYER_B_ID.to_string()));

    // 4. World continuity assertions
    assert_eq!(transformed.metadata.current_season, "summer");
    assert_eq!(transformed.metadata.day_of_month, 18);
    assert_eq!(transformed.metadata.year, 2);
}

#[test]
fn test_host_migration_roundtrip_a_to_b_to_a() {
    let orig_save_xml = generate_test_save_xml();
    let orig_info_xml = generate_test_save_game_info_xml();

    let orig_parsed = ParsedSave::parse(&orig_save_xml).unwrap();
    let orig_player_a_fp = compute_full_farmer_fingerprint(orig_parsed.root.get_child("player").unwrap()).unwrap();

    // 1. Migrate A -> B
    let step1 = HostMigrator::migrate(&orig_save_xml, &orig_info_xml, PLAYER_B_ID)
        .expect("A -> B failed");

    // 2. Migrate B -> A
    let step2 = HostMigrator::migrate(&step1.transformed_save_xml, &step1.transformed_save_game_info_xml, PLAYER_A_ID)
        .expect("B -> A failed");

    assert_eq!(step2.previous_host_id, PLAYER_B_ID);
    assert_eq!(step2.new_host_id, PLAYER_A_ID);

    let roundtrip_parsed = ParsedSave::parse(&step2.transformed_save_xml).unwrap();
    let roundtrip_player_a_fp = compute_full_farmer_fingerprint(roundtrip_parsed.root.get_child("player").unwrap()).unwrap();

    // PlayerA full fingerprint (including homeLocation and upgradeLevel) MUST match original perfectly
    assert_eq!(
        orig_player_a_fp, roundtrip_player_a_fp,
        "PlayerA full state must be 100% bit-for-bit identical after A -> B -> A roundtrip"
    );

    // Host must be PlayerA again
    assert_eq!(roundtrip_parsed.metadata.host_player.unique_multiplayer_id, PLAYER_A_ID);
    assert_eq!(roundtrip_parsed.metadata.host_player.name, "PlayerA");
    assert_eq!(roundtrip_parsed.metadata.host_player.home_location, "FarmHouse");

    // Cabin must contain PlayerB again
    let cabin_farmhand = roundtrip_parsed.metadata.cabins[0].farmhand.as_ref().unwrap();
    assert_eq!(cabin_farmhand.unique_multiplayer_id, PLAYER_B_ID);
    assert_eq!(cabin_farmhand.name, "PlayerB");
}

#[test]
fn test_repeated_roundtrip_stress_test() {
    let mut current_save_xml = generate_test_save_xml();
    let mut current_info_xml = generate_test_save_game_info_xml();

    let orig_parsed = ParsedSave::parse(&current_save_xml).unwrap();
    let orig_a_stable_fp = compute_migration_stable_fingerprint(orig_parsed.root.get_child("player").unwrap()).unwrap();
    
    // Find PlayerB element
    let mut orig_b_stable_fp = String::new();
    for cabin in &orig_parsed.metadata.cabins {
        if let Some(ref fh) = cabin.farmhand {
            if fh.unique_multiplayer_id == PLAYER_B_ID {
                // locate in xml
                for b in &orig_parsed.root.get_child("locations").unwrap().get_child("GameLocation").unwrap().get_child("buildings").unwrap().children {
                    if let xmltree::XMLNode::Element(be) = b {
                        if let Some(ind) = be.get_child("indoors") {
                            if let Some(fhe) = ind.get_child("farmhand") {
                                orig_b_stable_fp = compute_migration_stable_fingerprint(fhe).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }

    // Run 30 consecutive migrations: A -> B -> A -> B -> ...
    const CYCLES: usize = 30;
    for i in 0..CYCLES {
        let (target_id, expected_host_name) = if i % 2 == 0 {
            (PLAYER_B_ID, "PlayerB")
        } else {
            (PLAYER_A_ID, "PlayerA")
        };

        let result = HostMigrator::migrate(&current_save_xml, &current_info_xml, target_id)
            .unwrap_or_else(|e| panic!("Cycle {} failed migrating to {}: {}", i, expected_host_name, e));

        let parsed = ParsedSave::parse(&result.transformed_save_xml).unwrap();
        assert_eq!(parsed.metadata.host_player.name, expected_host_name);

        // Verify fingerprints never drift
        if expected_host_name == "PlayerB" {
            let b_actual_fp = compute_migration_stable_fingerprint(parsed.root.get_child("player").unwrap()).unwrap();
            assert_eq!(b_actual_fp, orig_b_stable_fp, "PlayerB drifted at cycle {}", i);
        } else {
            let a_actual_fp = compute_migration_stable_fingerprint(parsed.root.get_child("player").unwrap()).unwrap();
            assert_eq!(a_actual_fp, orig_a_stable_fp, "PlayerA drifted at cycle {}", i);
        }

        current_save_xml = result.transformed_save_xml;
        current_info_xml = result.transformed_save_game_info_xml;
    }
}

#[test]
fn test_mod_data_and_unknown_xml_nodes_preserved() {
    let save_xml = generate_test_save_xml();
    let info_xml = generate_test_save_game_info_xml();

    let result = HostMigrator::migrate(&save_xml, &info_xml, PLAYER_B_ID).unwrap();

    // Verify custom modData survived in transformed XML
    assert!(result.transformed_save_xml.contains("mod_test_flag"));
    assert!(result.transformed_save_xml.contains("alpha_42"));
    assert!(result.transformed_save_xml.contains("beta_99"));

    let roundtrip = HostMigrator::migrate(&result.transformed_save_xml, &result.transformed_save_game_info_xml, PLAYER_A_ID).unwrap();
    assert!(roundtrip.transformed_save_xml.contains("alpha_42"));
    assert!(roundtrip.transformed_save_xml.contains("beta_99"));
}

#[test]
fn test_cannot_migrate_to_current_host() {
    let save_xml = generate_test_save_xml();
    let info_xml = generate_test_save_game_info_xml();

    // Target is PlayerA, but PlayerA is already host
    let err = HostMigrator::migrate(&save_xml, &info_xml, PLAYER_A_ID)
        .expect_err("Migrating to current host must fail");

    assert!(err.to_string().contains("already the host"));
}

#[test]
fn test_cannot_migrate_to_nonexistent_player() {
    let save_xml = generate_test_save_xml();
    let info_xml = generate_test_save_game_info_xml();

    let err = HostMigrator::migrate(&save_xml, &info_xml, 9999999999999)
        .expect_err("Migrating to nonexistent player ID must fail");

    assert!(err.to_string().contains("not found in any cabin"));
}

#[test]
fn test_rejects_malformed_xml() {
    let malformed_save = "<SaveGame><player><name>Incomplete";
    let info_xml = generate_test_save_game_info_xml();

    let err = HostMigrator::migrate(malformed_save, &info_xml, PLAYER_B_ID)
        .expect_err("Malformed XML must fail validation");

    assert!(err.to_string().contains("XML parsing error"));
}
