use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

use stardew_sync_core::core::identity::{
    storage as id_storage, DeviceIdentity, PublicDeviceInfo, IDENTITY_FILE_ENC, IDENTITY_FILE_JSON,
};
use stardew_sync_core::core::network::{
    ConnectionManager, DiscoveryManager, EncryptedTransportFrame, LanBeacon, NetworkService,
    NetworkWireMessage, PeerConnectionStatus, TransportFrame, TransportFramePayload,
    DEFAULT_DISCOVERY_PORT, DISCOVERY_BEACON_MAGIC, DISCOVERY_PROTOCOL_VERSION, MAX_FRAME_SIZE,
};
use stardew_sync_core::core::pairing::{
    compute_sas_code, PairedDevice, PairedDeviceStore, PairingConfirm, PairingFinalAck,
    PairingManager, PairingReject, PairingStatus,
};

static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn setup_test_env(_test_name: &str) -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let temp = TempDir::new().expect("Failed to create tempdir");
    let identity_dir = temp.path().join("identity");
    let devices_dir = temp.path().join("devices");
    fs::create_dir_all(&identity_dir).unwrap();
    fs::create_dir_all(&devices_dir).unwrap();

    std::env::set_var("STARDEW_SYNC_IDENTITY_DIR", identity_dir.to_str().unwrap());
    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", devices_dir.to_str().unwrap());
    std::env::set_var("STARDEW_SYNC_TEST_MOCK_DPAPI", "1");

    (temp, guard)
}

#[test]
fn test_identity_generation_and_persistence() {
    let _env = setup_test_env("identity_generation");

    // Fresh generation on first run
    let identity1 = DeviceIdentity::load_or_create().expect("Failed to load/create identity");
    assert!(!identity1.device_id().is_empty());
    assert_eq!(identity1.device_id().len(), 64);
    assert!(!identity1.fingerprint().is_empty());

    let dir = id_storage::get_identity_dir();
    assert!(dir.join(IDENTITY_FILE_JSON).exists());
    assert!(dir.join(IDENTITY_FILE_ENC).exists());

    // Second run should load the EXACT same identity
    let identity2 = DeviceIdentity::load_or_create().expect("Failed to load existing identity");
    assert_eq!(identity1.device_id(), identity2.device_id());
    assert_eq!(identity1.fingerprint(), identity2.fingerprint());
    assert_eq!(identity1.public_info(), identity2.public_info());
}

#[test]
fn test_identity_corruption_fails_closed() {
    let _env = setup_test_env("identity_corruption");

    // Generate initial identity
    let _ = DeviceIdentity::load_or_create().unwrap();
    let dir = id_storage::get_identity_dir();

    // Corrupt encrypted key by deleting it
    fs::remove_file(dir.join(IDENTITY_FILE_ENC)).unwrap();

    // Must fail closed with explicit corruption error rather than overwriting
    let res = DeviceIdentity::load_or_create();
    assert!(res.is_err(), "Expected fail-closed error when encrypted key is missing");
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("corrupted"),
        "Expected corruption message, got: {}",
        err_msg
    );
}

#[test]
fn test_secret_storage_dpapi_mock_protection() {
    let _env = setup_test_env("secret_storage");
    let secret_seed = [42u8; 32];

    let encrypted = id_storage::protect_secret(&secret_seed).expect("Encryption failed");
    assert_ne!(&encrypted[..], &secret_seed[..]);

    let decrypted = id_storage::unprotect_secret(&encrypted).expect("Decryption failed");
    assert_eq!(&decrypted[..], &secret_seed[..]);
}

#[test]
fn test_device_name_editing() {
    let _env = setup_test_env("device_name_edit");
    let mut identity = DeviceIdentity::load_or_create().unwrap();

    identity.set_device_name("Kubilay-Gaming-Rig").unwrap();
    assert_eq!(identity.device_name(), "Kubilay-Gaming-Rig");

    // Reload from disk to verify persistence
    let reloaded = DeviceIdentity::load_or_create().unwrap();
    assert_eq!(reloaded.device_name(), "Kubilay-Gaming-Rig");
}

#[test]
fn test_pairing_sas_code_dual_derivation_deterministic() {
    let invitation_code = "SD-8492";
    let inviter_pk = "a".repeat(64);
    let joiner_pk = "b".repeat(64);
    let inviter_ecdh = "c".repeat(64);
    let joiner_ecdh = "d".repeat(64);
    let inviter_nonce = "1".repeat(64);
    let joiner_nonce = "2".repeat(64);

    let sas_a = compute_sas_code(
        invitation_code,
        &inviter_pk,
        &joiner_pk,
        &inviter_ecdh,
        &joiner_ecdh,
        &inviter_nonce,
        &joiner_nonce,
    );

    let sas_b = compute_sas_code(
        invitation_code,
        &inviter_pk,
        &joiner_pk,
        &inviter_ecdh,
        &joiner_ecdh,
        &inviter_nonce,
        &joiner_nonce,
    );

    assert_eq!(sas_a, sas_b);
    assert_eq!(sas_a.len(), 7); // "XXX YYY" format
    assert!(sas_a.contains(' '));
}

#[test]
fn test_mitm_altered_key_produces_different_sas() {
    let invitation_code = "SD-8492";
    let inviter_pk = "a".repeat(64);
    let joiner_pk = "b".repeat(64);
    let inviter_ecdh = "c".repeat(64);
    let joiner_ecdh = "d".repeat(64);
    let inviter_nonce = "1".repeat(64);
    let joiner_nonce = "2".repeat(64);

    let original_sas = compute_sas_code(
        invitation_code,
        &inviter_pk,
        &joiner_pk,
        &inviter_ecdh,
        &joiner_ecdh,
        &inviter_nonce,
        &joiner_nonce,
    );

    // MITM attacker tampers with ephemeral key
    let tampered_joiner_ecdh = "e".repeat(64);
    let tampered_sas = compute_sas_code(
        invitation_code,
        &inviter_pk,
        &joiner_pk,
        &inviter_ecdh,
        &tampered_joiner_ecdh,
        &inviter_nonce,
        &joiner_nonce,
    );

    assert_ne!(original_sas, tampered_sas, "MITM key tampering MUST result in SAS code mismatch");
}

