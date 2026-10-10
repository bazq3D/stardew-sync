use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_cryptographic_signature_verification_of_release_artifact() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent of src-tauri should be workspace root")
        .to_path_buf();

    // 1. Read tauri.conf.json pubkey
    let tauri_conf_path = workspace_root.join("src-tauri").join("tauri.conf.json");
    let tauri_conf_content =
        fs::read_to_string(&tauri_conf_path).expect("tauri.conf.json must be readable");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_content).expect("tauri.conf.json must parse as JSON");

    let pubkey_b64 = tauri_conf["plugins"]["updater"]["pubkey"]
        .as_str()
        .expect("plugins.updater.pubkey must be a string");

    let pubkey_raw = BASE64.decode(pubkey_b64).expect("pubkey must be valid base64");
    let pubkey_str = String::from_utf8(pubkey_raw).expect("pubkey must be valid UTF-8");

    let public_key =
        PublicKey::decode(&pubkey_str).expect("PublicKey must decode successfully from minisign string");

    // 2. Read manifest (prioritize target/release/bundle/nsis/latest.json if present)
    let target_manifest_path = workspace_root
        .join("target")
        .join("release")
        .join("bundle")
        .join("nsis")
        .join("latest.json");
    let manifest_path = if target_manifest_path.exists() {
        target_manifest_path
    } else {
        workspace_root.join("docs").join("updater-release-template.json")
    };
    let manifest_content =
        fs::read_to_string(&manifest_path).expect("manifest must be readable");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_content).expect("manifest must parse as JSON");

    let manifest_version = manifest["version"]
        .as_str()
        .expect("manifest version must be string");

    let raw_url = manifest["platforms"]["windows-x86_64"]["url"]
        .as_str()
        .expect("platforms.windows-x86_64.url must be a string");
    let canonical_filename = raw_url
        .split('/')
        .last()
        .expect("url must contain installer filename");

    let sig_b64 = manifest["platforms"]["windows-x86_64"]["signature"]
        .as_str()
        .expect("signature must be a string");

    let sig_raw = BASE64.decode(sig_b64).expect("signature must be valid base64");
    let sig_str = String::from_utf8(sig_raw).expect("signature must be valid UTF-8");

    let signature =
        Signature::decode(&sig_str).expect("Signature must decode successfully from minisign string");

    // 3. Verify trusted comment
    let trusted_comment = signature.trusted_comment();
    let expected_version_str = format!("version:{}", manifest_version);
    let expected_file_exact = format!("file:{}", canonical_filename);
    let expected_file_space = format!("file:{}", canonical_filename.replace("Stardew.Sync_", "Stardew Sync_"));
    assert!(
        trusted_comment.contains(&expected_version_str),
        "Trusted comment must contain {}: {}",
        expected_version_str,
        trusted_comment
    );
    assert!(
        trusted_comment.contains(&expected_file_exact) || trusted_comment.contains(&expected_file_space),
        "Trusted comment must contain file name: {}",
        trusted_comment
    );

    // 4. Verify actual authoritative installer bytes
    let nsis_dir = workspace_root.join("target").join("release").join("bundle").join("nsis");
    let installer_path = nsis_dir.join(canonical_filename);

    if !installer_path.exists() {
        if std::env::var("CI").is_ok() || std::env::var("REQUIRE_INSTALLER_VERIFY").is_ok() {
            panic!(
                "Canonical installer {:?} MUST exist on disk in release verification!",
                installer_path
            );
        }
        println!(
            "Installer {} does not exist yet on disk; skipping byte-level verification until build produces it.",
            canonical_filename
        );
        return;
    }

    // 5. Reject unexpected additional or ambiguous installers for this release version
    let mut version_exes = Vec::new();
    if let Ok(entries) = fs::read_dir(&nsis_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".exe") && name.contains(manifest_version) {
                version_exes.push(name);
            }
        }
    }
    assert_eq!(
        version_exes,
        vec![canonical_filename.to_string()],
        "NSIS bundle directory must contain ONLY the canonical installer for v{}, found: {:?}",
        manifest_version,
        version_exes
    );

    // 6. Verify mandatory .sig file on disk
    let sig_file_path = nsis_dir.join(format!("{}.sig", canonical_filename));
    if !sig_file_path.exists() {
        panic!(
            "Mandatory signature file {:?} MUST exist on disk during release verification!",
            sig_file_path
        );
    }
    let sig_file_bytes = fs::read_to_string(&sig_file_path).expect(".sig file must be readable");
    assert_eq!(
        sig_file_bytes.trim(),
        sig_b64.trim(),
        "Manifest signature must match the .sig file generated on disk by the build"
    );

    // 7. Verify cryptographic signature against installer bytes
    let installer_bytes = fs::read(&installer_path).expect("installer file must be readable");
    assert!(!installer_bytes.is_empty(), "Installer file must not be empty");
    let verify_result = public_key.verify(&installer_bytes, &signature, true);
    assert!(
        verify_result.is_ok(),
        "Minisign cryptographic signature verification must succeed: {:?}",
        verify_result.err()
    );

    // 8. Also verify windows-x86_64-nsis signature matches and verifies identically
    let nsis_sig_b64 = manifest["platforms"]["windows-x86_64-nsis"]["signature"]
        .as_str()
        .expect("windows-x86_64-nsis signature must be a string");
    assert_eq!(sig_b64, nsis_sig_b64, "Both platform entries must share identical signature");

    let nsis_sig_raw = BASE64.decode(nsis_sig_b64).expect("nsis signature must be valid base64");
    let nsis_sig_str = String::from_utf8(nsis_sig_raw).expect("nsis signature must be valid UTF-8");
    let nsis_signature =
        Signature::decode(&nsis_sig_str).expect("nsis signature must decode successfully");
    let nsis_verify = public_key.verify(&installer_bytes, &nsis_signature, true);
    assert!(
        nsis_verify.is_ok(),
        "Minisign verification for windows-x86_64-nsis must succeed: {:?}",
        nsis_verify.err()
    );
}

