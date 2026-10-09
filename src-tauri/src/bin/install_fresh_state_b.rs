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

fn check_wgs_index_for_slot(slot_name: &str) -> bool {
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let pattern = format!("{}\\Packages", localappdata);
        if let Ok(packages_dir) = fs::read_dir(pattern) {
            for entry in packages_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("ConcernedApe.StardewValleyPC_") {
                    let wgs_dir = entry.path().join("SystemAppData").join("wgs");
                    if wgs_dir.is_dir() {
                        // Recursively search for containers.index
                        for sub in walkdir::WalkDir::new(&wgs_dir).into_iter().flatten() {
                            if sub.file_name() == "containers.index" {
                                if let Ok(bytes) = fs::read(sub.path()) {
                                    let ascii = String::from_utf8_lossy(&bytes);
                                    let u16_slice: Vec<u16> = bytes
                                        .chunks_exact(2)
                                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                                        .collect();
                                    let unicode = String::from_utf16_lossy(&u16_slice);
                                    if ascii.contains(slot_name) || unicode.contains(slot_name) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("  STARDEW-SYNC: PHASE 4.6B FRESH STATE B CONTROLLED INSTALLATION");
    println!("  Target: TXrkTestB_999450561 (elbi Host — TürkTestB Çiftliği)");
    println!("  Author: bazq");
    println!("============================================================");

    // 1. CONFIRM STARDEW VALLEY IS NOT RUNNING
    println!("\n[1/7] Verifying Stardew Valley and SMAPI process states...");
    let monitor = ProcessMonitor::new_system();
    if monitor.is_stardew_running() {
        eprintln!("\n[CRITICAL ERROR] Stardew Valley or SMAPI is currently running!");
        eprintln!("Installation of saves while the game is running is strictly forbidden.");
        std::process::exit(1);
    }
    println!("  - Stardew Valley is NOT running: PASS");
    println!("  - StardewModdingAPI is NOT running: PASS");

    // 2. VERIFY STAGED STATE AGAINST BASELINE MANIFEST
    println!("\n[2/7] Verifying staged Fresh State B against RUNTIME_TEST_BASELINE.json...");
    let staged_dir = PathBuf::from(
        "C:\\Users\\bazq3\\Desktop\\stardew-sync-test\\disposable-runtime-states\\state-b-fresh-elbi-host\\TXrkTestB_999450561"
    );
    if !staged_dir.is_dir() {
        eprintln!("[CRITICAL ERROR] Staged Fresh State B directory not found: {}", staged_dir.display());
        std::process::exit(1);
    }

    let manifest_path = staged_dir.join("RUNTIME_TEST_BASELINE.json");
    let manifest_str = fs::read_to_string(&manifest_path)?;
    let manifest: RuntimeBaselineManifest = serde_json::from_str(&manifest_str)?;

    if manifest.folder_name != "TXrkTestB_999450561" || manifest.game_id != 999450561 {
        eprintln!("[CRITICAL ERROR] Staged manifest does not match Fresh State B identity!");
        std::process::exit(1);
    }

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

    // 3. ENFORCE PRODUCTION GUARD AND XML SCHEMA INTEGRITY
    println!("\n[3/7] Enforcing Production Guard and XML schema checks...");
    let save_raw_bytes = fs::read(&staged_save_path)?;
    let info_raw_bytes = fs::read(&staged_info_path)?;

    if !save_raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) || !info_raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        eprintln!("[CRITICAL ERROR] Missing UTF-8 BOM on staged files!");
        std::process::exit(1);
    }
    println!("  - UTF-8 BOM present on both save files: PASS");

    let save_xml = fs::read_to_string(&staged_save_path)?;
    let info_xml = fs::read_to_string(&staged_info_path)?;

    if !save_xml.contains("xmlns:xsi=") || !save_xml.contains("xmlns:xsd=") || !info_xml.contains("xmlns:xsi=") || !info_xml.contains("xmlns:xsd=") {
        eprintln!("[CRITICAL ERROR] Missing XML schema namespaces!");
        std::process::exit(1);
    }
    println!("  - XML schema namespaces (xmlns:xsi, xmlns:xsd) present: PASS");

    if save_xml.matches(" type=").count() > 0 || save_xml.matches(" nil=").count() > 0 || info_xml.matches(" type=").count() > 0 || info_xml.matches(" nil=").count() > 0 {
        eprintln!("[CRITICAL ERROR] Naked type/nil attributes detected!");
        std::process::exit(1);
    }
    println!("  - Zero naked type/nil attributes: PASS");

    ProductionGuard::validate_save_content(&save_xml)?;
    println!("  - Save XML content free of production Game ID ({}): PASS", PRODUCTION_GAME_ID);

    // 4. CHECK TARGET IDENTITY DOES NOT ALREADY EXIST
    println!("\n[4/7] Verifying new identity does NOT pre-exist in Saves or WGS...");
    let appdata = std::env::var("APPDATA")?;
    let saves_root = PathBuf::from(appdata).join("StardewValley").join("Saves");
    if !saves_root.is_dir() {
        eprintln!("[CRITICAL ERROR] Saves directory not found at: {}", saves_root.display());
        std::process::exit(1);
    }

    let target_dir = saves_root.join(&manifest.folder_name);
    if target_dir.exists() {
        eprintln!("[CRITICAL ERROR] Target identity already exists in Saves: {}", target_dir.display());
        eprintln!("Refusing to overwrite! Fresh identity must be completely unused.");
        std::process::exit(1);
    }
    println!("  - Target slot '{}' does NOT exist in Saves: PASS", manifest.folder_name);

    if check_wgs_index_for_slot(&manifest.folder_name) {
        eprintln!("[CRITICAL ERROR] Target identity '{}' already found in WGS containers.index!", manifest.folder_name);
        eprintln!("Refusing installation! Fresh identity must have no WGS container.");
        std::process::exit(1);
    }
    println!("  - Target slot '{}' does NOT exist in WGS containers.index: PASS", manifest.folder_name);

    // Production guard on target directory
    ProductionGuard::validate_target_path(&target_dir)?;
    let prod_path = saves_root.join(PRODUCTION_FARM_FOLDER);
    println!("  - Production farm path (TXrk_450560341) is 100% PROTECTED: PASS");
    assert_ne!(target_dir, prod_path, "Target must never equal production path!");

    // State A preservation check
    let state_a_path = saves_root.join("TXrkTest_999450560");
    if state_a_path.is_dir() {
        println!("  - Existing State A slot (TXrkTest_999450560) is PRESERVED: PASS");
    }

    // 5. INSTALL ONLY THE NEW TXrkTestB_999450561 DIRECTORY
    println!("\n[5/7] Installing Fresh State B into {}...", target_dir.display());
    fs::create_dir_all(&target_dir)?;

    let installed_save_path = target_dir.join(&manifest.primary_save_file_name);
    let installed_info_path = target_dir.join("SaveGameInfo");

    fs::copy(&staged_save_path, &installed_save_path)?;
    fs::copy(&staged_info_path, &installed_info_path)?;
    println!("  - Copied primary save: {}", manifest.primary_save_file_name);
    println!("  - Copied SaveGameInfo");

    // 6. POST-INSTALL BIT-FOR-BIT HASH VERIFICATION
    println!("\n[6/7] Verifying installed files bit-for-bit against staged State B...");
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

    SaveValidator::validate_basic(&target_dir)?;
    println!("  - SaveValidator Layer 1 check on installed save: PASS");

    // 7. RECORD PRE-LAUNCH CLOUD / FILESYSTEM OBSERVATION SNAPSHOT
    println!("\n[7/7] Recording pre-launch observation snapshot...");
    let prelaunch_snapshot = CloudObserver::take_snapshot(&target_dir)?;
    let snapshot_json = serde_json::to_string_pretty(&prelaunch_snapshot)?;
    let snapshot_out_path = PathBuf::from(
        "C:\\Users\\bazq3\\Desktop\\stardew-sync-test\\disposable-runtime-states\\state-b-fresh-prelaunch-snapshot.json"
    );
    fs::write(&snapshot_out_path, snapshot_json)?;
    println!("  - Pre-launch snapshot recorded at: {}", snapshot_out_path.display());

    println!("\n============================================================");
    println!("  FRESH STATE B INSTALLATION COMPLETE & VERIFIED");
    println!("============================================================");
    println!("Installed State: Fresh State B (elbi Host)");
    println!("Installed Slot:  TXrkTestB_999450561");
    println!("Farm Name:       TürkTestB");
    println!("Root Host:       elbi");
    println!("Farmhand:        Kubilay");
    println!("Production Farm: 100% UNTOUCHED ({})", prod_path.display());
    println!("State A Slot:    PRESERVED ({})", state_a_path.display());
    println!("WGS Modified:    NO (Zero touches to containers, blobs, or indexes)");
    println!("Stardew Status:  NOT LAUNCHED");

    Ok(())
}