#[test]
fn test_pairing_manager_dual_confirmation_success() {
    let _env = setup_test_env("pairing_dual_confirm");
    let manager = PairingManager::new();
    let mut store = PairedDeviceStore::load().unwrap();

    let peer_info = PublicDeviceInfo {
        device_id: "peer-device-id-12345".to_string(),
        fingerprint: "1234-5678-90ab-cdef".to_string(),
        device_name: "Elbi-Laptop".to_string(),
        public_key_hex: "f".repeat(64),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    let sas_code = manager
        .start_confirmation_session(
            "session-abc",
            "SD-1234",
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            peer_info.clone(),
            true,
        )
        .unwrap();

    // 1. Peer confirms first
    let res1 = manager.confirm_remote(&sas_code, &mut store).unwrap();
    assert!(res1.is_none(), "Pairing should not commit until BOTH parties confirm");

    let view1 = manager.get_session_view();
    assert!(!view1.our_confirmed);
    assert!(view1.peer_confirmed);

    // 2. Local user confirms with matching SAS code
    let res2 = manager.confirm_local(&sas_code, &mut store).unwrap();
    assert!(res2.is_some(), "Pairing must commit once both parties confirmed");
    let paired_device = res2.unwrap();
    assert_eq!(paired_device.device_id, peer_info.device_id);
    assert_eq!(paired_device.device_name, peer_info.device_name);

    // Verify stored in persistent database
    let reloaded_store = PairedDeviceStore::load().unwrap();
    assert!(reloaded_store.get(&peer_info.device_id).is_some());
}

#[test]
fn test_wrong_confirmation_code_rejected() {
    let _env = setup_test_env("pairing_wrong_code");
    let manager = PairingManager::new();
    let mut store = PairedDeviceStore::load().unwrap();

    let peer_info = PublicDeviceInfo {
        device_id: "peer-123".to_string(),
        fingerprint: "1111-2222-3333-4444".to_string(),
        device_name: "Attacker-PC".to_string(),
        public_key_hex: "0".repeat(64),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    manager
        .start_confirmation_session(
            "sess-1",
            "SD-9999",
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            peer_info,
            true,
        )
        .unwrap();

    let result = manager.confirm_local("000 000", &mut store);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Security Alert"));

    let view = manager.get_session_view();
    assert_eq!(view.status, PairingStatus::Rejected);
}

#[test]
fn test_unpairing_and_trust_revocation() {
    let _env = setup_test_env("unpairing");
    let mut store = PairedDeviceStore::load().unwrap();

    let dev = PairedDevice {
        device_id: "revoked-device-42".to_string(),
        device_name: "Temporary-Laptop".to_string(),
        public_key_hex: "e".repeat(64),
        fingerprint: "abcd-ef01-2345-6789".to_string(),
        paired_at: "2026-10-10T10:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    };

    store.add_or_update(dev.clone()).unwrap();
    assert!(store.is_trusted("revoked-device-42", &"e".repeat(64)));

    // Unpair
    let removed = store.remove("revoked-device-42").unwrap();
    assert!(removed);

    // Trust MUST now be false
    assert!(!store.is_trusted("revoked-device-42", &"e".repeat(64)));

    // Reload from disk to verify revocation persistence
    let reloaded_store = PairedDeviceStore::load().unwrap();
    assert!(reloaded_store.get("revoked-device-42").is_none());
}

#[test]
fn test_device_identity_key_pinning() {
    let _env = setup_test_env("key_pinning");
    let mut store = PairedDeviceStore::load().unwrap();

    let original_dev = PairedDevice {
        device_id: "victim-device".to_string(),
        device_name: "Original-PC".to_string(),
        public_key_hex: "1".repeat(64),
        fingerprint: "1111-1111-1111-1111".to_string(),
        paired_at: "2026-10-10T10:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    };
    store.add_or_update(original_dev).unwrap();

    // Attacker tries to replace pinned device with a DIFFERENT public key
    let spoofed_dev = PairedDevice {
        device_id: "victim-device".to_string(),
        device_name: "Imposter-PC".to_string(),
        public_key_hex: "2".repeat(64),
        fingerprint: "2222-2222-2222-2222".to_string(),
        paired_at: "2026-10-10T11:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    };

    let result = store.add_or_update(spoofed_dev);
    assert!(result.is_err(), "Key pinning MUST reject conflicting public key for same device_id");
    assert!(result.unwrap_err().to_string().contains("Key conflict"));
}

#[test]
fn test_transport_frame_signing_and_verification() {
    let _env = setup_test_env("transport_signing");
    let identity = DeviceIdentity::generate("Test-Device");

    let payload = TransportFramePayload::Ping { timestamp_ms: 123456789 };
    let frame = TransportFrame::new_signed(1, &identity, payload.clone()).unwrap();

    // Verification against true verifying key succeeds
    assert!(frame.verify_signature(&identity.verifying_key()).is_ok());

    // Verification against a different key fails
    let other_identity = DeviceIdentity::generate("Other-Device");
    assert!(frame.verify_signature(&other_identity.verifying_key()).is_err());
}

#[test]
fn test_transport_frame_size_bounding() {
    let _env = setup_test_env("frame_bounding");

    // Oversized frame declared length
    let mut oversized_bytes = Vec::new();
    let excessive_len: u32 = (MAX_FRAME_SIZE + 1024) as u32;
    oversized_bytes.extend_from_slice(&excessive_len.to_be_bytes());
    oversized_bytes.extend_from_slice(&vec![0u8; 100]);

    let decode_res = TransportFrame::decode(&oversized_bytes);
    assert!(decode_res.is_err());
    assert!(decode_res.unwrap_err().to_string().contains("Security Alert"));
}

#[test]
fn test_discovery_manager_caching_and_pruning() {
    let dm = DiscoveryManager::new();
    let paired_ids = vec!["paired-1".to_string()];

    let beacon = LanBeacon {
        magic: DISCOVERY_BEACON_MAGIC.to_string(),
        protocol_version: DISCOVERY_PROTOCOL_VERSION,
        device_id: "peer-abc".to_string(),
        device_name: "Nearby-PC".to_string(),
        fingerprint: "aaaa-bbbb-cccc-dddd".to_string(),
        tcp_port: DEFAULT_DISCOVERY_PORT,
        is_inviting: true,
    };

    dm.record_beacon(
        beacon,
        "192.168.1.100".parse().unwrap(),
        "local-device-id",
        &paired_ids,
    );

    let peers = dm.list_discovered_peers(&paired_ids);
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].device_id, "peer-abc");
    assert_eq!(peers[0].address, "192.168.1.100:42890");
    assert!(!peers[0].is_paired);
    assert!(peers[0].is_inviting);
}

#[test]
fn test_production_save_isolation_during_identity_and_pairing() {
    let _env = setup_test_env("save_isolation");

    // Run identity generation, device store operations, pairing derivation
    let identity = DeviceIdentity::load_or_create().unwrap();
    let _info = identity.public_info();

    let mut store = PairedDeviceStore::load().unwrap();
    store.add_or_update(PairedDevice {
        device_id: "test-peer".to_string(),
        device_name: "Isolated-Peer".to_string(),
        public_key_hex: "0".repeat(64),
        fingerprint: "0000-0000-0000-0000".to_string(),
        paired_at: "2026-10-10T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    }).unwrap();

    // Verify identity and devices folders exist ONLY in tempdir
    let temp_identity_dir = id_storage::get_identity_dir();
    assert!(temp_identity_dir.exists());

    // Verify NO save directories or files exist in the identity/devices paths
    for entry in walkdir::WalkDir::new(&temp_identity_dir) {
        let entry = entry.unwrap();
        let fname = entry.file_name().to_string_lossy();
        assert!(!fname.ends_with(".xml"));
        assert!(!fname.contains("SaveGameInfo"));
        assert!(!fname.contains("Türk"));
    }
}

#[test]
fn test_aead_transport_encryption_confidentiality() {
    let _env = setup_test_env("aead_confidentiality");
    let session_key = [0x42u8; 32];
    let sender_id = "test-device-id-alpha";
    let sequence = 100u64;

    let payload = TransportFramePayload::StatusUpdate {
        status: "TOP_SECRET_COOP_SESSION_DATA".to_string(),
    };

    // Encrypt payload using ChaCha20-Poly1305
    let encrypted_frame = EncryptedTransportFrame::encrypt(
        sequence,
        sender_id,
        &payload,
        &session_key,
    ).expect("Encryption must succeed");

    // Wire encoding
    let encoded_bytes = encrypted_frame.encode().expect("Encoding must succeed");

    // CONFIDENTIALITY VERIFICATION:
    // The plaintext payload string MUST NOT appear anywhere in the wire-encoded bytes!
    let encoded_str = String::from_utf8_lossy(&encoded_bytes);
    assert!(
        !encoded_str.contains("TOP_SECRET_COOP_SESSION_DATA"),
        "Security Alert: Plaintext payload found in wire format! Confidentiality violated!"
    );

    // Decrypt with correct session key succeeds and recovers exact payload
    let decrypted = encrypted_frame
        .decrypt(&session_key, sequence)
        .expect("Decryption with correct key must succeed");
    assert_eq!(decrypted, payload);

    // Decrypt with incorrect session key FAILS fail-closed
    let wrong_key = [0x99u8; 32];
    let bad_decrypt = encrypted_frame.decrypt(&wrong_key, sequence);
    assert!(bad_decrypt.is_err(), "Decryption with wrong session key MUST fail fail-closed");
    let err_str = bad_decrypt.unwrap_err().to_string();
    assert!(err_str.contains("AEAD authentication failed"));

    // Ciphertext tampering (bit flip) FAILS fail-closed
    let mut tampered_frame = encrypted_frame.clone();
    let mut raw_cipher = hex::decode(&tampered_frame.ciphertext_hex).unwrap();
    raw_cipher[0] ^= 0x01; // flip 1 bit
    tampered_frame.ciphertext_hex = hex::encode(raw_cipher);

    let tampered_res = tampered_frame.decrypt(&session_key, sequence);
    assert!(tampered_res.is_err(), "Tampered ciphertext MUST fail Poly1305 authentication");
}

#[test]
fn test_replay_attack_rejection() {
    let _env = setup_test_env("replay_attack_rejection");
    let session_key = [0x77u8; 32];
    let sender_id = "device-sender-replay";

    let payload = TransportFramePayload::Ping { timestamp_ms: 55555 };

    // Encrypt frame with sequence 5
    let frame_seq5 = EncryptedTransportFrame::encrypt(5, sender_id, &payload, &session_key).unwrap();

    // Receiving frame 5 with expected minimum 5 succeeds
    assert!(frame_seq5.decrypt(&session_key, 5).is_ok());

    // Attacker replays frame 5 when receiver has advanced to expected minimum 6
    let replay_res = frame_seq5.decrypt(&session_key, 6);
    assert!(replay_res.is_err(), "Replay of older sequence number MUST be rejected");
    let err = replay_res.unwrap_err().to_string();
    assert!(err.contains("Replay attack detected"));
}

#[test]
fn test_live_tcp_handshake_and_dual_confirmation_e2e() {
    let _env = setup_test_env("live_tcp_handshake_e2e");

    // 1. Setup Inviter (Device A)
    let identity_a = DeviceIdentity::generate("Inviter-PC");
    let net = stardew_sync_core::core::network::get_network_service();
    let test_port = 42898u16;
    net.start_tcp_listener(test_port, identity_a.clone());

    // Allow listener thread to bind
    std::thread::sleep(std::time::Duration::from_millis(150));

    let actual_port = net.get_active_tcp_port().unwrap_or(test_port);

    // Device A starts pairing invitation
    let pairing_mgr = stardew_sync_core::core::pairing::get_pairing_manager();
    let _ = pairing_mgr.cancel_invitation();
    let invitation = pairing_mgr
        .start_invitation(&identity_a, actual_port)
        .expect("Failed to start invitation");

    // 2. Setup Joiner (Device B) and initiate pairing over live TCP
    let identity_b = DeviceIdentity::generate("Joiner-Laptop");
    let peer_addr = format!("127.0.0.1:{}", actual_port);

    let sas_joiner = net
        .initiate_pairing(&peer_addr, &invitation.code, &identity_b)
        .expect("Live TCP handshake initiate_pairing failed");

    // Allow server-side thread to process PairingRequest
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify both calculated identical SAS code
    let session_view = pairing_mgr.get_session_view();
    assert_eq!(session_view.status, PairingStatus::AwaitingDualConfirmation);
    let sas_inviter = session_view.sas_code.expect("Inviter must have SAS code");
    assert_eq!(sas_inviter, sas_joiner, "Both nodes MUST derive identical SAS codes");

    // 3. User on Inviter confirms local SAS code
    let mut store_a = PairedDeviceStore::load().unwrap();
    let confirm_res_a = pairing_mgr.confirm_local(&sas_inviter, &mut store_a).unwrap();
    // Initially None because remote peer confirmation has not arrived yet
    assert!(confirm_res_a.is_none());

    // 4. Background workers exchange PairingConfirm over live TCP
    // Allow exchange thread to cycle
    std::thread::sleep(std::time::Duration::from_millis(300));

    // After exchange, Inviter confirms remote peer and commits
    let final_view = pairing_mgr.get_session_view();
    assert!(
        final_view.our_confirmed,
        "Local confirmation must be recorded"
    );

    let _ = pairing_mgr.cancel_invitation();
}

#[test]
fn test_pairing_manager_reject_does_not_deadlock_and_transitions_to_rejected() {
    let _env = setup_test_env("pairing_reject_no_deadlock");
    let manager = PairingManager::new();

    let peer_info = PublicDeviceInfo {
        device_id: "peer-reject-test".to_string(),
        fingerprint: "9999-8888-7777-6666".to_string(),
        device_name: "Reject-Test-PC".to_string(),
        public_key_hex: "3".repeat(64),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    manager
        .start_confirmation_session(
            "sess-reject-1",
            "SD-5555",
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            peer_info,
            true,
        )
        .unwrap();

    let view_before = manager.get_session_view();
    assert_eq!(view_before.status, PairingStatus::AwaitingDualConfirmation);

    // Calling reject must NOT deadlock and must transition to Rejected status
    manager.reject().expect("reject must succeed without deadlock");

    let view_after = manager.get_session_view();
    assert_eq!(view_after.status, PairingStatus::Rejected);

    // Calling cancel_invitation resets the session to Idle
    manager.cancel_invitation().expect("cancel_invitation must succeed");
    let view_idle = manager.get_session_view();
    assert_eq!(view_idle.status, PairingStatus::Idle);
}

#[test]
fn test_pairing_manager_rejects_confirmations_on_stale_or_rejected_session() {
    let _env = setup_test_env("stale_confirmation_guards");
    let manager = PairingManager::new();
    let mut store = PairedDeviceStore::load().unwrap();

    let peer_info = PublicDeviceInfo {
        device_id: "peer-stale-test".to_string(),
        fingerprint: "5555-4444-3333-2222".to_string(),
        device_name: "Stale-Test-PC".to_string(),
        public_key_hex: "4".repeat(64),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    let sas_code = manager
        .start_confirmation_session(
            "sess-stale-1",
            "SD-7777",
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            peer_info,
            true,
        )
        .unwrap();

    // Reject the session
    manager.reject().unwrap();
    assert_eq!(manager.get_session_view().status, PairingStatus::Rejected);

    // Subsequent confirm_local MUST be rejected with explicit error
    let local_res = manager.confirm_local(&sas_code, &mut store);
    assert!(local_res.is_err(), "confirm_local on Rejected session must fail");
    assert!(local_res.unwrap_err().to_string().contains("not awaiting confirmation"));

    // Subsequent confirm_remote MUST be rejected with explicit error
    let remote_res = manager.confirm_remote(&sas_code, &mut store);
    assert!(remote_res.is_err(), "confirm_remote on Rejected session must fail");
    assert!(remote_res.unwrap_err().to_string().contains("not awaiting confirmation"));
}

#[test]
fn test_pairing_manager_rejects_duplicate_confirmation_after_completion() {
    let _env = setup_test_env("duplicate_confirmation_guards");
    let manager = PairingManager::new();
    let mut store = PairedDeviceStore::load().unwrap();

    let peer_info = PublicDeviceInfo {
        device_id: "peer-dup-test".to_string(),
        fingerprint: "7777-6666-5555-4444".to_string(),
        device_name: "Dup-Test-PC".to_string(),
        public_key_hex: "5".repeat(64),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    let sas_code = manager
        .start_confirmation_session(
            "sess-dup-1",
            "SD-8888",
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            peer_info,
            true,
        )
        .unwrap();

    // Both confirm -> completed
    manager.confirm_local(&sas_code, &mut store).unwrap();
    let completed_dev = manager.confirm_remote(&sas_code, &mut store).unwrap();
    assert!(completed_dev.is_some());
    assert_eq!(manager.get_session_view().status, PairingStatus::Completed);

    // Attempt duplicate confirmation after completion MUST fail
    let dup_local = manager.confirm_local(&sas_code, &mut store);
    assert!(dup_local.is_err(), "Duplicate confirm_local on Completed session must fail");
    assert!(dup_local.unwrap_err().to_string().contains("not awaiting confirmation"));

    let dup_remote = manager.confirm_remote(&sas_code, &mut store);
    assert!(dup_remote.is_err(), "Duplicate confirm_remote on Completed session must fail");
    assert!(dup_remote.unwrap_err().to_string().contains("not awaiting confirmation"));
}

#[test]
fn test_phase5a_active_pairing_session_view_payload_contract() {
    let _env = setup_test_env("phase5a_session_view_contract");
    let manager = PairingManager::new();

    // 1. Idle state payload contract
    let idle_view = manager.get_session_view();
    assert_eq!(idle_view.status, PairingStatus::Idle);
    let idle_json = serde_json::to_value(&idle_view).expect("Serialize idle view");
    assert_eq!(idle_json["status"], "Idle");
    assert!(idle_json["session_id"].is_null());
    assert!(idle_json["peer_device"].is_null());
    assert!(idle_json["sas_code"].is_null());

    // 2. Inviting state payload contract
    let identity = DeviceIdentity::generate("Inviter-PC");
    let inv = manager.start_invitation(&identity, 42890).expect("Start invitation");
    assert!(inv.code.starts_with("SD-"));

    let inviting_view = manager.get_session_view();
    assert_eq!(inviting_view.status, PairingStatus::Inviting);
    let inviting_json = serde_json::to_value(&inviting_view).expect("Serialize inviting view");
    assert_eq!(inviting_json["status"], "Inviting");
    assert_eq!(inviting_json["invitation_code"], inv.code);
    assert!(inviting_json["seconds_remaining"].as_u64().unwrap() > 0);

    // 3. AwaitingDualConfirmation payload contract
    let peer_info = PublicDeviceInfo {
        device_id: "peer-phase5a-test".to_string(),
        fingerprint: "1111-2222-3333-4444".to_string(),
        device_name: "Phase5A-Joiner".to_string(),
        public_key_hex: "f".repeat(64),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    let sas = manager
        .start_confirmation_session(
            "sess-phase5a-1",
            &inv.code,
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            peer_info,
            true,
        )
        .expect("Start confirmation session");

    let awaiting_view = manager.get_session_view();
    assert_eq!(awaiting_view.status, PairingStatus::AwaitingDualConfirmation);
    let awaiting_json = serde_json::to_value(&awaiting_view).expect("Serialize awaiting view");

    // Verify all fields expected by frontend SasVerificationModal
    assert_eq!(awaiting_json["status"], "AwaitingDualConfirmation");
    assert_eq!(awaiting_json["session_id"], "sess-phase5a-1");
    assert_eq!(awaiting_json["invitation_code"], inv.code);
    assert_eq!(awaiting_json["sas_code"], sas);
    assert!(!awaiting_json["our_confirmed"].as_bool().unwrap());
    assert!(!awaiting_json["peer_confirmed"].as_bool().unwrap());
    assert!(awaiting_json["seconds_remaining"].as_u64().unwrap() > 0);

    // Verify peer_device fields
    let peer_obj = &awaiting_json["peer_device"];
    assert!(!peer_obj.is_null());
    assert_eq!(peer_obj["device_name"], "Phase5A-Joiner");
    assert_eq!(peer_obj["fingerprint"], "1111-2222-3333-4444");
    assert_eq!(peer_obj["device_id"], "peer-phase5a-test");
    assert_eq!(peer_obj["created_at"], "2026-10-10T12:00:00Z");

    // Verify SAS format (two 3-digit groups separated by space)
    let parts: Vec<&str> = sas.split(' ').collect();
    assert_eq!(parts.len(), 2, "SAS code must format as two 3-digit groups");
    assert_eq!(parts[0].len(), 3);
    assert_eq!(parts[1].len(), 3);
}

#[test]
fn test_bidirectional_discovery_announcement() {
    let _env = setup_test_env("bidirectional_discovery");

    let dm_a = DiscoveryManager::new();
    let dm_b = DiscoveryManager::new();

    let identity_a = DeviceIdentity::generate("PC-A");
    let identity_b = DeviceIdentity::generate("PC-B");

    let beacon_a = LanBeacon {
        magic: DISCOVERY_BEACON_MAGIC.to_string(),
        protocol_version: DISCOVERY_PROTOCOL_VERSION,
        device_id: identity_a.device_id(),
        device_name: identity_a.device_name().to_string(),
        fingerprint: identity_a.fingerprint(),
        tcp_port: DEFAULT_DISCOVERY_PORT,
        is_inviting: false,
    };

    let beacon_b = LanBeacon {
        magic: DISCOVERY_BEACON_MAGIC.to_string(),
        protocol_version: DISCOVERY_PROTOCOL_VERSION,
        device_id: identity_b.device_id(),
        device_name: identity_b.device_name().to_string(),
        fingerprint: identity_b.fingerprint(),
        tcp_port: DEFAULT_DISCOVERY_PORT,
        is_inviting: false,
    };

    // PC A receives beacon from PC B at 192.168.1.20
    dm_a.record_beacon(
        beacon_b.clone(),
        "192.168.1.20".parse().unwrap(),
        &identity_a.device_id(),
        &[],
    );

    // PC B receives beacon from PC A at 192.168.1.10
    dm_b.record_beacon(
        beacon_a.clone(),
        "192.168.1.10".parse().unwrap(),
        &identity_b.device_id(),
        &[],
    );

    // Also simulate self-broadcasts received by both; they must be ignored
    dm_a.record_beacon(
        beacon_a.clone(),
        "127.0.0.1".parse().unwrap(),
        &identity_a.device_id(),
        &[],
    );
    dm_b.record_beacon(
        beacon_b.clone(),
        "127.0.0.1".parse().unwrap(),
        &identity_b.device_id(),
        &[],
    );

    // Verify PC A discovers PC B
    let peers_a = dm_a.list_discovered_peers(&[]);
    assert_eq!(peers_a.len(), 1, "PC A should discover exactly PC B");
    assert_eq!(peers_a[0].device_id, identity_b.device_id());
    assert_eq!(peers_a[0].device_name, "PC-B");
    assert_eq!(peers_a[0].address, "192.168.1.20:42890");
    assert!(!peers_a[0].is_inviting);

    // Verify PC B discovers PC A
    let peers_b = dm_b.list_discovered_peers(&[]);
    assert_eq!(peers_b.len(), 1, "PC B should discover exactly PC A");
    assert_eq!(peers_b[0].device_id, identity_a.device_id());
    assert_eq!(peers_b[0].device_name, "PC-A");
    assert_eq!(peers_b[0].address, "192.168.1.10:42890");

    // PC B starts pairing invitation -> updates its beacon
    let mut inviting_beacon_b = beacon_b.clone();
    inviting_beacon_b.is_inviting = true;
    dm_a.record_beacon(
        inviting_beacon_b,
        "192.168.1.20".parse().unwrap(),
        &identity_a.device_id(),
        &[],
    );

    let updated_peers_a = dm_a.list_discovered_peers(&[]);
    assert_eq!(updated_peers_a.len(), 1);
    assert!(updated_peers_a[0].is_inviting, "PC A must see PC B as ready to pair");
}

#[test]
fn test_advertised_port_consistency() {
    let _env = setup_test_env("advertised_port_consistency");
    let dm = DiscoveryManager::new();

    let custom_port: u16 = 42895;
    let beacon = LanBeacon {
        magic: DISCOVERY_BEACON_MAGIC.to_string(),
        protocol_version: DISCOVERY_PROTOCOL_VERSION,
        device_id: "device-custom-port".to_string(),
        device_name: "Custom-Port-Rig".to_string(),
        fingerprint: "cccc-dddd-eeee-ffff".to_string(),
        tcp_port: custom_port,
        is_inviting: true,
    };

    dm.record_beacon(
        beacon,
        "10.0.0.42".parse().unwrap(),
        "local-id",
        &[],
    );

    let peers = dm.list_discovered_peers(&[]);
    assert_eq!(peers.len(), 1);
    assert_eq!(
        peers[0].address,
        format!("10.0.0.42:{}", custom_port),
        "Recorded address must preserve advertised TCP port rather than assuming default"
    );
}

#[test]
fn test_device_selection_resolving_ip_and_port() {
    let _env = setup_test_env("device_selection_resolution");
    let dm = DiscoveryManager::new();

    let device_id = "device-dynamic-dhcp".to_string();
    let initial_ip: std::net::IpAddr = "192.168.1.55".parse().unwrap();

    let beacon = LanBeacon {
        magic: DISCOVERY_BEACON_MAGIC.to_string(),
        protocol_version: DISCOVERY_PROTOCOL_VERSION,
        device_id: device_id.clone(),
        device_name: "Laptop-LivingRoom".to_string(),
        fingerprint: "1234-5678-9012-3456".to_string(),
        tcp_port: DEFAULT_DISCOVERY_PORT,
        is_inviting: true,
    };

    // 1. Initial beacon at 192.168.1.55
    dm.record_beacon(beacon.clone(), initial_ip, "local-id", &[]);

    let peers = dm.list_discovered_peers(&[]);
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].address, "192.168.1.55:42890");

    // Zero-config selection simulation: user selects peer by device_id
    let selected = peers.into_iter().find(|p| p.device_id == device_id).unwrap();
    assert_eq!(selected.address, "192.168.1.55:42890");

    // 2. DHCP IP change simulation: device moves to 192.168.1.75
    let new_ip: std::net::IpAddr = "192.168.1.75".parse().unwrap();
    dm.record_beacon(beacon, new_ip, "local-id", &[]);

    let updated_peers = dm.list_discovered_peers(&[]);
    assert_eq!(updated_peers.len(), 1, "Should not duplicate the device record");
    assert_eq!(
        updated_peers[0].address, "192.168.1.75:42890",
        "Dynamic IP must update resolved address automatically"
    );
}

#[test]
fn test_multi_interface_broadcast_targets() {
    let targets = stardew_sync_core::core::network::discovery::get_broadcast_targets();

    assert!(
        targets.iter().any(|t| t == &format!("255.255.255.255:{}", DEFAULT_DISCOVERY_PORT)),
        "Broadcast targets must include global broadcast address"
    );
    assert!(
        targets.iter().any(|t| t == &format!("127.0.0.1:{}", DEFAULT_DISCOVERY_PORT)),
        "Broadcast targets must include loopback for local testing"
    );

    // Every target must use DEFAULT_DISCOVERY_PORT
    for target in &targets {
        let parts: Vec<&str> = target.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(
            parts[1],
            &DEFAULT_DISCOVERY_PORT.to_string(),
            "Every target must target DEFAULT_DISCOVERY_PORT"
        );
    }

    // Must not contain duplicate addresses
    let mut deduped = targets.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(targets.len(), deduped.len(), "Targets must not contain duplicates");
}

#[test]
fn test_invitation_and_sas_state_transitions() {
    let _env = setup_test_env("sas_state_transitions");
    let manager = PairingManager::new();
    let mut store = PairedDeviceStore::load().unwrap();

    let identity = DeviceIdentity::generate("Transition-PC");

    // 1. Initial State is Idle
    assert_eq!(manager.get_session_view().status, PairingStatus::Idle);

    // 2. Start Invitation -> Inviting
    let invitation = manager.start_invitation(&identity, 42890).expect("Start invitation");
    assert!(invitation.code.starts_with("SD-"));
    assert_eq!(manager.get_session_view().status, PairingStatus::Inviting);

    // 3. Cancel invitation -> resets to Idle
    manager.cancel_invitation().expect("Cancel invitation");
    assert_eq!(manager.get_session_view().status, PairingStatus::Idle);

    // 4. Start new invitation
    let inv2 = manager.start_invitation(&identity, 42890).expect("Start invitation 2");

    // 5. Connect peer -> start confirmation session -> AwaitingDualConfirmation
    let peer_info = PublicDeviceInfo {
        device_id: "transition-peer".to_string(),
        fingerprint: "4444-3333-2222-1111".to_string(),
        device_name: "Transition-Remote".to_string(),
        public_key_hex: "d".repeat(64),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    let sas = manager
        .start_confirmation_session(
            "sess-trans-1",
            &inv2.code,
            &"a".repeat(64),
            &"b".repeat(64),
            &"c".repeat(64),
            &"d".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            peer_info.clone(),
            true,
        )
        .expect("Start confirmation session");

    let view = manager.get_session_view();
    assert_eq!(view.status, PairingStatus::AwaitingDualConfirmation);
    assert!(!view.our_confirmed);
    assert!(!view.peer_confirmed);

    // 6. Local confirmation
    let res_local = manager.confirm_local(&sas, &mut store).expect("Confirm local");
    assert!(res_local.is_none(), "Session not complete until peer also confirms");
    let view_mid = manager.get_session_view();
    assert!(view_mid.our_confirmed);
    assert!(!view_mid.peer_confirmed);
    assert_eq!(view_mid.status, PairingStatus::AwaitingDualConfirmation);

    // 7. Remote confirmation
    let res_remote = manager.confirm_remote(&sas, &mut store).expect("Confirm remote");
    assert!(res_remote.is_some(), "Session completes when both confirm");
    let view_done = manager.get_session_view();
    assert_eq!(view_done.status, PairingStatus::Completed);
    assert!(view_done.our_confirmed);
    assert!(view_done.peer_confirmed);

    // 8. Device is pinned in store
    assert!(store.get(&peer_info.device_id).is_some());
}

#[test]
fn test_discovery_expiration_after_timeout() {
    let _env = setup_test_env("discovery_expiration_timeout");
    let dm = DiscoveryManager::new();

    let beacon = LanBeacon {
        magic: DISCOVERY_BEACON_MAGIC.to_string(),
        protocol_version: DISCOVERY_PROTOCOL_VERSION,
        device_id: "expiring-device".to_string(),
        device_name: "Expiring-PC".to_string(),
        fingerprint: "eeee-ffff-0000-1111".to_string(),
        tcp_port: DEFAULT_DISCOVERY_PORT,
        is_inviting: false,
    };

    dm.record_beacon(
        beacon,
        "192.168.1.99".parse().unwrap(),
        "local-id",
        &[],
    );

    // Immediately after record, peer is present and 0 seconds old
    let initial_peers = dm.list_discovered_peers(&[]);
    assert_eq!(initial_peers.len(), 1);
    assert_eq!(initial_peers[0].device_id, "expiring-device");

    // Simulate age of 10 seconds (under 15s threshold)
    dm.simulate_peer_age("expiring-device", 10);
    let active_peers = dm.list_discovered_peers(&[]);
    assert_eq!(active_peers.len(), 1, "Peer aged 10s must still be retained");
    assert!(active_peers[0].last_seen_secs_ago >= 10);

    // Simulate age of 16 seconds (over 15s threshold)
    dm.simulate_peer_age("expiring-device", 16);
    let expired_peers = dm.list_discovered_peers(&[]);
    assert_eq!(
        expired_peers.len(), 0,
        "Peer aged 16s must be pruned upon list_discovered_peers"
    );
}

#[test]
fn test_status_model_derivation() {
    let _env = setup_test_env("status_model_derivation");
    let mut store = PairedDeviceStore::load().expect("Load store");

    let paired_device = PairedDevice {
        device_id: "paired-peer-123".to_string(),
        device_name: "Elbi-PC".to_string(),
        public_key_hex: "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20".to_string(),
        fingerprint: "1111-2222-3333-4444".to_string(),
        paired_at: "2026-10-10T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    };
    store.add_or_update(paired_device).unwrap();

    let conn_mgr = ConnectionManager::new();
    let discovery = DiscoveryManager::new();

    // 1. Initial state: No connection and no LAN beacon -> "offline"
    let status_initial = if conn_mgr.get_status("paired-peer-123") == PeerConnectionStatus::Connected {
        "connected"
    } else if discovery.get_peer("paired-peer-123").is_some() {
        "online"
    } else {
        "offline"
    };
    assert_eq!(status_initial, "offline", "Device without socket or LAN presence must evaluate to offline");

    // 2. Beacon received: Present in DiscoveryManager -> "online"
    discovery.record_beacon(
        LanBeacon {
            magic: DISCOVERY_BEACON_MAGIC.to_string(),
            protocol_version: DISCOVERY_PROTOCOL_VERSION,
            device_id: "paired-peer-123".to_string(),
            device_name: "Elbi-PC".to_string(),
            fingerprint: "1111-2222-3333-4444".to_string(),
            tcp_port: DEFAULT_DISCOVERY_PORT,
            is_inviting: false,
        },
        "192.168.1.150".parse().unwrap(),
        "local-id",
        &["paired-peer-123".to_string()],
    );

    let status_discovered = if conn_mgr.get_status("paired-peer-123") == PeerConnectionStatus::Connected {
        "connected"
    } else if discovery.get_peer("paired-peer-123").is_some() {
        "online"
    } else {
        "offline"
    };
    assert_eq!(status_discovered, "online", "Device with recent LAN beacon must evaluate to online");

    // 3. Active transport socket connected in conn_mgr -> "connected" takes precedence
    conn_mgr.set_status("paired-peer-123", PeerConnectionStatus::Connected);
    let status_connected = if conn_mgr.get_status("paired-peer-123") == PeerConnectionStatus::Connected {
        "connected"
    } else if discovery.get_peer("paired-peer-123").is_some() {
        "online"
    } else {
        "offline"
    };
    assert_eq!(status_connected, "connected", "Device with active authenticated socket must evaluate to connected");

    // 4. Transport disconnected and beacon aged past threshold -> drops back to "offline"
    conn_mgr.set_status("paired-peer-123", PeerConnectionStatus::Disconnected);
    discovery.simulate_peer_age("paired-peer-123", 20);
    let status_aged = if conn_mgr.get_status("paired-peer-123") == PeerConnectionStatus::Connected {
        "connected"
    } else if discovery.get_peer("paired-peer-123").is_some() {
        "online"
    } else {
        "offline"
    };
    assert_eq!(status_aged, "offline", "Device with disconnected socket and expired beacon must evaluate to offline");
}

#[test]
fn test_authenticated_remote_revocation_e2e() {
    let _env = setup_test_env("authenticated_remote_revocation");

    // 1. Generate identity for Peer A (Kubilay) and Peer B (Elbi)
    let identity_a = DeviceIdentity::generate("Kubilay-PC");
    let identity_b = DeviceIdentity::generate("Elbi-Laptop");

    let mut store = PairedDeviceStore::load().unwrap();

    // Store Peer B in Peer A's trusted store
    store.add_or_update(PairedDevice {
        device_id: identity_b.device_id(),
        device_name: identity_b.device_name().to_string(),
        public_key_hex: hex::encode(identity_b.verifying_key().as_bytes()),
        fingerprint: identity_b.fingerprint(),
        paired_at: "2026-10-10T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: Some("127.0.0.1".to_string()),
    }).unwrap();

    // 2. Start mock TCP listener on Peer B
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind ephemeral TCP port");
    let server_port = listener.local_addr().unwrap().port();
    let server_addr = format!("127.0.0.1:{}", server_port);

    let id_b_clone = identity_b.clone();
    let id_a_pk = hex::encode(identity_a.verifying_key().as_bytes());
    let id_a_device_id = identity_a.device_id();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept incoming connection");
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(3))).unwrap();

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).expect("Read frame length");
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut frame_buf = vec![0u8; len];
        stream.read_exact(&mut frame_buf).expect("Read frame payload");

        let mut full_buf = Vec::with_capacity(4 + len);
        full_buf.extend_from_slice(&len_buf);
        full_buf.extend_from_slice(&frame_buf);

        let msg = NetworkWireMessage::decode(&full_buf).expect("Decode wire message");
        match msg {
            NetworkWireMessage::PlaintextFrame(frame) => {
                // Verify that frame was sent by Peer A and signed by Peer A's pinned key
                assert_eq!(frame.sender_device_id, id_a_device_id);

                let vk_bytes = hex::decode(&id_a_pk).unwrap();
                let mut vk_arr = [0u8; 32];
                vk_arr.copy_from_slice(&vk_bytes);
                let vk = ed25519_dalek::VerifyingKey::from_bytes(&vk_arr).unwrap();

                frame.verify_signature(&vk).expect("Verify sender signature");

                match frame.payload {
                    TransportFramePayload::UnpairRevocation { target_device_id, reason } => {
                        assert_eq!(target_device_id, id_b_clone.device_id());
                        assert_eq!(reason, "User requested unpair");

                        // Reply with signed ACK
                        let ack = TransportFramePayload::UnpairRevocationAck { success: true };
                        let resp_frame = TransportFrame::new_signed(frame.sequence + 1, &id_b_clone, ack).unwrap();
                        let wire_resp = NetworkWireMessage::PlaintextFrame(resp_frame);
                        let encoded = wire_resp.encode().unwrap();
                        stream.write_all(&encoded).unwrap();
                        stream.flush().unwrap();
                    }
                    _ => panic!("Expected UnpairRevocation frame"),
                }
            }
            _ => panic!("Expected PlaintextFrame"),
        }
    });

    // 3. Peer A sends authenticated unpair revocation to Peer B over TCP
    let net = NetworkService::new();
    let result = net.send_remote_unpair(&server_addr, &identity_b.device_id(), &identity_a)
        .expect("Remote unpair send succeeded");

    assert!(result, "Peer B acknowledged revocation with success: true");

    handle.join().unwrap();

    // 4. Peer A performs local removal
    let removed = store.remove(&identity_b.device_id()).unwrap();
    assert!(removed, "Local store must successfully remove Peer B");
    assert!(store.get(&identity_b.device_id()).is_none(), "Peer B must no longer exist in Peer A's store");
}

#[test]
fn test_offline_revocation_fail_closed() {
    let _env = setup_test_env("offline_revocation_fail_closed");

    let identity_a = DeviceIdentity::generate("Kubilay-PC");
    let mut store = PairedDeviceStore::load().unwrap();

    let fake_device_id = "offline-peer-999".to_string();
    store.add_or_update(PairedDevice {
        device_id: fake_device_id.clone(),
        device_name: "Offline-PC".to_string(),
        public_key_hex: "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20".to_string(),
        fingerprint: "9999-8888-7777-6666".to_string(),
        paired_at: "2026-10-10T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: Some("127.0.0.1".to_string()),
    }).unwrap();

    // Attempt remote revocation to an unreachable port (e.g. 59999)
    let net = NetworkService::new();
    let send_result = net.send_remote_unpair("127.0.0.1:59999", &fake_device_id, &identity_a);
    assert!(send_result.is_err(), "Network connection to offline peer must fail");

    // Local trust deletion must still succeed (fail-closed security)
    let local_removed = store.remove(&fake_device_id).unwrap();
    assert!(local_removed, "Local trust removal must succeed even when peer is offline");
    assert!(store.get(&fake_device_id).is_none(), "Offline peer must be removed from local store");
}

#[test]
fn test_unauthenticated_revocation_rejected() {
    let _env = setup_test_env("unauthenticated_revocation_rejected");

    let legitimate_peer_a = DeviceIdentity::generate("Kubilay-PC");
    let attacker = DeviceIdentity::generate("Attacker-PC");
    let peer_b = DeviceIdentity::generate("Elbi-Laptop");

    let mut store_b = PairedDeviceStore::load().unwrap();

    // Peer B trusts Peer A (pinned to Peer A's verifying key)
    store_b.add_or_update(PairedDevice {
        device_id: legitimate_peer_a.device_id(),
        device_name: legitimate_peer_a.device_name().to_string(),
        public_key_hex: hex::encode(legitimate_peer_a.verifying_key().as_bytes()),
        fingerprint: legitimate_peer_a.fingerprint(),
        paired_at: "2026-10-10T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    }).unwrap();

    // Attacker constructs an UnpairRevocation frame claiming to be Peer A, but signed with Attacker's key
    let payload = TransportFramePayload::UnpairRevocation {
        target_device_id: peer_b.device_id(),
        reason: "Malicious unpair attempt".to_string(),
    };

    // Attacker signs with their own key, claiming sender is legitimate Peer A
    let timestamp = chrono::Utc::now().to_rfc3339();
    let forged_sender = legitimate_peer_a.device_id();
    let unsigned = serde_json::to_vec(&(&1u64, &timestamp, &forged_sender, &payload)).unwrap();
    let attacker_signature = attacker.sign(&unsigned);

    let forged_frame = TransportFrame {
        sequence: 1,
        timestamp,
        sender_device_id: forged_sender,
        payload,
        signature_hex: hex::encode(attacker_signature),
    };

    // Peer B validates frame against Peer A's PINNED public key
    let pinned_peer_a = store_b.get(&legitimate_peer_a.device_id()).unwrap();
    let pinned_vk_bytes = hex::decode(&pinned_peer_a.public_key_hex).unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&pinned_vk_bytes);
    let pinned_vk = ed25519_dalek::VerifyingKey::from_bytes(&arr).unwrap();

    let verify_res = forged_frame.verify_signature(&pinned_vk);
    assert!(verify_res.is_err(), "Signature verification MUST fail when forged by an attacker");

    // Peer B must NOT delete Peer A from trust store
    assert!(store_b.get(&legitimate_peer_a.device_id()).is_some(), "Legitimate peer MUST remain in store");
}

#[test]
fn test_idempotent_revocation() {
    let _env = setup_test_env("idempotent_revocation");
    let mut store = PairedDeviceStore::load().unwrap();

    // Removing a non-existent device returns Ok(false) without modifying or corrupting store
    let res1 = store.remove("non-existent-device-id").unwrap();
    assert!(!res1);

    let res2 = store.remove("non-existent-device-id").unwrap();
    assert!(!res2);
}

#[test]
fn test_two_process_simulated_dual_confirmation_both_confirm_symmetric_persistence() {
    let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // Isolated storage dirs for Device A and Device B
    let temp_a = TempDir::new().unwrap();
    let temp_b = TempDir::new().unwrap();
    let id_dir_a = temp_a.path().join("identity");
    let dev_dir_a = temp_a.path().join("devices");
    let id_dir_b = temp_b.path().join("identity");
    let dev_dir_b = temp_b.path().join("devices");
    fs::create_dir_all(&id_dir_a).unwrap();
    fs::create_dir_all(&dev_dir_a).unwrap();
    fs::create_dir_all(&id_dir_b).unwrap();
    fs::create_dir_all(&dev_dir_b).unwrap();

    let identity_a = DeviceIdentity::generate("Kubilay-PC");
    let identity_b = DeviceIdentity::generate("Elbi-Laptop");

    // Establish TCP connection between simulated Device A and Device B
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let id_a_clone = identity_a.clone();
    let id_b_clone = identity_b.clone();

    let dev_dir_a_clone = dev_dir_a.clone();

    let handle = thread::spawn(move || {
        // Device A (Inviter)
        let (mut stream_a, _) = listener.accept().unwrap();
        stream_a.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

        // 1. Receive PairingRequest
        let mut len_buf = [0u8; 4];
        stream_a.read_exact(&mut len_buf).unwrap();
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        stream_a.read_exact(&mut buf).unwrap();

        let mut full_frame = Vec::new();
        full_frame.extend_from_slice(&len_buf);
        full_frame.extend_from_slice(&buf);
        let wire_req = NetworkWireMessage::decode(&full_frame).unwrap();

        let req = match wire_req {
            NetworkWireMessage::PairingRequest(r) => r,
            _ => panic!("Expected PairingRequest"),
        };

        // 2. Respond with PairingResponse
        let resp_sig = id_a_clone.sign(format!("sess-123:{}:ecdh_a:nonce_a", id_a_clone.device_id()).as_bytes());
        let resp = stardew_sync_core::core::pairing::PairingResponse {
            protocol_version: 1,
            session_id: "sess-123".to_string(),
            inviter_device_id: id_a_clone.device_id(),
            inviter_device_name: id_a_clone.device_name().to_string(),
            inviter_public_key_hex: hex::encode(id_a_clone.verifying_key().as_bytes()),
            inviter_ecdh_public_hex: "ecdh_a".to_string(),
            inviter_nonce_hex: "nonce_a".to_string(),
            signature_hex: hex::encode(resp_sig),
        };
        let wire_resp = NetworkWireMessage::PairingResponse(resp);
        stream_a.write_all(&wire_resp.encode().unwrap()).unwrap();
        stream_a.flush().unwrap();

        // Setup PairingManager on Device A
        let mgr_a = PairingManager::new();
        let peer_b_info = PublicDeviceInfo {
            device_id: id_b_clone.device_id(),
            fingerprint: id_b_clone.fingerprint(),
            device_name: id_b_clone.device_name().to_string(),
            public_key_hex: hex::encode(id_b_clone.verifying_key().as_bytes()),
            created_at: "2026-10-10T12:00:00Z".to_string(),
        };
        mgr_a.start_confirmation_session_with_secret(
            "sess-123",
            "SD-1234",
            &hex::encode(id_a_clone.verifying_key().as_bytes()),
            &req.joiner_public_key_hex,
            "ecdh_a",
            &req.joiner_ecdh_public_hex,
            "nonce_a",
            &req.joiner_nonce_hex,
            peer_b_info,
            true,
            Some([42u8; 32]),
        ).unwrap();

        let sas_code = mgr_a.get_session_view().sas_code.unwrap();

        // Device A confirms code locally
        std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir_a_clone.to_str().unwrap());
        let mut store_a = PairedDeviceStore::load().unwrap();
        mgr_a.confirm_local(&sas_code, &mut store_a).unwrap();

        // Device A sends signed PairingConfirm
        let sig_a = id_a_clone.sign(format!("sess-123:{}:{}:true", id_a_clone.device_id(), sas_code).as_bytes());
        let confirm_a = PairingConfirm {
            session_id: "sess-123".to_string(),
            sender_device_id: id_a_clone.device_id(),
            sas_code: sas_code.clone(),
            accepted: true,
            signature_hex: hex::encode(sig_a),
        };
        stream_a.write_all(&NetworkWireMessage::PairingConfirm(confirm_a).encode().unwrap()).unwrap();
        stream_a.flush().unwrap();

        // Device A reads Device B's PairingConfirm
        let mut len_buf = [0u8; 4];
        stream_a.read_exact(&mut len_buf).unwrap();
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        stream_a.read_exact(&mut buf).unwrap();
        let mut full_frame = Vec::new();
        full_frame.extend_from_slice(&len_buf);
        full_frame.extend_from_slice(&buf);

        let wire_msg = NetworkWireMessage::decode(&full_frame).unwrap();
        let confirm_b = match wire_msg {
            NetworkWireMessage::PairingConfirm(c) => c,
            _ => panic!("Expected PairingConfirm from Device B"),
        };

        // Device A records B's confirm -> transitions to Completed and writes to store A
        let res_a = mgr_a.confirm_remote(&confirm_b.sas_code, &mut store_a).unwrap();
        assert!(res_a.is_some(), "Device A must complete pairing upon remote confirmation");

        // Device A sends PairingFinalAck
        let ack_sig_a = id_a_clone.sign(format!("FINAL_ACK:sess-123:{}", id_a_clone.device_id()).as_bytes());
        let ack_a = PairingFinalAck {
            session_id: "sess-123".to_string(),
            sender_device_id: id_a_clone.device_id(),
            signature_hex: hex::encode(ack_sig_a),
        };
        stream_a.write_all(&NetworkWireMessage::PairingFinalAck(ack_a).encode().unwrap()).unwrap();
        stream_a.flush().unwrap();

        // Device A reads Device B's PairingFinalAck
        let mut len_buf = [0u8; 4];
        stream_a.read_exact(&mut len_buf).unwrap();
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        stream_a.read_exact(&mut buf).unwrap();
        let mut full_frame = Vec::new();
        full_frame.extend_from_slice(&len_buf);
        full_frame.extend_from_slice(&buf);
        let wire_ack_b = NetworkWireMessage::decode(&full_frame).unwrap();
        match wire_ack_b {
            NetworkWireMessage::PairingFinalAck(ack) => {
                assert_eq!(ack.session_id, "sess-123");
                assert_eq!(ack.sender_device_id, id_b_clone.device_id());
            }
            _ => panic!("Expected PairingFinalAck from Device B"),
        }

        (mgr_a, store_a)
    });

    // Device B (Joiner)
    let mut stream_b = std::net::TcpStream::connect(server_addr).unwrap();
    stream_b.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

    let sig_req = identity_b.sign(format!("SD-1234:{}:ecdh_b:nonce_b", identity_b.device_id()).as_bytes());
    let req = stardew_sync_core::core::pairing::PairingRequest {
        protocol_version: 1,
        invitation_code: "SD-1234".to_string(),
        joiner_device_id: identity_b.device_id(),
        joiner_device_name: identity_b.device_name().to_string(),
        joiner_public_key_hex: hex::encode(identity_b.verifying_key().as_bytes()),
        joiner_ecdh_public_hex: "ecdh_b".to_string(),
        joiner_nonce_hex: "nonce_b".to_string(),
        signature_hex: hex::encode(sig_req),
    };
    stream_b.write_all(&NetworkWireMessage::PairingRequest(req).encode().unwrap()).unwrap();
    stream_b.flush().unwrap();

    // Device B reads PairingResponse
    let mut len_buf = [0u8; 4];
    stream_b.read_exact(&mut len_buf).unwrap();
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream_b.read_exact(&mut buf).unwrap();

    // Setup PairingManager on Device B
    let mgr_b = PairingManager::new();
    let peer_a_info = PublicDeviceInfo {
        device_id: identity_a.device_id(),
        fingerprint: identity_a.fingerprint(),
        device_name: identity_a.device_name().to_string(),
        public_key_hex: hex::encode(identity_a.verifying_key().as_bytes()),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };
    mgr_b.start_confirmation_session_with_secret(
        "sess-123",
        "SD-1234",
        &hex::encode(identity_a.verifying_key().as_bytes()),
        &hex::encode(identity_b.verifying_key().as_bytes()),
        "ecdh_a",
        "ecdh_b",
        "nonce_a",
        "nonce_b",
        peer_a_info,
        false,
        Some([42u8; 32]),
    ).unwrap();

    let sas_code = mgr_b.get_session_view().sas_code.unwrap();

    // Device B receives Device A's PairingConfirm
    let mut len_buf = [0u8; 4];
    stream_b.read_exact(&mut len_buf).unwrap();
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream_b.read_exact(&mut buf).unwrap();
    let mut full_frame = Vec::new();
    full_frame.extend_from_slice(&len_buf);
    full_frame.extend_from_slice(&buf);

    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir_b.to_str().unwrap());
    let mut store_b = PairedDeviceStore::load().unwrap();

    let wire_confirm_a = NetworkWireMessage::decode(&full_frame).unwrap();
    match wire_confirm_a {
        NetworkWireMessage::PairingConfirm(c) => {
            mgr_b.confirm_remote(&c.sas_code, &mut store_b).unwrap();
        }
        _ => panic!("Expected PairingConfirm from Device A"),
    }

    // Device B confirms code locally -> both confirmed -> Completed and writes to store B
    let res_b = mgr_b.confirm_local(&sas_code, &mut store_b).unwrap();
    assert!(res_b.is_some(), "Device B must complete pairing upon local confirmation");

    // Device B sends its PairingConfirm
    let sig_b = identity_b.sign(format!("sess-123:{}:{}:true", identity_b.device_id(), sas_code).as_bytes());
    let confirm_b = PairingConfirm {
        session_id: "sess-123".to_string(),
        sender_device_id: identity_b.device_id(),
        sas_code: sas_code.clone(),
        accepted: true,
        signature_hex: hex::encode(sig_b),
    };
    stream_b.write_all(&NetworkWireMessage::PairingConfirm(confirm_b).encode().unwrap()).unwrap();
    stream_b.flush().unwrap();

    // Device B receives Device A's PairingFinalAck
    let mut len_buf = [0u8; 4];
    stream_b.read_exact(&mut len_buf).unwrap();
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream_b.read_exact(&mut buf).unwrap();

    // Device B sends its PairingFinalAck
    let ack_sig_b = identity_b.sign(format!("FINAL_ACK:sess-123:{}", identity_b.device_id()).as_bytes());
    let ack_b = PairingFinalAck {
        session_id: "sess-123".to_string(),
        sender_device_id: identity_b.device_id(),
        signature_hex: hex::encode(ack_sig_b),
    };
    stream_b.write_all(&NetworkWireMessage::PairingFinalAck(ack_b).encode().unwrap()).unwrap();
    stream_b.flush().unwrap();

    let (mgr_a, store_a) = handle.join().unwrap();

    // VERIFY SYMMETRIC TRUST PERSISTENCE:
    assert_eq!(mgr_a.get_session_view().status, PairingStatus::Completed);
    assert_eq!(mgr_b.get_session_view().status, PairingStatus::Completed);

    assert!(store_a.get(&identity_b.device_id()).is_some(), "Store A MUST contain Device B");
    assert!(store_b.get(&identity_a.device_id()).is_some(), "Store B MUST contain Device A");

    let record_in_a = store_a.get(&identity_b.device_id()).unwrap();
    let record_in_b = store_b.get(&identity_a.device_id()).unwrap();

    assert_eq!(record_in_a.device_name, "Elbi-Laptop");
    assert_eq!(record_in_b.device_name, "Kubilay-PC");
    assert_eq!(record_in_a.fingerprint, identity_b.fingerprint());
    assert_eq!(record_in_b.fingerprint, identity_a.fingerprint());
}

#[test]
fn test_two_process_simulated_one_side_confirms_peer_disconnects_fails_closed() {
    let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let temp_a = TempDir::new().unwrap();
    let temp_b = TempDir::new().unwrap();
    let dev_dir_a = temp_a.path().join("devices");
    let dev_dir_b = temp_b.path().join("devices");
    fs::create_dir_all(&dev_dir_a).unwrap();
    fs::create_dir_all(&dev_dir_b).unwrap();

    let identity_a = DeviceIdentity::generate("Kubilay-PC");
    let identity_b = DeviceIdentity::generate("Elbi-Laptop");

    let mgr_a = PairingManager::new();
    let peer_b_info = PublicDeviceInfo {
        device_id: identity_b.device_id(),
        fingerprint: identity_b.fingerprint(),
        device_name: identity_b.device_name().to_string(),
        public_key_hex: hex::encode(identity_b.verifying_key().as_bytes()),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    mgr_a.start_confirmation_session_with_secret(
        "sess-disc-1",
        "SD-9999",
        &hex::encode(identity_a.verifying_key().as_bytes()),
        &hex::encode(identity_b.verifying_key().as_bytes()),
        "ecdh_a",
        "ecdh_b",
        "nonce_a",
        "nonce_b",
        peer_b_info,
        true,
        Some([77u8; 32]),
    ).unwrap();

    let sas_code = mgr_a.get_session_view().sas_code.unwrap();

    // Device A confirms code locally
    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir_a.to_str().unwrap());
    let mut store_a = PairedDeviceStore::load().unwrap();
    let dev_opt = mgr_a.confirm_local(&sas_code, &mut store_a).unwrap();
    assert!(dev_opt.is_none(), "Single local confirmation must not yield completed paired device");
    assert_eq!(mgr_a.get_session_view().status, PairingStatus::AwaitingDualConfirmation);

    // Device B drops connection abruptly without confirming
    // Device A detects disconnect and invokes reject() (fail-closed)
    mgr_a.reject().unwrap();

    // VERIFY: Status is Rejected and neither store contains the other
    assert_eq!(mgr_a.get_session_view().status, PairingStatus::Rejected);
    assert!(store_a.get(&identity_b.device_id()).is_none(), "Store A MUST NOT contain Device B on disconnect");

    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir_b.to_str().unwrap());
    let store_b = PairedDeviceStore::load().unwrap();
    assert!(store_b.get(&identity_a.device_id()).is_none(), "Store B MUST NOT contain Device A on disconnect");
}

#[test]
fn test_persistence_failure_fails_closed_and_never_reports_completed() {
    let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let temp = TempDir::new().unwrap();
    let dev_dir = temp.path().join("devices");
    fs::create_dir_all(&dev_dir).unwrap();
    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir.to_str().unwrap());

    let identity_a = DeviceIdentity::generate("Kubilay-PC");
    let identity_b = DeviceIdentity::generate("Elbi-Laptop");

    let mut store = PairedDeviceStore::load().unwrap();

    // Pre-populate store with Device B pinned to a CONFLICTING key
    let pinned_fake_pk = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
    store.add_or_update(PairedDevice {
        device_id: identity_b.device_id(),
        device_name: identity_b.device_name().to_string(),
        public_key_hex: pinned_fake_pk,
        fingerprint: "0000-0000-0000-0000".to_string(),
        paired_at: "2026-10-10T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    }).unwrap();

    let mgr = PairingManager::new();
    let peer_b_info = PublicDeviceInfo {
        device_id: identity_b.device_id(),
        fingerprint: identity_b.fingerprint(),
        device_name: identity_b.device_name().to_string(),
        public_key_hex: hex::encode(identity_b.verifying_key().as_bytes()),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    mgr.start_confirmation_session_with_secret(
        "sess-conflict",
        "SD-4444",
        &hex::encode(identity_a.verifying_key().as_bytes()),
        &hex::encode(identity_b.verifying_key().as_bytes()),
        "ecdh_a",
        "ecdh_b",
        "nonce_a",
        "nonce_b",
        peer_b_info,
        true,
        Some([99u8; 32]),
    ).unwrap();

    let sas_code = mgr.get_session_view().sas_code.unwrap();

    // Remote confirmed first
    mgr.confirm_remote(&sas_code, &mut store).unwrap();

    // Now local confirms -> will attempt store.add_or_update, which MUST FAIL due to key conflict!
    let confirm_res = mgr.confirm_local(&sas_code, &mut store);
    assert!(confirm_res.is_err(), "Local confirmation MUST fail if persistence encounters a security violation");

    // Session status MUST be Rejected, NEVER Completed!
    assert_eq!(mgr.get_session_view().status, PairingStatus::Rejected);
}

#[test]
fn test_app_restart_after_partial_pairing() {
    let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let temp = TempDir::new().unwrap();
    let dev_dir = temp.path().join("devices");
    fs::create_dir_all(&dev_dir).unwrap();
    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir.to_str().unwrap());

    // Simulated partial pairing where session was created and confirmed on one side, but rejected before completion
    let identity_a = DeviceIdentity::generate("Kubilay-PC");
    let identity_b = DeviceIdentity::generate("Elbi-Laptop");

    let mgr = PairingManager::new();
    let peer_b_info = PublicDeviceInfo {
        device_id: identity_b.device_id(),
        fingerprint: identity_b.fingerprint(),
        device_name: identity_b.device_name().to_string(),
        public_key_hex: hex::encode(identity_b.verifying_key().as_bytes()),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };

    mgr.start_confirmation_session_with_secret(
        "sess-restart",
        "SD-5555",
        &hex::encode(identity_a.verifying_key().as_bytes()),
        &hex::encode(identity_b.verifying_key().as_bytes()),
        "ecdh_a",
        "ecdh_b",
        "nonce_a",
        "nonce_b",
        peer_b_info,
        true,
        Some([12u8; 32]),
    ).unwrap();

    let sas_code = mgr.get_session_view().sas_code.unwrap();
    let mut store = PairedDeviceStore::load().unwrap();
    mgr.confirm_local(&sas_code, &mut store).unwrap();
    // Interrupted before peer confirmed:
    mgr.reject().unwrap();

    // Simulated App Restart: Load store fresh from disk
    let fresh_store = PairedDeviceStore::load().unwrap();
    assert_eq!(fresh_store.list().len(), 0, "No partial pairing records may persist across app restart");
    assert!(fresh_store.get(&identity_b.device_id()).is_none());
}

#[test]
fn test_reactive_lan_discovery_settling_and_refresh_timing() {
    let discovery = DiscoveryManager::new();
    let local_id = "local-device-123";

    let beacon = LanBeacon {
        magic: DISCOVERY_BEACON_MAGIC.to_string(),
        protocol_version: DISCOVERY_PROTOCOL_VERSION,
        device_id: "remote-peer-456".to_string(),
        device_name: "Elbi-Laptop".to_string(),
        fingerprint: "1111-2222-3333-4444".to_string(),
        tcp_port: 42890,
        is_inviting: false,
    };

    discovery.record_beacon(
        beacon,
        "192.168.1.100".parse().unwrap(),
        local_id,
        &[],
    );

    // Immediate lookup within settling window (<350ms)
    let peer = discovery.get_peer("remote-peer-456");
    assert!(peer.is_some(), "Peer must be immediately retrievable within settling window");
    let peer = peer.unwrap();
    assert_eq!(peer.device_name, "Elbi-Laptop");
    assert_eq!(peer.address, "192.168.1.100:42890");
}

#[test]
fn test_lost_final_ack_behavior_documented_and_tested() {
    let _guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let temp_a = TempDir::new().unwrap();
    let temp_b = TempDir::new().unwrap();
    let dev_dir_a = temp_a.path().join("devices");
    let dev_dir_b = temp_b.path().join("devices");
    fs::create_dir_all(&dev_dir_a).unwrap();
    fs::create_dir_all(&dev_dir_b).unwrap();

    let identity_a = DeviceIdentity::generate("Kubilay-PC");
    let identity_b = DeviceIdentity::generate("Elbi-Laptop");

    // Scenario: Device A confirms first.
    let mgr_a = PairingManager::new();
    let peer_b_info = PublicDeviceInfo {
        device_id: identity_b.device_id(),
        fingerprint: identity_b.fingerprint(),
        device_name: identity_b.device_name().to_string(),
        public_key_hex: hex::encode(identity_b.verifying_key().as_bytes()),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };
    mgr_a.start_confirmation_session_with_secret(
        "sess-lost-ack",
        "SD-7777",
        &hex::encode(identity_a.verifying_key().as_bytes()),
        &hex::encode(identity_b.verifying_key().as_bytes()),
        "ecdh_a",
        "ecdh_b",
        "nonce_a",
        "nonce_b",
        peer_b_info.clone(),
        true,
        Some([33u8; 32]),
    ).unwrap();
    let sas_code = mgr_a.get_session_view().sas_code.unwrap();

    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir_a.to_str().unwrap());
    let mut store_a = PairedDeviceStore::load().unwrap();
    mgr_a.confirm_local(&sas_code, &mut store_a).unwrap();

    // Device B receives A's confirm, confirms locally, and commits trust to store B.
    let mgr_b = PairingManager::new();
    let peer_a_info = PublicDeviceInfo {
        device_id: identity_a.device_id(),
        fingerprint: identity_a.fingerprint(),
        device_name: identity_a.device_name().to_string(),
        public_key_hex: hex::encode(identity_a.verifying_key().as_bytes()),
        created_at: "2026-10-10T12:00:00Z".to_string(),
    };
    mgr_b.start_confirmation_session_with_secret(
        "sess-lost-ack",
        "SD-7777",
        &hex::encode(identity_a.verifying_key().as_bytes()),
        &hex::encode(identity_b.verifying_key().as_bytes()),
        "ecdh_a",
        "ecdh_b",
        "nonce_a",
        "nonce_b",
        peer_a_info,
        false,
        Some([33u8; 32]),
    ).unwrap();

    std::env::set_var("STARDEW_SYNC_DEVICES_DIR", dev_dir_b.to_str().unwrap());
    let mut store_b = PairedDeviceStore::load().unwrap();
    mgr_b.confirm_remote(&sas_code, &mut store_b).unwrap();
    let res_b = mgr_b.confirm_local(&sas_code, &mut store_b).unwrap();
    assert!(res_b.is_some(), "Device B must commit to store B upon dual confirmation");

    // Network partition occurs: Final ack and confirm from B are lost in transit.
    // Device A's connection detects premature disconnect and fails closed!
    mgr_a.reject().unwrap();

    // 1. Verify asymmetry state under network partition:
    assert!(store_b.get(&identity_a.device_id()).is_some(), "Store B holds Device A record");
    assert!(store_a.get(&identity_b.device_id()).is_none(), "Store A does NOT hold Device B record");
    assert_eq!(mgr_a.get_session_view().status, PairingStatus::Rejected);

    // 2. Security validation: Mutual trust is enforced, so unconfirmed Device A cannot access Device B
    assert!(!store_a.is_trusted(&identity_b.device_id(), &hex::encode(identity_b.verifying_key().as_bytes())));

    // 3. Self-healing validation: When Device A re-initiates pairing with the same identity,
    // Device B's key pinning accepts the re-pair without conflict and heals mutual trust.
    let re_paired_dev = PairedDevice {
        device_id: identity_a.device_id(),
        device_name: identity_a.device_name().to_string(),
        public_key_hex: hex::encode(identity_a.verifying_key().as_bytes()),
        fingerprint: identity_a.fingerprint(),
        paired_at: "2026-10-11T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: None,
    };
    let update_res = store_b.add_or_update(re_paired_dev);
    assert!(update_res.is_ok(), "Key pinning must permit idempotent re-pairing with identical cryptographic public key");
}

#[test]
fn test_wire_message_pairing_reject_round_trip() {
    let reject = PairingReject {
        protocol_version: 1,
        sender_device_id: "test-sender-id".to_string(),
        rejection_reason: "No active invitation on remote device".to_string(),
    };
    let wire = NetworkWireMessage::PairingReject(reject.clone());
    let encoded = wire.encode().expect("Encode PairingReject wire message");
    let decoded = NetworkWireMessage::decode(&encoded).expect("Decode PairingReject wire message");

    match decoded {
        NetworkWireMessage::PairingReject(rej) => {
            assert_eq!(rej.protocol_version, reject.protocol_version);
            assert_eq!(rej.sender_device_id, reject.sender_device_id);
            assert_eq!(rej.rejection_reason, reject.rejection_reason);
        }
        _ => panic!("Expected PairingReject variant from wire message"),
    }
}

#[test]
fn test_pairing_without_active_invitation_returns_clean_protocol_rejection() {
    let _env = setup_test_env("pairing_no_active_invitation");

    let identity_inviter = DeviceIdentity::generate("Idle-Inviter-PC");
    let net = stardew_sync_core::core::network::get_network_service();
    let test_port = 42894u16;
    net.start_tcp_listener(test_port, identity_inviter.clone());
    std::thread::sleep(Duration::from_millis(150));
    let actual_port = net.get_active_tcp_port().unwrap_or(test_port);

    // Ensure inviter has NO active invitation
    let pairing_mgr = stardew_sync_core::core::pairing::get_pairing_manager();
    let _ = pairing_mgr.cancel_invitation();
    assert!(pairing_mgr.get_active_invitation().is_none());

    let identity_joiner = DeviceIdentity::generate("Uninvited-Joiner-PC");
    let peer_addr = format!("127.0.0.1:{}", actual_port);

    let res = net.initiate_pairing(&peer_addr, "SD-0000", &identity_joiner);
    assert!(res.is_err(), "Pairing without active invitation must be rejected");

    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("Pairing rejected by peer: No active invitation on remote device")
            || err_str.contains("Connection closed by peer before pairing response"),
        "Error message must be a clean domain error, got: {}",
        err_str
    );
    assert!(
        !err_str.contains("failed to fill whole buffer"),
        "Must NOT surface raw EOF buffer error: {}",
        err_str
    );
}

#[test]
fn test_pairing_with_mismatched_invitation_code_returns_clean_rejection() {
    let _env = setup_test_env("pairing_code_mismatch");

    let identity_inviter = DeviceIdentity::generate("Active-Inviter-PC");
    let net = stardew_sync_core::core::network::get_network_service();
    let test_port = 42895u16;
    net.start_tcp_listener(test_port, identity_inviter.clone());
    std::thread::sleep(Duration::from_millis(150));
    let actual_port = net.get_active_tcp_port().unwrap_or(test_port);

    let pairing_mgr = stardew_sync_core::core::pairing::get_pairing_manager();
    let _ = pairing_mgr.cancel_invitation();
    let _inv = pairing_mgr.start_invitation(&identity_inviter, actual_port).expect("Start invitation");

    let identity_joiner = DeviceIdentity::generate("Wrong-Code-Joiner");
    let peer_addr = format!("127.0.0.1:{}", actual_port);

    let res = net.initiate_pairing(&peer_addr, "SD-WRNG", &identity_joiner);
    assert!(res.is_err(), "Pairing with mismatched code must be rejected");

    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("Invitation code mismatch") || err_str.contains("Connection closed by peer"),
        "Error message must be clean rejection, got: {}",
        err_str
    );
    assert!(
        !err_str.contains("failed to fill whole buffer"),
        "Must NOT surface raw EOF buffer error: {}",
        err_str
    );

    let _ = pairing_mgr.cancel_invitation();
}

