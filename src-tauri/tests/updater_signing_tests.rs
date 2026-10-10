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

    // 2. Read manifest template
    let manifest_path = workspace_root.join("docs").join("updater-release-template.json");
    let manifest_content =
        fs::read_to_string(&manifest_path).expect("updater-release-template.json must be readable");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_content).expect("manifest must parse as JSON");

    let manifest_version = manifest["version"]
        .as_str()
        .expect("manifest version must be string");

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
    let expected_file_str = format!("file:Stardew Sync_{}_x64-setup.exe", manifest_version);
    assert!(
        trusted_comment.contains(&expected_version_str),
        "Trusted comment must contain {}: {}",
        expected_version_str,
        trusted_comment
    );
    assert!(
        trusted_comment.contains(&expected_file_str),
        "Trusted comment must contain {}: {}",
        expected_file_str,
        trusted_comment
    );

    // 4. Verify actual installer bytes
    let installer_name = format!("Stardew Sync_{}_x64-setup.exe", manifest_version);
    let installer_path = workspace_root
        .join("target")
        .join("release")
        .join("bundle")
        .join("nsis")
        .join(&installer_name);

    if !installer_path.exists() {
        println!(
            "Installer {} does not exist yet on disk; skipping byte-level verification until build produces it.",
            installer_name
        );
        return;
    }

    let installer_bytes = fs::read(&installer_path).expect("installer file must be readable");

    // Verify cryptographic signature against installer bytes
    let verify_result = public_key.verify(&installer_bytes, &signature, true);
    assert!(
        verify_result.is_ok(),
        "Minisign cryptographic signature verification must succeed: {:?}",
        verify_result.err()
    );

    // Also verify windows-x86_64-nsis signature matches and verifies identically
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
fn test_downloaded_github_release_cryptographic_parity() {
    let temp_verify_dir = std::env::temp_dir().join("stardew_release_verify");
    if !temp_verify_dir.exists() {
        return;
    }
    let installer_path = temp_verify_dir.join("Stardew.Sync_0.1.2_x64-setup.exe");
    let sig_path = temp_verify_dir.join("Stardew.Sync_0.1.2_x64-setup.exe.sig");
    let manifest_path = temp_verify_dir.join("latest.json");

    if !installer_path.exists() || !sig_path.exists() || !manifest_path.exists() {
        return;
    }

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