#[test]
fn test_manifest_url_encoding_matches_disk_asset() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent of src-tauri should be workspace root")
        .to_path_buf();

    let manifest_path = workspace_root.join("docs").join("updater-release-template.json");
    let manifest_content =
        fs::read_to_string(&manifest_path).expect("updater-release-template.json must be readable");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_content).expect("manifest must parse as JSON");

    let url = manifest["platforms"]["windows-x86_64"]["url"]
        .as_str()
        .expect("url must be a string");

    let manifest_version = manifest["version"]
        .as_str()
        .expect("manifest version must be string");
    let expected_url_prefix_dot = format!(
        "https://github.com/bazq3D/stardew-sync/releases/download/v{}/Stardew.Sync_{}_x64-setup.exe",
        manifest_version, manifest_version
    );
    let expected_url_prefix_space = format!(
        "https://github.com/bazq3D/stardew-sync/releases/download/v{}/Stardew%20Sync_{}_x64-setup.exe",
        manifest_version, manifest_version
    );

    assert!(
        url.starts_with(&expected_url_prefix_dot) || url.starts_with(&expected_url_prefix_space),
        "URL must match canonical repository and correctly normalized/encoded filename: {}",
        url
    );

    // Also verify nsis fallback target key
    let nsis_url = manifest["platforms"]["windows-x86_64-nsis"]["url"]
        .as_str()
        .expect("windows-x86_64-nsis url must be a string");
    assert_eq!(url, nsis_url);
}

#[test]
#[ignore = "Historical v0.1.2 verification requires setting STARDEW_V0_1_2_VERIFY_DIR"]
fn test_downloaded_github_release_cryptographic_parity() {
    let verify_dir_str = std::env::var("STARDEW_V0_1_2_VERIFY_DIR")
        .expect("STARDEW_V0_1_2_VERIFY_DIR must be set to run historical v0.1.2 verification");
    let temp_verify_dir = PathBuf::from(verify_dir_str);
    assert!(
        temp_verify_dir.exists(),
        "Specified STARDEW_V0_1_2_VERIFY_DIR does not exist: {:?}",
        temp_verify_dir
    );

    let installer_path = temp_verify_dir.join("Stardew.Sync_0.1.2_x64-setup.exe");
    let sig_path = temp_verify_dir.join("Stardew.Sync_0.1.2_x64-setup.exe.sig");
    let manifest_path = temp_verify_dir.join("latest.json");

    assert!(installer_path.exists(), "Historical installer missing: {:?}", installer_path);
    assert!(sig_path.exists(), "Historical signature missing: {:?}", sig_path);
    assert!(manifest_path.exists(), "Historical manifest missing: {:?}", manifest_path);

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent of src-tauri should be workspace root")
        .to_path_buf();

    let tauri_conf_path = workspace_root.join("src-tauri").join("tauri.conf.json");
    let tauri_conf_content = fs::read_to_string(&tauri_conf_path).expect("tauri.conf.json");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_content).expect("tauri.conf.json json");
    let pubkey_b64 = tauri_conf["plugins"]["updater"]["pubkey"].as_str().unwrap();
    let pubkey_raw = BASE64.decode(pubkey_b64).unwrap();
    let pubkey_str = String::from_utf8(pubkey_raw).unwrap();
    let public_key = PublicKey::decode(&pubkey_str).unwrap();

    let sig_str = fs::read_to_string(&sig_path).unwrap();
    let sig_raw = BASE64.decode(sig_str.trim()).unwrap();
    let sig_text = String::from_utf8(sig_raw).unwrap();
    let signature = Signature::decode(&sig_text).unwrap();
    let installer_bytes = fs::read(&installer_path).unwrap();

    let verify_result = public_key.verify(&installer_bytes, &signature, true);
    assert!(
        verify_result.is_ok(),
        "Cryptographic signature verification of published GitHub asset failed: {:?}",
        verify_result.err()
    );

    // Also verify manifest signature
    let manifest_content = fs::read_to_string(&manifest_path).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).unwrap();
    assert!(!manifest["version"].as_str().unwrap().is_empty());
    let manifest_sig_b64 = manifest["platforms"]["windows-x86_64"]["signature"].as_str().unwrap();
    let manifest_sig_raw = BASE64.decode(manifest_sig_b64).unwrap();
    let manifest_sig_str = String::from_utf8(manifest_sig_raw).unwrap();
    let manifest_signature = Signature::decode(&manifest_sig_str).unwrap();
    let manifest_verify = public_key.verify(&installer_bytes, &manifest_signature, true);
    assert!(
        manifest_verify.is_ok(),
        "Cryptographic signature in latest.json failed to verify installer: {:?}",
        manifest_verify.err()
    );
}