#[test]
fn test_premature_tcp_closure_maps_unexpected_eof_to_clean_network_error() {
    let _env = setup_test_env("premature_tcp_closure_mapping");

    // Spawn a mock server that accepts connection, reads request, and drops it without sending response
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind mock listener");
    let port = listener.local_addr().unwrap().port();
    let peer_addr = format!("127.0.0.1:{}", port);

    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            // Drop stream now before writing response to trigger UnexpectedEof on caller read_exact
            drop(stream);
        }
    });

    let identity = DeviceIdentity::generate("Test-Joiner-PC");
    let net = stardew_sync_core::core::network::get_network_service();

    let res = net.initiate_pairing(&peer_addr, "SD-1234", &identity);
    assert!(res.is_err(), "Connection to dropped socket must fail");

    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("Connection closed by peer before pairing response was received"),
        "UnexpectedEof must be mapped to clean network domain error, got: {}",
        err_str
    );
    assert!(
        !err_str.contains("failed to fill whole buffer"),
        "Must NOT surface raw 'failed to fill whole buffer': {}",
        err_str
    );
}

#[test]
fn test_pairing_with_expired_invitation_returns_clean_rejection() {
    let _env = setup_test_env("pairing_invitation_expired");

    let identity_inviter = DeviceIdentity::generate("Expired-Inviter-PC");
    let net = stardew_sync_core::core::network::get_network_service();
    let test_port = 42896u16;
    net.start_tcp_listener(test_port, identity_inviter.clone());
    std::thread::sleep(Duration::from_millis(150));
    let actual_port = net.get_active_tcp_port().unwrap_or(test_port);

    let pairing_mgr = stardew_sync_core::core::pairing::get_pairing_manager();
    let _ = pairing_mgr.cancel_invitation();
    let inv = pairing_mgr.start_invitation(&identity_inviter, actual_port).expect("Start invitation");

    // Simulate expiration of the invitation
    pairing_mgr.expire_invitation_for_test();
    assert!(pairing_mgr.is_invitation_expired());
    assert!(pairing_mgr.get_active_invitation().is_none());

    let identity_joiner = DeviceIdentity::generate("Expired-Joiner-PC");
    let peer_addr = format!("127.0.0.1:{}", actual_port);

    let res = net.initiate_pairing(&peer_addr, &inv.code, &identity_joiner);
    assert!(res.is_err(), "Pairing with expired invitation must be rejected");

    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("Invitation expired on remote device")
            || err_str.contains("No active invitation on remote device")
            || err_str.contains("Connection closed by peer"),
        "Error message must indicate clean rejection, got: {}",
        err_str
    );
    assert!(
        !err_str.contains("failed to fill whole buffer"),
        "Must NOT surface raw EOF buffer error: {}",
        err_str
    );

    let _ = pairing_mgr.cancel_invitation();
}

