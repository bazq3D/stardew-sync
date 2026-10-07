use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use stardew_sync_core::{
    DisposableIdentity, DisposableSaveManager, ParsedSave, ProductionGuard, SaveValidator,
    PRODUCTION_GAME_ID,
};

fn compute_file_sha256(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_directory_files(dir: &Path) -> std::io::Result<HashMap<String, String>> {
    let mut hashes = HashMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let filename = entry.file_name().to_string_lossy().to_string();
            let hash = compute_file_sha256(&entry.path())?;
            hashes.insert(filename, hash);
        }
    }
    Ok(hashes)
}

fn is_inside_live_stardew_directory(path: &Path) -> bool {
    let canonical = match path.canonicalize() {
        Ok(p) => p.to_string_lossy().to_lowercase(),
        Err(_) => path.to_string_lossy().to_lowercase(),
    };

    if let Ok(appdata) = std::env::var("APPDATA") {
        let live_saves = format!("{}\\stardewvalley\\saves", appdata).to_lowercase();
        if canonical.starts_with(&live_saves) || canonical.contains("\\stardewvalley\\saves") {
            return true;
        }
    }

    canonical.contains("stardewvalley\\saves") || canonical.contains("stardewvalley/saves")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin prepare_disposable -- <PATH_TO_OFFLINE_SOURCE_FARM>");
        eprintln!("Example: cargo run --bin prepare_disposable -- \"C:\\Users\\bazq3\\Desktop\\stardew-sync-test\\FARM_KLASORU\\TXrk_450560341\"");
        std::process::exit(1);
    }

    let source_input = PathBuf::from(&args[1]);

    println!("============================================================");
    println!("  STARDEW-SYNC: PHASE 4.6 DISPOSABLE TEST PREPARATION");
    println!("  Author: bazq");
    println!("============================================================");
    println!("Source copy path: {}", source_input.display());

    // 1. SAFETY CHECKS
    if is_inside_live_stardew_directory(&source_input) {
        eprintln!("\n[FATAL ERROR] REFUSING EXECUTION!");
        eprintln!("Supplied path resolves inside the live Stardew save directory.");
        eprintln!("Live saves must NEVER be touched directly.");
        std::process::exit(1);
    }

    let farm_path = if source_input.join("SaveGameInfo").is_file() {
        source_input.clone()
    } else {
        let mut sub = None;
        for entry in fs::read_dir(&source_input)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.path().join("SaveGameInfo").is_file() {
                sub = Some(entry.path());
                break;
            }
        }
        sub.ok_or_else(|| {
            format!(
                "No valid Stardew save found in '{}'",
                source_input.display()
            )
        })?
    };

    println!("\n[1/6] Validating source copy immutability baseline...");
    let initial_hashes = hash_directory_files(&farm_path)?;
    for (file, hash) in &initial_hashes {
        println!("  - {}: {}", file, hash);
    }

    // 2. READ SOURCE XML
    let (source_save_xml, source_info_xml) = SaveValidator::validate_basic(&farm_path)?;
    let parsed_source = ParsedSave::parse(&source_save_xml)?;

    println!("\n[2/6] Parsing source metadata...");
    println!("  - Source Farm Name: {}", parsed_source.metadata.farm_name);
    println!(
        "  - Source Host: {}",
        parsed_source.metadata.host_player.name
    );
    println!("  - Discovered farmhands:");
    for fh in &parsed_source.metadata.farmhands {
        println!(
            "    * {} (ID: [REDACTED], Home: {})",
            fh.name, fh.home_location
        );
    }

    // Find elbi
    let elbi_fh = parsed_source
        .metadata
        .farmhands
        .iter()
        .find(|f| f.name.to_lowercase().contains("elbi"))
        .ok_or_else(|| "Farmhand 'elbi' not found in source save".to_string())?;

    let elbi_id = elbi_fh.unique_multiplayer_id;

    // 3. CONFIGURE DISPOSABLE IDENTITY
    let identity = DisposableIdentity::default();
    println!("\n[3/6] Configuring disposable runtime identity:");
    println!("  - Target Farm Name: {}", identity.farm_name);
    println!(
        "  - Sanitized Folder Prefix: {}",
        identity.sanitized_folder_prefix
    );
    println!("  - Distinct Game ID: {}", identity.game_id);
    println!("  - Disposable Folder Name: {}", identity.folder_name());
    println!(
        "  - Primary Save File Name: {}",
        identity.primary_save_file_name()
    );

    // 4. PRODUCTION GUARD ENFORCEMENT
    println!("\n[4/6] Enforcing Production Guard checks...");
    let folder_name = identity.folder_name();
    let target_folder_path = Path::new(&folder_name);
    ProductionGuard::validate_target_path(target_folder_path)?;
    println!(
        "  - Target folder '{}' is NOT production farm (TXrk_450560341): PASS",
        identity.folder_name()
    );
    println!(
        "  - Target Game ID ({}) does NOT match production ID ({}): PASS",
        identity.game_id, PRODUCTION_GAME_ID
    );

    // 5. PREPARE BOTH ISOLATED STATES
    println!("\n[5/6] Generating and validating State A (Kubilay Host) and State B (elbi Host)...");
    let (state_a, state_b) = DisposableSaveManager::prepare_both_states(
        &source_save_xml,
        &source_info_xml,
        &identity,
        elbi_id,
    )?;

    // Guard check generated XMLs
    ProductionGuard::validate_save_content(&state_a.save_xml)?;
    ProductionGuard::validate_save_content(&state_b.save_xml)?;
    println!("  - State A XML content verified free of production Game ID: PASS");
    println!("  - State B XML content verified free of production Game ID: PASS");

    // Output directory (isolated staging outside live saves)
    let output_base =
        PathBuf::from("C:\\Users\\bazq3\\Desktop\\stardew-sync-test\\disposable-runtime-states");
    let state_a_dir = output_base
        .join("state-a-kubilay-host")
        .join(identity.folder_name());
    let state_b_dir = output_base
        .join("state-b-elbi-host")
        .join(identity.folder_name());

    // Write State A
    fs::create_dir_all(&state_a_dir)?;
    fs::write(
        state_a_dir.join(identity.primary_save_file_name()),
        &state_a.save_xml,
    )?;
    fs::write(
        state_a_dir.join("SaveGameInfo"),
        &state_a.save_game_info_xml,
    )?;
    let manifest_a_json = serde_json::to_string_pretty(&state_a.manifest)?;
    fs::write(
        state_a_dir.join("RUNTIME_TEST_BASELINE.json"),
        &manifest_a_json,
    )?;

    // Write State B
    fs::create_dir_all(&state_b_dir)?;
    fs::write(
        state_b_dir.join(identity.primary_save_file_name()),
        &state_b.save_xml,
    )?;
    fs::write(
        state_b_dir.join("SaveGameInfo"),
        &state_b.save_game_info_xml,
    )?;
    let manifest_b_json = serde_json::to_string_pretty(&state_b.manifest)?;
    fs::write(
        state_b_dir.join("RUNTIME_TEST_BASELINE.json"),
        &manifest_b_json,
    )?;

    // Validate written files
    SaveValidator::validate_basic(&state_a_dir)?;
    SaveValidator::validate_basic(&state_b_dir)?;
    println!("  - State A staged at: {}", state_a_dir.display());
    println!("  - State B staged at: {}", state_b_dir.display());

    // 6. SOURCE COPY IMMUTABILITY VERIFICATION
    println!("\n[6/6] Verifying source copy immutability...");
    let final_hashes = hash_directory_files(&farm_path)?;
    for (file, initial_hash) in &initial_hashes {
        let final_hash = final_hashes
            .get(file)
            .ok_or_else(|| format!("File {} was deleted!", file))?;
        if initial_hash != final_hash {
            eprintln!("[FATAL ERROR] Source file {} was modified!", file);
            std::process::exit(1);
        }
        println!("  - {}: UNCHANGED ({})", file, initial_hash);
    }

    println!("\n============================================================");
    println!("  STAGE 1 PREPARATION COMPLETE");
    println!("============================================================");
    println!("State A Manifest Summary:");
    println!("  - Host: {} (ID: [REDACTED])", state_a.manifest.host_name);
    println!(
        "  - Primary Save SHA-256: {}",
        state_a.manifest.primary_save_sha256
    );
    println!(
        "  - SaveGameInfo SHA-256: {}",
        state_a.manifest.save_game_info_sha256
    );
    println!(
        "  - Kubilay Fingerprint:  {}",
        state_a.manifest.kubilay_fingerprint
    );
    println!(
        "  - elbi Fingerprint:     {}",
        state_a.manifest.elbi_fingerprint
    );
    println!("\nState B Manifest Summary:");
    println!("  - Host: {} (ID: [REDACTED])", state_b.manifest.host_name);
    println!(
        "  - Primary Save SHA-256: {}",
        state_b.manifest.primary_save_sha256
    );
    println!(
        "  - SaveGameInfo SHA-256: {}",
        state_b.manifest.save_game_info_sha256
    );
    println!(
        "  - Kubilay Fingerprint:  {}",
        state_b.manifest.kubilay_fingerprint
    );
    println!(
        "  - elbi Fingerprint:     {}",
        state_b.manifest.elbi_fingerprint
    );
    println!("\nNOTE: Disposable test saves are staged OFFLINE in:");
    println!("  {}", output_base.display());
    println!("THEY ARE NOT INSTALLED INTO STARDEW SAVES DIRECTORY YET.");
    println!("STARDEW HAS NOT BEEN LAUNCHED.");
    println!("READY FOR USER APPROVAL.");

    Ok(())
}
