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

    assert_eq!(manifest["version"], "0.1.1");

    let sig_b64 = manifest["platforms"]["windows-x86_64"]["signature"]
        .as_str()
        .expect("signature must be a string");

    let sig_raw = BASE64.decode(sig_b64).expect("signature must be valid base64");
    let sig_str = String::from_utf8(sig_raw).expect("signature must be valid UTF-8");

    let signature =
        Signature::decode(&sig_str).expect("Signature must decode successfully from minisign string");

    // 3. Verify trusted comment
    let trusted_comment = signature.trusted_comment();
    assert!(
        trusted_comment.contains("version:0.1.1"),
        "Trusted comment must contain version:0.1.1: {}",
        trusted_comment
    );
    assert!(
        trusted_comment.contains("file:Stardew Sync_0.1.1_x64-setup.exe"),
        "Trusted comment must contain file:Stardew Sync_0.1.1_x64-setup.exe: {}",
        trusted_comment
    );

    // 4. Verify actual installer bytes
    let installer_path = workspace_root
        .join("target")
        .join("release")
        .join("bundle")
        .join("nsis")
        .join("Stardew Sync_0.1.1_x64-setup.exe");

    assert!(
        installer_path.exists(),
        "Built installer artifact must exist at: {}",
        installer_path.display()
    );

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

    assert!(
        url.starts_with("https://github.com/bazq3D/stardew-sync/releases/download/v0.1.1/Stardew%20Sync_0.1.1_x64-setup.exe"),
        "URL must match canonical repository and correctly URL-encoded filename: {}",
        url
    );

    // Also verify nsis fallback target key
    let nsis_url = manifest["platforms"]["windows-x86_64-nsis"]["url"]
        .as_str()
        .expect("windows-x86_64-nsis url must be a string");
    assert_eq!(url, nsis_url);
}