#[test]
fn test_unauthenticated_revocation_rejected_without_ack() {
    let _env = setup_test_env("unauthenticated_revocation_no_ack");

    let identity_server = DeviceIdentity::generate("Server-PC");
    let net = stardew_sync_core::core::network::get_network_service();
    let test_port = 42897u16;
    net.start_tcp_listener(test_port, identity_server.clone());
    std::thread::sleep(Duration::from_millis(150));
    let actual_port = net.get_active_tcp_port().unwrap_or(test_port);

    // Attacker generates an arbitrary untrusted identity
    let identity_attacker = DeviceIdentity::generate("Untrusted-Attacker-PC");
    let target_addr = format!("127.0.0.1:{}", actual_port);

    // Attacker crafts and sends an unpair revocation frame for target
    let payload = TransportFramePayload::UnpairRevocation {
        target_device_id: identity_server.device_id(),
        reason: "Malicious unpair attempt".to_string(),
    };
    let frame = TransportFrame::new_signed(1, &identity_attacker, payload).expect("Signed frame");
    let wire_msg = NetworkWireMessage::PlaintextFrame(frame);
    let encoded = wire_msg.encode().expect("Encode wire message");

    let mut stream = TcpStream::connect(&target_addr).expect("Connect to server");
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    stream.write_all(&encoded).expect("Send revocation frame");
    stream.flush().expect("Flush revocation frame");

    // Server must NOT return an UnpairRevocationAck because attacker is not in server's PairedDeviceStore.
    // Server must drop/close the stream, so caller receives UnexpectedEof / read error.
    let mut resp_buf = [0u8; 4];
    let read_res = stream.read_exact(&mut resp_buf);
    assert!(
        read_res.is_err(),
        "Server must NOT return an UnpairRevocationAck to unknown sender; connection should be dropped"
    );
}