#[test]
fn test_historical_v0_1_3_regression_signature_verification() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent of src-tauri should be workspace root")
        .to_path_buf();

    // 1. Read updater public key embedded in tauri.conf.json
    let tauri_conf_path = workspace_root.join("src-tauri").join("tauri.conf.json");
    let tauri_conf_content = fs::read_to_string(&tauri_conf_path).expect("tauri.conf.json must be readable");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_content).expect("tauri.conf.json must parse as JSON");
    let pubkey_b64 = tauri_conf["plugins"]["updater"]["pubkey"].as_str().expect("pubkey missing");
    let pubkey_raw = BASE64.decode(pubkey_b64).expect("valid base64 pubkey");
    let pubkey_str = String::from_utf8(pubkey_raw).expect("valid utf8 pubkey");
    let public_key = PublicKey::decode(&pubkey_str).expect("decode minisign public key");

    // 2. Read repository-owned v0.1.3 release template manifest
    let template_path = workspace_root.join("docs").join("updater-release-template.json");
    let template_content = fs::read_to_string(&template_path).expect("updater-release-template.json must be readable");
    let manifest: serde_json::Value = serde_json::from_str(&template_content).expect("manifest must parse as JSON");

    assert_eq!(manifest["version"].as_str().unwrap(), "0.1.3", "Historical fixture version must be 0.1.3");
    let sig_b64 = manifest["platforms"]["windows-x86_64"]["signature"].as_str().unwrap();
    let sig_raw = BASE64.decode(sig_b64).expect("valid base64 signature");
    let sig_str = String::from_utf8(sig_raw).expect("valid utf8 signature");
    let signature = Signature::decode(&sig_str).expect("decode minisign signature");

    let trusted_comment = signature.trusted_comment();
    assert!(trusted_comment.contains("version:0.1.3"), "Trusted comment must contain version:0.1.3: {}", trusted_comment);
    assert!(
        trusted_comment.contains("file:Stardew.Sync_0.1.3_x64-setup.exe") || trusted_comment.contains("file:Stardew Sync_0.1.3_x64-setup.exe"),
        "Trusted comment must contain file binding: {}",
        trusted_comment
    );

    // 3. Resolve installer artifact path (explicitly supplied path or local build bundle)
    let artifact_path = if let Ok(custom_path) = std::env::var("STARDEW_V0_1_3_INSTALLER_PATH") {
        let custom = PathBuf::from(custom_path);
        assert!(
            custom.exists(),
            "Explicitly supplied STARDEW_V0_1_3_INSTALLER_PATH does not exist: {:?}",
            custom
        );
        Some(custom)
    } else {
        let bundle_path = workspace_root
            .join("target")
            .join("release")
            .join("bundle")
            .join("nsis")
            .join("Stardew.Sync_0.1.3_x64-setup.exe");
        if bundle_path.exists() {
            Some(bundle_path)
        } else {
            None
        }
    };

    // If an explicit path was supplied or local artifact exists, verify byte-level cryptographic signature and hash
    if let Some(installer_path) = artifact_path {
        let installer_bytes = fs::read(&installer_path).expect("installer artifact must be readable");
        assert_eq!(
            installer_bytes.len(),
            4024280,
            "Authoritative v0.1.3 installer must be exactly 4024280 bytes, found {}",
            installer_bytes.len()
        );

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&installer_bytes);
        let hash_hex = format!("{:x}", hasher.finalize());
        assert_eq!(
            hash_hex,
            "af99cce49587857f78fff8ca6d909664841063c6fdc64bdb4bd072bdb31d55f6",
            "Authoritative v0.1.3 installer SHA-256 mismatch"
        );

        let verify_result = public_key.verify(&installer_bytes, &signature, true);
        assert!(
            verify_result.is_ok(),
            "Cryptographic signature verification failed against authoritative v0.1.3 installer: {:?}",
            verify_result.err()
        );
    } else {
        // If the artifact was explicitly requested via REQUIRE_INSTALLER_VERIFY, fail closed
        if std::env::var("REQUIRE_INSTALLER_VERIFY").is_ok() {
            panic!("Required v0.1.3 verification artifact is missing! Set STARDEW_V0_1_3_INSTALLER_PATH or ensure bundle installer exists.");
        }
    }
}
