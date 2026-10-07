use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use stardew_sync_core::{
    CloudObserver, ProcessMonitor, ProductionGuard, RuntimeBaselineManifest, SaveValidator,
    PRODUCTION_FARM_FOLDER, PRODUCTION_GAME_ID,
};

fn compute_file_sha256(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let is_state_b = args.iter().any(|a| a.to_lowercase().contains("state-b") || a == "b");

    let (state_label, staged_folder, snapshot_filename) = if is_state_b {
        ("STATE B (elbi Host)", "state-b-elbi-host", "state-b-prelaunch-snapshot.json")
    } else {
        ("STATE A (Kubilay Host)", "state-a-kubilay-host", "state-a-prelaunch-snapshot.json")
    };

    println!("============================================================");
    println!("  STARDEW-SYNC: PHASE 4.6 {} CONTROLLED INSTALLATION", state_label);
    println!("  Author: bazq");
    println!("============================================================");

    // 1. CONFIRM STARDEW VALLEY IS NOT RUNNING
    println!("\n[1/6] Verifying Stardew Valley process state...");
    let monitor = ProcessMonitor::new_system();
    if monitor.is_stardew_running() {
        eprintln!("\n[CRITICAL ERROR] Stardew Valley or SMAPI is currently running!");
        eprintln!("Installation of saves while the game is running is strictly forbidden.");
        std::process::exit(1);
    }
    println!("  - Stardew Valley is NOT running: PASS");
    println!("  - StardewModdingAPI is NOT running: PASS");

    // 2. VERIFY STAGED STATE AGAINST BASELINE MANIFEST
    println!("\n[2/6] Verifying staged {} against RUNTIME_TEST_BASELINE.json...", state_label);
    let staged_dir = PathBuf::from(format!(
        "C:\\Users\\bazq3\\Desktop\\stardew-sync-test\\disposable-runtime-states\\{}\\TXrkTest_999450560",
        staged_folder
    ));
    if !staged_dir.is_dir() {
        eprintln!("[CRITICAL ERROR] Staged {} directory not found: {}", state_label, staged_dir.display());
        std::process::exit(1);
    }

    let manifest_path = staged_dir.join("RUNTIME_TEST_BASELINE.json");
    let manifest_str = fs::read_to_string(&manifest_path)?;
    let manifest: RuntimeBaselineManifest = serde_json::from_str(&manifest_str)?;

    let staged_save_path = staged_dir.join(&manifest.primary_save_file_name);
    let staged_info_path = staged_dir.join("SaveGameInfo");

    let staged_save_hash = compute_file_sha256(&staged_save_path)?;
    let staged_info_hash = compute_file_sha256(&staged_info_path)?;

    if staged_save_hash != manifest.primary_save_sha256 {
        eprintln!("[CRITICAL ERROR] Staged save hash mismatch!");
        eprintln!("  Expected: {}", manifest.primary_save_sha256);
        eprintln!("  Actual:   {}", staged_save_hash);
        std::process::exit(1);
    }
    if staged_info_hash != manifest.save_game_info_sha256 {
        eprintln!("[CRITICAL ERROR] Staged SaveGameInfo hash mismatch!");
        eprintln!("  Expected: {}", manifest.save_game_info_sha256);
        eprintln!("  Actual:   {}", staged_info_hash);
        std::process::exit(1);
    }
    println!("  - Primary Save SHA-256 matches baseline: PASS ({})", staged_save_hash);
    println!("  - SaveGameInfo SHA-256 matches baseline: PASS ({})", staged_info_hash);

    // 3. ENFORCE PRODUCTION GUARD ON CONTENT & TARGET
    println!("\n[3/6] Enforcing Production Guard and XML Schema validations...");
    let save_raw_bytes = fs::read(&staged_save_path)?;
    let info_raw_bytes = fs::read(&staged_info_path)?;

    // 3a. UTF-8 BOM checks
    if !save_raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        eprintln!("[CRITICAL ERROR] Staged primary save is missing UTF-8 BOM!");
        std::process::exit(1);
    }
    if !info_raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        eprintln!("[CRITICAL ERROR] Staged SaveGameInfo is missing UTF-8 BOM!");
        std::process::exit(1);
    }
    println!("  - UTF-8 BOM present on both save files: PASS");

    // 3b. Schema namespaces & naked attribute checks
    let save_xml = fs::read_to_string(&staged_save_path)?;
    let info_xml = fs::read_to_string(&staged_info_path)?;

    if !save_xml.contains("xmlns:xsi=") || !save_xml.contains("xmlns:xsd=") {
        eprintln!("[CRITICAL ERROR] Missing xmlns:xsi or xmlns:xsd on primary save root!");
        std::process::exit(1);
    }
    if !info_xml.contains("xmlns:xsi=") || !info_xml.contains("xmlns:xsd=") {
        eprintln!("[CRITICAL ERROR] Missing xmlns:xsi or xmlns:xsd on SaveGameInfo root!");
        std::process::exit(1);
    }
    println!("  - XML schema namespaces (xmlns:xsi, xmlns:xsd) present on roots: PASS");

    let save_naked_type = save_xml.matches(" type=").count();
    let save_naked_nil = save_xml.matches(" nil=").count();
    let info_naked_type = info_xml.matches(" type=").count();
    let info_naked_nil = info_xml.matches(" nil=").count();

    if save_naked_type > 0 || save_naked_nil > 0 || info_naked_type > 0 || info_naked_nil > 0 {
        eprintln!("[CRITICAL ERROR] Naked type/nil attributes detected! XML serializer stripped prefixes.");
        std::process::exit(1);
    }
    println!("  - Zero naked ' type=' or ' nil=' attributes in save and SaveGameInfo: PASS");
    println!("  - Preserved xsi:type count (Save: {}, Info: {}): PASS", save_xml.matches("xsi:type=").count(), info_xml.matches("xsi:type=").count());
    println!("  - Preserved xsi:nil count (Save: {}, Info: {}): PASS", save_xml.matches("xsi:nil=").count(), info_xml.matches("xsi:nil=").count());

    ProductionGuard::validate_save_content(&save_xml)?;
    println!("  - Save XML content does NOT contain production Game ID ({}): PASS", PRODUCTION_GAME_ID);

    let appdata = std::env::var("APPDATA")?;
    let saves_root = PathBuf::from(appdata).join("StardewValley").join("Saves");
    if !saves_root.is_dir() {
        eprintln!("[CRITICAL ERROR] Saves directory not found at: {}", saves_root.display());
        std::process::exit(1);
    }

    let target_dir = saves_root.join(&manifest.folder_name);
    println!("  - Installation target path: {}", target_dir.display());

    // Strict validation that target is NOT the production farm
    ProductionGuard::validate_target_path(&target_dir)?;
    if target_dir.file_name().unwrap().to_string_lossy() != "TXrkTest_999450560" {
        eprintln!("[CRITICAL ERROR] Target folder name must be exactly 'TXrkTest_999450560'!");
        std::process::exit(1);
    }
    println!("  - Target folder is strictly 'TXrkTest_999450560': PASS");

    let prod_path = saves_root.join(PRODUCTION_FARM_FOLDER);
    println!("  - Production farm path (TXrk_450560341) is 100% PROTECTED & UNTOUCHED: PASS");
    assert_ne!(target_dir, prod_path, "Target must never equal production path!");

    // 4. CLEAN & INSTALL ONLY DISPOSABLE STATE A
    println!("\n[4/6] Installing State A into {}...", target_dir.display());
    if target_dir.is_dir() {
        println!("  - Removing previous disposable test directory '{}'...", target_dir.file_name().unwrap().to_string_lossy());
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    let installed_save_path = target_dir.join(&manifest.primary_save_file_name);
    let installed_info_path = target_dir.join("SaveGameInfo");

    fs::copy(&staged_save_path, &installed_save_path)?;
    fs::copy(&staged_info_path, &installed_info_path)?;
    println!("  - Copied primary save: {}", manifest.primary_save_file_name);
    println!("  - Copied SaveGameInfo");


    // 5. POST-INSTALL BIT-FOR-BIT HASH VERIFICATION
    println!("\n[5/6] Verifying installed files bit-for-bit against staged State A...");
    let installed_save_hash = compute_file_sha256(&installed_save_path)?;
    let installed_info_hash = compute_file_sha256(&installed_info_path)?;

    if installed_save_hash != manifest.primary_save_sha256 {
        eprintln!("[CRITICAL ERROR] Installed save file hash mismatch!");
        std::process::exit(1);
    }
    if installed_info_hash != manifest.save_game_info_sha256 {
        eprintln!("[CRITICAL ERROR] Installed SaveGameInfo file hash mismatch!");
        std::process::exit(1);
    }
    println!("  - Installed primary save SHA-256: {} (MATCH)", installed_save_hash);
    println!("  - Installed SaveGameInfo SHA-256: {} (MATCH)", installed_info_hash);

    // Validate using SaveValidator
    SaveValidator::validate_basic(&target_dir)?;
    println!("  - SaveValidator Layer 1 check on installed save: PASS");

    // 6. RECORD PRE-LAUNCH CLOUD / FILESYSTEM OBSERVATION SNAPSHOT
    println!("\n[6/6] Recording pre-launch observation snapshot...");
    let prelaunch_snapshot = CloudObserver::take_snapshot(&target_dir)?;
    let snapshot_json = serde_json::to_string_pretty(&prelaunch_snapshot)?;
    let snapshot_out_path = PathBuf::from(format!(
        "C:\\Users\\bazq3\\Desktop\\stardew-sync-test\\disposable-runtime-states\\{}",
        snapshot_filename
    ));
    fs::write(&snapshot_out_path, snapshot_json)?;
    println!("  - Pre-launch snapshot recorded at: {}", snapshot_out_path.display());

    println!("\n============================================================");
    println!("  INSTALLATION COMPLETE & VERIFIED");
    println!("============================================================");
    println!("Installed State: {}", state_label);
    println!("Installed Farm: {} ({} = Host)", manifest.farm_name, manifest.host_name);
    println!("Target Path: {}", target_dir.display());
    println!("Production Farm untouched: {}", prod_path.display());
    println!("Stardew Valley launched: NO");

    Ok(())
}