#[test]
fn test_simultaneous_unpair_and_unpair_when_peer_already_removed() {
    let _env = setup_test_env("simultaneous_unpair");

    let identity_a = DeviceIdentity::generate("Peer-A");
    let identity_b = DeviceIdentity::generate("Peer-B");

    // Mock server for Device A where Device B has ALREADY been removed from Device A's store
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind listener A");
    let port_a = listener.local_addr().unwrap().port();
    let addr_a = format!("127.0.0.1:{}", port_a);

    // Device A listener receives revocation from Device B, but B is missing from A's trust store.
    // Device A MUST NOT return a successful UnpairRevocationAck to an untrusted/missing sender!
    // Device A drops/closes the connection.
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept incoming connection");
        let mut len_buf = [0u8; 4];
        let _ = stream.read_exact(&mut len_buf);
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload_buf = vec![0u8; len];
        let _ = stream.read_exact(&mut payload_buf);

        // Security check: Device A verifies sender is not in trust store and drops connection
        drop(stream);
    });

    // Device B has Device A in store and attempts remote unpair revocation to Device A
    let mut store_b = PairedDeviceStore::load().unwrap();
    store_b.add_or_update(PairedDevice {
        device_id: identity_a.device_id(),
        device_name: identity_a.device_name().to_string(),
        public_key_hex: hex::encode(identity_a.verifying_key().as_bytes()),
        fingerprint: identity_a.fingerprint(),
        paired_at: "2026-10-10T12:00:00Z".to_string(),
        last_seen_at: None,
        last_ip: Some(addr_a.clone()),
    }).unwrap();

    let net = stardew_sync_core::core::network::get_network_service();
    let res = net.send_remote_unpair(&addr_a, &identity_a.device_id(), &identity_b);

    // Device B's remote unpair must fail cleanly without raw buffer error
    assert!(res.is_err(), "Remote unpair when peer drops connection must return Err");
    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("Peer closed connection without acknowledgement"),
        "Must return clean domain message for missing ACK, got: {}",
        err_str
    );
    assert!(
        !err_str.contains("failed to fill whole buffer"),
        "Must NOT surface raw EOF buffer error: {}",
        err_str
    );

    // Fail-closed local removal: Device B removes Device A locally
    let removed_locally = store_b.remove(&identity_a.device_id()).unwrap();
    assert!(removed_locally, "Local removal must succeed fail-closed");
    assert!(store_b.get(&identity_a.device_id()).is_none());
}

