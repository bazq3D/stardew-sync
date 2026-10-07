use std::fs;
use std::path::PathBuf;

use stardew_sync_core::{PostRuntimeAnalyzer, RuntimeBaselineManifest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cargo run --bin analyze_runtime_test -- <PATH_TO_MANIFEST_JSON> <PATH_TO_TEST_SAVE_DIR>");
        eprintln!("Example: cargo run --bin analyze_runtime_test -- \"path/to/RUNTIME_TEST_BASELINE.json\" \"path/to/TXrkTest_999450560\"");
        std::process::exit(1);
    }

    let manifest_path = PathBuf::from(&args[1]);
    let save_dir_path = PathBuf::from(&args[2]);

    println!("============================================================");
    println!("  STARDEW-SYNC: POST-RUNTIME SAVE ANALYZER");
    println!("  Author: bazq");
    println!("============================================================");
    println!("Manifest: {}", manifest_path.display());
    println!("Save directory: {}", save_dir_path.display());

    let manifest_json = fs::read_to_string(&manifest_path)?;
    let manifest: RuntimeBaselineManifest = serde_json::from_str(&manifest_json)?;

    let report = PostRuntimeAnalyzer::analyze_folder(&manifest, &save_dir_path)?;

    println!("\n[1/4] STRUCTURAL ANALYSIS:");
    println!(
        "  - XML Well-Formed: {}",
        if report.structural.xml_parses_successfully {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - Single Root Host: {}",
        if report.structural.exactly_one_root_host {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - Expected Host Matches: {}",
        if report.structural.expected_host_matches {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - Unique Multiplayer IDs: {}",
        if report.structural.unique_multiplayer_ids {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - All Expected Farmers Present: {}",
        if report.structural.all_expected_farmers_present {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - Cabin References Valid: {}",
        if report.structural.cabin_references_valid {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - Home Locations Valid: {}",
        if report.structural.home_locations_valid {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - SaveGameInfo Matches: {}",
        if report.structural.save_game_info_valid {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  - modData Preserved: {}",
        if report.structural.mod_data_preserved {
            "PASS"
        } else {
            "FAIL"
        }
    );

    println!("\n[2/4] PLAYER SEMANTIC ANALYSIS:");
    for f in &report.farmers {
        println!(
            "  - Player '{}' (ID: [REDACTED], Host: {}, Home: {}):",
            f.name, f.is_host, f.home_location
        );
        println!("    * Fingerprint Changed: {}", f.fingerprint_changed);
        if let Some(ref bf) = f.baseline_fingerprint {
            println!("    * Baseline Fingerprint: {}", bf);
        }
        println!("    * Current Fingerprint:  {}", f.current_fingerprint);
        if !f.gameplay_changes_detected.is_empty() {
            println!(
                "    * Gameplay Changes Detected: {:?}",
                f.gameplay_changes_detected
            );
        }
        println!(
            "    * Core Identity Preserved: {}",
            if f.identity_preserved { "YES" } else { "NO" }
        );
    }

    println!("\n[3/4] WORLD & PROGRESSION ANALYSIS:");
    println!(
        "  - In-Game Date: Season {}, Day {}, Year {}",
        report.world.season, report.world.day_of_month, report.world.year
    );
    if let Some(t) = report.world.time_of_day {
        println!("  - Time of Day: {}", t);
    }
    println!("  - Location Count: {}", report.world.location_count);
    println!("  - Cabin Count: {}", report.world.farm_building_count);
    println!(
        "  - Gameplay Progression Observed: {}",
        report.world.gameplay_progression_observed
    );
    if !report.world.observed_progression_details.is_empty() {
        println!(
            "  - Progression Details: {:?}",
            report.world.observed_progression_details
        );
    }

    println!("\n[4/4] SERIALIZATION & INTEGRITY VERDICT:");
    println!(
        "  - Byte Identical to Baseline: {}",
        report.byte_identical_to_baseline
    );
    println!(
        "  - Serialization Rewrite Detected: {}",
        report.serialization_rewrite_detected
    );
    if !report.unexpected_destructive_changes.is_empty() {
        println!(
            "  - [CRITICAL] Unexpected Destructive Changes: {:?}",
            report.unexpected_destructive_changes
        );
    }

    println!("\n============================================================");
    if report.is_healthy {
        println!("  OVERALL STATUS: HEALTHY (VALIDATED FOR STARDEW RUNTIME)");
    } else {
        println!("  OVERALL STATUS: CORRUPTED / ANOMALY DETECTED");
    }
    println!("============================================================");

    Ok(())
}