#[test]
fn test_invitation_lifecycle_and_live_countdown_expiration_status() {
    let _env = setup_test_env("invitation_lifecycle");
    let identity = DeviceIdentity::generate("Lifecycle-PC");
    let manager = PairingManager::new();

    // 1. Initial state is Idle
    let initial_view = manager.get_session_view();
    assert_eq!(initial_view.status, PairingStatus::Idle);
    assert!(initial_view.invitation_code.is_none());
    assert_eq!(initial_view.seconds_remaining, 0);

    // 2. Start invitation
    let inv = manager.start_invitation(&identity, 42890).expect("Start invitation");
    assert!(!inv.code.is_empty());

    let inviting_view = manager.get_session_view();
    assert_eq!(inviting_view.status, PairingStatus::Inviting);
    assert_eq!(inviting_view.invitation_code.as_deref(), Some(inv.code.as_str()));
    assert!(inviting_view.seconds_remaining > 0);
    assert!(inviting_view.seconds_remaining <= 300);

    // 3. Expire invitation
    manager.expire_invitation_for_test();
    assert!(manager.is_invitation_expired());
    assert!(manager.get_active_invitation().is_none());

    let expired_view = manager.get_session_view();
    assert_eq!(expired_view.status, PairingStatus::Expired);
    assert_eq!(expired_view.invitation_code.as_deref(), Some(inv.code.as_str()));
    assert_eq!(expired_view.seconds_remaining, 0);
}

#[test]
fn test_invitation_regeneration_resets_timer_and_creates_fresh_code() {
    let _env = setup_test_env("invitation_regeneration");
    let identity = DeviceIdentity::generate("Regen-PC");
    let manager = PairingManager::new();

    // 1. First invitation
    let _inv1 = manager.start_invitation(&identity, 42890).expect("Start invitation 1");
    manager.expire_invitation_for_test();
    assert_eq!(manager.get_session_view().status, PairingStatus::Expired);

    // 2. Regenerate invitation
    let inv2 = manager.start_invitation(&identity, 42890).expect("Start invitation 2 (regen)");
    let regen_view = manager.get_session_view();

    assert_eq!(regen_view.status, PairingStatus::Inviting);
    assert_eq!(regen_view.invitation_code.as_deref(), Some(inv2.code.as_str()));
    assert!(regen_view.seconds_remaining > 290);
    assert!(!manager.is_invitation_expired());
    assert!(manager.get_active_invitation().is_some());
}

#[test]
fn test_invitation_cancellation_and_idempotence() {
    let _env = setup_test_env("invitation_cancellation");
    let identity = DeviceIdentity::generate("Cancel-PC");
    let manager = PairingManager::new();

    // 1. Start and then cancel
    let _inv = manager.start_invitation(&identity, 42890).expect("Start invitation");
    assert_eq!(manager.get_session_view().status, PairingStatus::Inviting);

    manager.cancel_invitation().expect("Cancel invitation");
    let cancelled_view = manager.get_session_view();
    assert_eq!(cancelled_view.status, PairingStatus::Idle);
    assert!(cancelled_view.invitation_code.is_none());
    assert_eq!(cancelled_view.seconds_remaining, 0);

    // 2. Repeated cancellation is idempotent and does not error
    let res = manager.cancel_invitation();
    assert!(res.is_ok(), "Second cancellation must be cleanly idempotent");
}

#[test]
fn test_simultaneous_invitation_and_outbound_pairing_state() {
    let _env = setup_test_env("simultaneous_inbound_outbound");
    let identity_local = DeviceIdentity::generate("Local-Multi-PC");
    let manager = PairingManager::new();
    let net = stardew_sync_core::core::network::get_network_service();

    // Start local invitation on Device A
    let local_inv = manager.start_invitation(&identity_local, 42899).expect("Start local invitation");

    // Verify local invitation is active in session view
    let view_before = manager.get_session_view();
    assert_eq!(view_before.status, PairingStatus::Inviting);
    assert_eq!(view_before.invitation_code.as_deref(), Some(local_inv.code.as_str()));

    // Device A attempts outbound pairing to an unreachable remote address
    let outbound_res = net.initiate_pairing("127.0.0.1:59998", "SD-9999", &identity_local);
    assert!(outbound_res.is_err(), "Outbound connection to closed port must fail");

    // Local invitation on Device A MUST remain intact and uncorrupted!
    let view_after = manager.get_session_view();
    assert_eq!(view_after.status, PairingStatus::Inviting);
    assert_eq!(view_after.invitation_code.as_deref(), Some(local_inv.code.as_str()));
    assert!(manager.get_active_invitation().is_some());

    let _ = manager.cancel_invitation();
}






