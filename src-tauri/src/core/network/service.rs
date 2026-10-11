use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use chrono::Utc;
use ed25519_dalek::VerifyingKey;
use uuid::Uuid;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use crate::core::errors::CoreError;
use crate::core::identity::{DeviceIdentity, PublicDeviceInfo};
use crate::core::network::discovery::{get_discovery_manager, LanBeacon, DEFAULT_DISCOVERY_PORT};
use crate::core::network::transport::{
    get_connection_manager, EncryptedTransportFrame, NetworkWireMessage, PeerConnectionStatus,
    TransportFrame, TransportFramePayload, MAX_FRAME_SIZE, TRANSPORT_PROTOCOL_VERSION,
};
use crate::core::pairing::{
    compute_sas_code, generate_nonce_hex, get_pairing_manager, PairedDeviceStore,
    PairingConfirm, PairingFinalAck, PairingReject, PairingRequest, PairingResponse, PairingStatus, INVITATION_EXPIRATION_SECS,
};

pub struct NetworkService {
    running: AtomicBool,
    tcp_running: AtomicBool,
    announcer_running: AtomicBool,
    active_tcp_port: Mutex<Option<u16>>,
}

impl NetworkService {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            tcp_running: AtomicBool::new(false),
            announcer_running: AtomicBool::new(false),
            active_tcp_port: Mutex::new(None),
        }
    }

    pub fn set_active_tcp_port(&self, port: u16) {
        if let Ok(mut lock) = self.active_tcp_port.lock() {
            *lock = Some(port);
        }
    }

    pub fn get_active_tcp_port(&self) -> Option<u16> {
        self.active_tcp_port.lock().ok().and_then(|g| *g)
    }

    /// Starts the background UDP discovery listener and beacon processor
    pub fn start_discovery_listener(&self, local_device_id: String) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let is_running = Arc::new(AtomicBool::new(true));
        let r_flag = is_running.clone();
        let last_reactive_announce = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10)));
        let last_reactive_ann_clone = last_reactive_announce.clone();

        thread::spawn(move || {
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", DEFAULT_DISCOVERY_PORT)) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!(
                        "[bazq-network] Discovery port {} unavailable ({}). LAN auto-discovery may require manual IP entry.",
                        DEFAULT_DISCOVERY_PORT, err
                    );
                    return;
                }
            };

            let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));
            let mut buf = [0u8; 2048];

            while r_flag.load(Ordering::Relaxed) {
                if let Ok((len, src)) = socket.recv_from(&mut buf) {
                    if let Ok(beacon) = serde_json::from_slice::<LanBeacon>(&buf[..len]) {
                        let paired_ids: Vec<String> = PairedDeviceStore::load()
                            .map(|s| s.list().into_iter().map(|d| d.device_id).collect())
                            .unwrap_or_default();

                        let is_external = beacon.device_id != local_device_id;
                        get_discovery_manager().record_beacon(
                            beacon,
                            src.ip(),
                            &local_device_id,
                            &paired_ids,
                        );

                        // Rate-limited reactive announce-back (max 1 per sec): reply promptly to peer beacons
                        // so Refresh button and LAN presence update within milliseconds without waiting for 3s ticker
                        if is_external {
                            if let Ok(mut last_ann) = last_reactive_ann_clone.lock() {
                                if last_ann.elapsed() >= Duration::from_millis(1000) {
                                    *last_ann = Instant::now();
                                    if let Ok(identity) = DeviceIdentity::load_or_create() {
                                        let net = get_network_service();
                                        let port = net.get_active_tcp_port().unwrap_or(DEFAULT_DISCOVERY_PORT);
                                        let manager = get_pairing_manager();
                                        let is_inviting = manager.get_active_invitation().is_some();
                                        let _ = get_discovery_manager().broadcast_beacon(
                                            &identity.public_info(),
                                            port,
                                            is_inviting,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Starts the background periodic UDP beacon announcer for zero-configuration bidirectional discovery
    pub fn start_discovery_announcer(&self, local_identity: DeviceIdentity) {
        if self.announcer_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let is_running = Arc::new(AtomicBool::new(true));
        let r_flag = is_running.clone();

        thread::spawn(move || {
            while r_flag.load(Ordering::Relaxed) {
                let net = get_network_service();
                let port = net.get_active_tcp_port().unwrap_or(DEFAULT_DISCOVERY_PORT);
                let manager = get_pairing_manager();
                let is_inviting = manager.get_active_invitation().is_some();

                let _ = get_discovery_manager().broadcast_beacon(
                    &local_identity.public_info(),
                    port,
                    is_inviting,
                );

                thread::sleep(Duration::from_secs(3));
            }
        });
    }

    /// Starts the background TCP listener to accept pairing handshakes and authenticated encrypted frames
    pub fn start_tcp_listener(&self, initial_port: u16, local_identity: DeviceIdentity) {
        if self.tcp_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let listener = match TcpListener::bind(format!("0.0.0.0:{}", initial_port)) {
            Ok(l) => {
                self.set_active_tcp_port(initial_port);
                l
            }
            Err(_) => {
                // Fallback to initial_port + 1 or OS-assigned port
                match TcpListener::bind(format!("0.0.0.0:{}", initial_port + 1)) {
                    Ok(l) => {
                        self.set_active_tcp_port(initial_port + 1);
                        l
                    }
                    Err(_) => match TcpListener::bind("0.0.0.0:0") {
                        Ok(l) => {
                            if let Ok(addr) = l.local_addr() {
                                self.set_active_tcp_port(addr.port());
                            }
                            l
                        }
                        Err(e) => {
                            eprintln!("[bazq-network] Failed to bind TCP listener: {}", e);
                            self.tcp_running.store(false, Ordering::SeqCst);
                            return;
                        }
                    },
                }
            }
        };

        let _ = listener.set_nonblocking(true);
        let actual_port = self.get_active_tcp_port().unwrap_or(initial_port);
        println!("[bazq-network] TCP listener active on 0.0.0.0:{}", actual_port);

        let is_running = Arc::new(AtomicBool::new(true));
        let r_flag = is_running.clone();

        thread::spawn(move || {
            while r_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, peer_addr)) => {
                        let _ = stream.set_nonblocking(false);
                        let identity_clone = local_identity.clone();
                        thread::spawn(move || {
                            handle_incoming_tcp_stream(stream, peer_addr, identity_clone);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        eprintln!("[bazq-network] TCP accept error: {}", e);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });
    }

    /// Connects to a peer over TCP to initiate a pairing handshake
    pub fn initiate_pairing(
        &self,
        peer_addr: &str,
        invitation_code: &str,
        local_identity: &DeviceIdentity,
    ) -> Result<String, CoreError> {
        println!("[bazq-pairing] Initiating pairing handshake to {}", peer_addr);
        let mut stream = TcpStream::connect(peer_addr)
            .map_err(|e| CoreError::Network(format!("Failed to connect to peer at {}: {}", peer_addr, e)))?;

        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

        // Generate ephemeral ECDH keypair
        let mut csprng = rand::rngs::OsRng;
        let ephemeral_secret = EphemeralSecret::random_from_rng(&mut csprng);
        let ephemeral_pub = X25519PublicKey::from(&ephemeral_secret);
        let joiner_ecdh_hex = hex::encode(ephemeral_pub.as_bytes());

        let joiner_nonce_hex = generate_nonce_hex();
        let joiner_pk_hex = hex::encode(local_identity.verifying_key().as_bytes());

        // Construct PairingRequest payload
        let req_payload_bytes = format!(
            "{}:{}:{}:{}",
            invitation_code,
            local_identity.device_id(),
            joiner_ecdh_hex,
            joiner_nonce_hex
        );
        let signature_bytes = local_identity.sign(req_payload_bytes.as_bytes());
        let signature_hex = hex::encode(signature_bytes);

        let request = PairingRequest {
            protocol_version: TRANSPORT_PROTOCOL_VERSION,
            invitation_code: invitation_code.to_string(),
            joiner_device_id: local_identity.device_id(),
            joiner_device_name: local_identity.device_name().to_string(),
            joiner_public_key_hex: joiner_pk_hex.clone(),
            joiner_ecdh_public_hex: joiner_ecdh_hex.clone(),
            joiner_nonce_hex: joiner_nonce_hex.clone(),
            signature_hex,
        };

        // Send request
        let wire_req = NetworkWireMessage::PairingRequest(request);
        let encoded_req = wire_req.encode()?;
        if let Err(e) = stream.write_all(&encoded_req).and_then(|_| stream.flush()) {
            eprintln!("[bazq-network] Failed to send pairing request to peer at {}: {:?}", peer_addr, e.kind());
            return Err(CoreError::Network(format!(
                "Failed to send pairing request to peer at {}. The peer may have closed the connection or restarted.",
                peer_addr
            )));
        }

        // Read response
        let mut resp_len_buf = [0u8; 4];
        if let Err(e) = stream.read_exact(&mut resp_len_buf) {
            match e.kind() {
                std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe => {
                    eprintln!("[bazq-network] Peer at {} closed connection prematurely before handshake response (kind={:?})", peer_addr, e.kind());
                    return Err(CoreError::Network(
                        "Connection closed by peer before pairing response was received. Ensure the peer is running, has an active invitation, and is on the same network.".to_string(),
                    ));
                }
                _ => {
                    return Err(CoreError::Network(format!(
                        "Failed to read handshake response header from peer at {}: {}",
                        peer_addr, e
                    )));
                }
            }
        }
        let resp_len = u32::from_be_bytes(resp_len_buf) as usize;
        if resp_len > MAX_FRAME_SIZE {
            return Err(CoreError::Network("Peer response exceeded maximum frame size".to_string()));
        }

        let mut resp_buf = vec![0u8; resp_len];
        if let Err(e) = stream.read_exact(&mut resp_buf) {
            match e.kind() {
                std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe => {
                    eprintln!("[bazq-network] Peer at {} truncated handshake response payload (kind={:?})", peer_addr, e.kind());
                    return Err(CoreError::Network(
                        "Truncated handshake response payload received from peer".to_string(),
                    ));
                }
                _ => {
                    return Err(CoreError::Network(format!(
                        "Failed to read handshake response payload from peer at {}: {}",
                        peer_addr, e
                    )));
                }
            }
        }

        let mut full_resp_buf = Vec::with_capacity(4 + resp_len);
        full_resp_buf.extend_from_slice(&resp_len_buf);
        full_resp_buf.extend_from_slice(&resp_buf);

        let wire_resp = NetworkWireMessage::decode(&full_resp_buf)?;
        let response = match wire_resp {
            NetworkWireMessage::PairingResponse(r) => r,
            NetworkWireMessage::PairingReject(rej) => {
                eprintln!("[bazq-pairing] Handshake explicitly rejected by peer {}: {}", rej.sender_device_id, rej.rejection_reason);
                return Err(CoreError::Pairing(format!("Pairing rejected by peer: {}", rej.rejection_reason)));
            }
            _ => return Err(CoreError::Network("Unexpected wire frame received during pairing handshake".to_string())),
        };

        // Verify inviter's signature
        let inviter_vk_bytes = hex::decode(&response.inviter_public_key_hex)
            .map_err(|e| CoreError::Network(format!("Invalid inviter public key hex: {}", e)))?;
        if inviter_vk_bytes.len() != 32 {
            return Err(CoreError::Network("Invalid inviter public key length".to_string()));
        }
        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&inviter_vk_bytes);
        let inviter_vk = VerifyingKey::from_bytes(&vk_arr)
            .map_err(|e| CoreError::Network(format!("Invalid inviter public key: {}", e)))?;

        let expected_resp_data = format!(
            "{}:{}:{}:{}",
            response.session_id,
            response.inviter_device_id,
            response.inviter_ecdh_public_hex,
            response.inviter_nonce_hex
        );
        let resp_sig_bytes = hex::decode(&response.signature_hex)
            .map_err(|e| CoreError::Network(format!("Invalid response signature hex: {}", e)))?;

        DeviceIdentity::verify(&inviter_vk, expected_resp_data.as_bytes(), &resp_sig_bytes)?;

        // Compute Diffie-Hellman shared secret
        let inviter_ecdh_bytes = hex::decode(&response.inviter_ecdh_public_hex)
            .map_err(|e| CoreError::Network(format!("Invalid inviter ECDH key hex: {}", e)))?;
        if inviter_ecdh_bytes.len() != 32 {
            return Err(CoreError::Network("Invalid inviter ECDH key length".to_string()));
        }
        let mut ecdh_arr = [0u8; 32];
        ecdh_arr.copy_from_slice(&inviter_ecdh_bytes);
        let inviter_ecdh_pub = X25519PublicKey::from(ecdh_arr);
        let shared_secret = ephemeral_secret.diffie_hellman(&inviter_ecdh_pub);

        // Compute SAS code
        let sas_code = compute_sas_code(
            invitation_code,
            &response.inviter_public_key_hex,
            &joiner_pk_hex,
            &response.inviter_ecdh_public_hex,
            &joiner_ecdh_hex,
            &response.inviter_nonce_hex,
            &joiner_nonce_hex,
        );

        // Initialize local session bound to the verified shared secret
        println!(
            "[bazq-pairing] Handshake response verified from peer '{}' ({}); starting confirmation session",
            response.inviter_device_name, response.inviter_device_id
        );

        // Populate peer info
        let peer_info = PublicDeviceInfo {
            device_id: response.inviter_device_id,
            fingerprint: DeviceIdentity::format_fingerprint(&inviter_vk),
            device_name: response.inviter_device_name,
            public_key_hex: response.inviter_public_key_hex.clone(),
            created_at: Utc::now().to_rfc3339(),
        };

        let pairing_mgr = get_pairing_manager();
        pairing_mgr.start_confirmation_session_with_secret(
            &response.session_id,
            invitation_code,
            &response.inviter_public_key_hex,
            &joiner_pk_hex,
            &response.inviter_ecdh_public_hex,
            &joiner_ecdh_hex,
            &response.inviter_nonce_hex,
            &joiner_nonce_hex,
            peer_info,
            false,
            Some(shared_secret.to_bytes()),
        )?;

        // Spawn background worker on this live stream to handle mutual dual confirmation exchange
        let identity_clone = local_identity.clone();
        let session_id_clone = response.session_id.clone();
        thread::spawn(move || {
            run_pairing_dual_confirmation_loop(stream, session_id_clone, identity_clone, inviter_vk);
        });

        Ok(sas_code)
    }

    /// Sends a ping frame to a paired peer and validates the authenticated pong response
    pub fn ping_peer(
        &self,
        peer_addr: &str,
        peer_device_id: &str,
        local_identity: &DeviceIdentity,
    ) -> Result<u64, CoreError> {
        let mut stream = TcpStream::connect(peer_addr)
            .map_err(|e| CoreError::Network(format!("Cannot connect to peer at {}: {}", peer_addr, e)))?;

        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

        let store = PairedDeviceStore::load()?;
        let peer_record = store.get(peer_device_id).ok_or_else(|| {
            CoreError::Pairing(format!("Peer {} is not in trusted device database", peer_device_id))
        })?;

        let conn_mgr = get_connection_manager();
        let now_ms = Utc::now().timestamp_millis() as u64;
        let payload = TransportFramePayload::Ping { timestamp_ms: now_ms };

        // Prefer AEAD authenticated encryption if an active session key exists
        if let Some((seq, session_key)) = conn_mgr.advance_send_sequence(peer_device_id) {
            let enc_frame = EncryptedTransportFrame::encrypt(
                seq,
                &local_identity.device_id(),
                &payload,
                &session_key,
            )?;
            let wire_msg = NetworkWireMessage::Encrypted(enc_frame);
            let encoded = wire_msg.encode()?;
            if let Err(e) = stream.write_all(&encoded).and_then(|_| stream.flush()) {
                return Err(CoreError::Network(format!("Failed to send encrypted ping to peer: {}", e)));
            }

            let mut len_buf = [0u8; 4];
            if let Err(e) = stream.read_exact(&mut len_buf) {
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe => {
                        return Err(CoreError::Network("Peer closed connection before pong response".to_string()));
                    }
                    _ => return Err(CoreError::Network(format!("Failed to read pong response header: {}", e))),
                }
            }
            let len = u32::from_be_bytes(len_buf) as usize;
            if len > MAX_FRAME_SIZE {
                return Err(CoreError::Network("Frame exceeded maximum allowed size".to_string()));
            }

            let mut payload_buf = vec![0u8; len];
            if let Err(e) = stream.read_exact(&mut payload_buf) {
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe => {
                        return Err(CoreError::Network("Truncated pong payload received from peer".to_string()));
                    }
                    _ => return Err(CoreError::Network(format!("Failed to read pong payload: {}", e))),
                }
            }

            let mut full_frame_buf = Vec::with_capacity(4 + len);
            full_frame_buf.extend_from_slice(&len_buf);
            full_frame_buf.extend_from_slice(&payload_buf);

            let resp_wire = NetworkWireMessage::decode(&full_frame_buf)?;
            match resp_wire {
                NetworkWireMessage::Encrypted(resp_enc) => {
                    let resp_payload = resp_enc.decrypt(&session_key, 1)?;
                    match resp_payload {
                        TransportFramePayload::Pong { timestamp_ms } => {
                            conn_mgr.set_status(peer_device_id, PeerConnectionStatus::Connected);
                            Ok(timestamp_ms)
                        }
                        _ => Err(CoreError::Network("Unexpected response frame type for Ping".to_string())),
                    }
                }
                _ => Err(CoreError::Network("Expected encrypted response frame".to_string())),
            }
        } else {
            // Signed frame fallback
            let frame = TransportFrame::new_signed(1, local_identity, payload)?;
            let wire_msg = NetworkWireMessage::PlaintextFrame(frame);
            let encoded = wire_msg.encode()?;
            if let Err(e) = stream.write_all(&encoded).and_then(|_| stream.flush()) {
                return Err(CoreError::Network(format!("Failed to send signed ping to peer: {}", e)));
            }

            let mut len_buf = [0u8; 4];
            if let Err(e) = stream.read_exact(&mut len_buf) {
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe => {
                        return Err(CoreError::Network("Peer closed connection before pong response".to_string()));
                    }
                    _ => return Err(CoreError::Network(format!("Failed to read pong response header: {}", e))),
                }
            }
            let len = u32::from_be_bytes(len_buf) as usize;
            if len > MAX_FRAME_SIZE {
                return Err(CoreError::Network("Frame exceeded maximum allowed size".to_string()));
            }

            let mut payload_buf = vec![0u8; len];
            if let Err(e) = stream.read_exact(&mut payload_buf) {
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe => {
                        return Err(CoreError::Network("Truncated pong payload received from peer".to_string()));
                    }
                    _ => return Err(CoreError::Network(format!("Failed to read pong payload: {}", e))),
                }
            }

            let mut full_frame_buf = Vec::with_capacity(4 + len);
            full_frame_buf.extend_from_slice(&len_buf);
            full_frame_buf.extend_from_slice(&payload_buf);

            let resp_wire = NetworkWireMessage::decode(&full_frame_buf)?;
            match resp_wire {
                NetworkWireMessage::PlaintextFrame(resp_frame) => {
                    let vk_bytes = hex::decode(&peer_record.public_key_hex)
                        .map_err(|e| CoreError::Network(format!("Invalid pinned key hex: {}", e)))?;
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&vk_bytes);
                    let vk = VerifyingKey::from_bytes(&arr)
                        .map_err(|e| CoreError::Network(format!("Invalid pinned verifying key: {}", e)))?;

                    resp_frame.verify_signature(&vk)?;

                    match resp_frame.payload {
                        TransportFramePayload::Pong { timestamp_ms } => {
                            conn_mgr.set_status(peer_device_id, PeerConnectionStatus::Connected);
                            Ok(timestamp_ms)
                        }
                        _ => Err(CoreError::Network("Unexpected response frame type for Ping".to_string())),
                    }
                }
                _ => Err(CoreError::Network("Unexpected response wire type for Ping".to_string())),
            }
        }
    }

    /// Sends an authenticated revocation frame to a remote peer over TCP to revoke mutual pairing
    pub fn send_remote_unpair(
        &self,
        peer_addr: &str,
        peer_device_id: &str,
        local_identity: &DeviceIdentity,
    ) -> Result<bool, CoreError> {
        let stream_res = if let Ok(sock_addr) = peer_addr.parse::<SocketAddr>() {
            TcpStream::connect_timeout(&sock_addr, Duration::from_secs(3))
        } else {
            TcpStream::connect(peer_addr)
        };

        let mut stream = stream_res
            .map_err(|e| CoreError::Network(format!("Failed to connect to peer at {}: {}", peer_addr, e)))?;

        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

        let store = PairedDeviceStore::load()?;
        let peer_record = store.get(peer_device_id).ok_or_else(|| {
            CoreError::Pairing(format!("Peer {} is not in trusted device database", peer_device_id))
        })?;

        let payload = TransportFramePayload::UnpairRevocation {
            target_device_id: peer_device_id.to_string(),
            reason: "User requested unpair".to_string(),
        };

        let frame = TransportFrame::new_signed(1, local_identity, payload)?;
        let wire_msg = NetworkWireMessage::PlaintextFrame(frame);
        let encoded = wire_msg.encode()?;
        if let Err(e) = stream.write_all(&encoded).and_then(|_| stream.flush()) {
            eprintln!("[bazq-network] Failed to send revocation frame to peer at {}: {:?}", peer_addr, e.kind());
            return Err(CoreError::Network(format!(
                "Failed to send revocation frame to peer at {}. The peer may have closed the connection or shut down.",
                peer_addr
            )));
        }

        let mut len_buf = [0u8; 4];
        if let Err(e) = stream.read_exact(&mut len_buf) {
            match e.kind() {
                std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe => {
                    eprintln!("[bazq-network] Peer at {} closed connection without revocation acknowledgement (peer may already have unpaired or disconnected, kind={:?})", peer_addr, e.kind());
                    return Err(CoreError::Network(
                        "Peer closed connection without acknowledgement (the device may already have unpaired or shut down)".to_string(),
                    ));
                }
                _ => {
                    return Err(CoreError::Network(format!(
                        "Failed to read revocation response header from peer at {}: {}",
                        peer_addr, e
                    )));
                }
            }
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(CoreError::Network("Revocation response exceeded maximum frame size".to_string()));
        }

        let mut payload_buf = vec![0u8; len];
        if let Err(e) = stream.read_exact(&mut payload_buf) {
            match e.kind() {
                std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe => {
                    eprintln!("[bazq-network] Truncated revocation response payload from {} (kind={:?})", peer_addr, e.kind());
                    return Err(CoreError::Network(
                        "Truncated revocation response payload received from peer".to_string(),
                    ));
                }
                _ => {
                    return Err(CoreError::Network(format!(
                        "Failed to read revocation response payload from peer at {}: {}",
                        peer_addr, e
                    )));
                }
            }
        }

        let mut full_frame_buf = Vec::with_capacity(4 + len);
        full_frame_buf.extend_from_slice(&len_buf);
        full_frame_buf.extend_from_slice(&payload_buf);

        let resp_wire = NetworkWireMessage::decode(&full_frame_buf)?;
        match resp_wire {
            NetworkWireMessage::PlaintextFrame(resp_frame) => {
                let vk_bytes = hex::decode(&peer_record.public_key_hex)
                    .map_err(|e| CoreError::Network(format!("Invalid pinned key hex: {}", e)))?;
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&vk_bytes);
                let vk = VerifyingKey::from_bytes(&arr)
                    .map_err(|e| CoreError::Network(format!("Invalid pinned verifying key: {}", e)))?;

                resp_frame.verify_signature(&vk)?;

                match resp_frame.payload {
                    TransportFramePayload::UnpairRevocationAck { success } => Ok(success),
                    _ => Err(CoreError::Network("Unexpected response frame type for unpair revocation".to_string())),
                }
            }
            _ => Err(CoreError::Network("Unexpected response wire type for unpair revocation".to_string())),
        }
    }
}

/// Handles incoming connection on the TCP listener
fn handle_incoming_tcp_stream(
    mut stream: TcpStream,
    _peer_addr: SocketAddr,
    local_identity: DeviceIdentity,
) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let mut len_buf = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut len_buf) {
        if e.kind() != std::io::ErrorKind::WouldBlock && e.kind() != std::io::ErrorKind::TimedOut {
            eprintln!("[bazq-network] Inbound TCP stream from {} closed before length header: kind={:?}", _peer_addr, e.kind());
        }
        return;
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        eprintln!("[bazq-network] Inbound frame from {} declared length {} exceeding {} bytes", _peer_addr, len, MAX_FRAME_SIZE);
        return;
    }

    let mut frame_buf = vec![0u8; len];
    if let Err(e) = stream.read_exact(&mut frame_buf) {
        eprintln!("[bazq-network] Inbound TCP stream from {} closed before frame payload (expected {} bytes): kind={:?}", _peer_addr, len, e.kind());
        return;
    }

    let mut full_buf = Vec::with_capacity(4 + len);
    full_buf.extend_from_slice(&len_buf);
    full_buf.extend_from_slice(&frame_buf);

    let wire_msg = match NetworkWireMessage::decode(&full_buf) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[bazq-network] Failed to decode inbound wire message from {}: {}", _peer_addr, e);
            return;
        }
    };

    match wire_msg {
        NetworkWireMessage::PairingRequest(req) => {
            let pairing_mgr = get_pairing_manager();
            let active_inv = match pairing_mgr.get_active_invitation() {
                Some(inv) => inv,
                None => {
                    let rejection_reason = if pairing_mgr.is_invitation_present() {
                        "Invitation expired on remote device".to_string()
                    } else {
                        "No active invitation on remote device".to_string()
                    };
                    eprintln!("[bazq-network] Incoming PairingRequest rejected from {}: {}", _peer_addr, rejection_reason);
                    let reject = PairingReject {
                        protocol_version: TRANSPORT_PROTOCOL_VERSION,
                        sender_device_id: local_identity.device_id(),
                        rejection_reason,
                    };
                    let wire_resp = NetworkWireMessage::PairingReject(reject);
                    if let Ok(encoded) = wire_resp.encode() {
                        let _ = stream.write_all(&encoded);
                        let _ = stream.flush();
                    }
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                }
            };

            if !active_inv.code.eq_ignore_ascii_case(&req.invitation_code) {
                eprintln!("[bazq-network] Incoming PairingRequest rejected from {}: Invitation code mismatch", _peer_addr);
                let reject = PairingReject {
                    protocol_version: TRANSPORT_PROTOCOL_VERSION,
                    sender_device_id: local_identity.device_id(),
                    rejection_reason: "Invitation code mismatch".to_string(),
                };
                let wire_resp = NetworkWireMessage::PairingReject(reject);
                if let Ok(encoded) = wire_resp.encode() {
                    let _ = stream.write_all(&encoded);
                    let _ = stream.flush();
                }
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return;
            }

            // Verify joiner signature
            let joiner_vk_bytes = match hex::decode(&req.joiner_public_key_hex) {
                Ok(b) if b.len() == 32 => b,
                _ => {
                    eprintln!("[bazq-network] Incoming PairingRequest rejected from {}: Invalid joiner public key format", _peer_addr);
                    let reject = PairingReject {
                        protocol_version: TRANSPORT_PROTOCOL_VERSION,
                        sender_device_id: local_identity.device_id(),
                        rejection_reason: "Invalid joiner public key format".to_string(),
                    };
                    let wire_resp = NetworkWireMessage::PairingReject(reject);
                    if let Ok(encoded) = wire_resp.encode() {
                        let _ = stream.write_all(&encoded);
                        let _ = stream.flush();
                    }
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                }
            };
            let mut vk_arr = [0u8; 32];
            vk_arr.copy_from_slice(&joiner_vk_bytes);
            let joiner_vk = match VerifyingKey::from_bytes(&vk_arr) {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("[bazq-network] Incoming PairingRequest rejected from {}: Invalid verifying key bytes", _peer_addr);
                    let reject = PairingReject {
                        protocol_version: TRANSPORT_PROTOCOL_VERSION,
                        sender_device_id: local_identity.device_id(),
                        rejection_reason: "Invalid verifying key".to_string(),
                    };
                    let wire_resp = NetworkWireMessage::PairingReject(reject);
                    if let Ok(encoded) = wire_resp.encode() {
                        let _ = stream.write_all(&encoded);
                        let _ = stream.flush();
                    }
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                }
            };

            let expected_req_data = format!(
                "{}:{}:{}:{}",
                req.invitation_code,
                req.joiner_device_id,
                req.joiner_ecdh_public_hex,
                req.joiner_nonce_hex
            );
            let sig_bytes = match hex::decode(&req.signature_hex) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("[bazq-network] Incoming PairingRequest rejected from {}: Invalid signature hex", _peer_addr);
                    let reject = PairingReject {
                        protocol_version: TRANSPORT_PROTOCOL_VERSION,
                        sender_device_id: local_identity.device_id(),
                        rejection_reason: "Invalid signature encoding".to_string(),
                    };
                    let wire_resp = NetworkWireMessage::PairingReject(reject);
                    if let Ok(encoded) = wire_resp.encode() {
                        let _ = stream.write_all(&encoded);
                        let _ = stream.flush();
                    }
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                }
            };
            if DeviceIdentity::verify(&joiner_vk, expected_req_data.as_bytes(), &sig_bytes).is_err() {
                eprintln!("[bazq-network] Incoming PairingRequest rejected from {}: Invalid Ed25519 signature", _peer_addr);
                let reject = PairingReject {
                    protocol_version: TRANSPORT_PROTOCOL_VERSION,
                    sender_device_id: local_identity.device_id(),
                    rejection_reason: "Cryptographic signature verification failed".to_string(),
                };
                let wire_resp = NetworkWireMessage::PairingReject(reject);
                if let Ok(encoded) = wire_resp.encode() {
                    let _ = stream.write_all(&encoded);
                    let _ = stream.flush();
                }
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return;
            }

            // Generate inviter ECDH keypair
            let mut csprng = rand::rngs::OsRng;
            let inviter_secret = EphemeralSecret::random_from_rng(&mut csprng);
            let inviter_pub = X25519PublicKey::from(&inviter_secret);
            let inviter_ecdh_hex = hex::encode(inviter_pub.as_bytes());

            // Compute Diffie-Hellman shared secret
            let joiner_ecdh_bytes = match hex::decode(&req.joiner_ecdh_public_hex) {
                Ok(b) if b.len() == 32 => b,
                _ => return,
            };
            let mut ecdh_arr = [0u8; 32];
            ecdh_arr.copy_from_slice(&joiner_ecdh_bytes);
            let joiner_ecdh_pub = X25519PublicKey::from(ecdh_arr);
            let shared_secret = inviter_secret.diffie_hellman(&joiner_ecdh_pub);

            let session_id = Uuid::new_v4().to_string();
            let inviter_nonce_hex = generate_nonce_hex();
            let inviter_pk_hex = hex::encode(local_identity.verifying_key().as_bytes());

            let resp_sign_data = format!(
                "{}:{}:{}:{}",
                session_id,
                local_identity.device_id(),
                inviter_ecdh_hex,
                inviter_nonce_hex
            );
            let resp_sig = local_identity.sign(resp_sign_data.as_bytes());

            let response = PairingResponse {
                protocol_version: TRANSPORT_PROTOCOL_VERSION,
                session_id: session_id.clone(),
                inviter_device_id: local_identity.device_id(),
                inviter_device_name: local_identity.device_name().to_string(),
                inviter_public_key_hex: inviter_pk_hex.clone(),
                inviter_ecdh_public_hex: inviter_ecdh_hex.clone(),
                inviter_nonce_hex: inviter_nonce_hex.clone(),
                signature_hex: hex::encode(resp_sig),
            };

            // Send response frame
            let wire_resp = NetworkWireMessage::PairingResponse(response);
            if let Ok(encoded) = wire_resp.encode() {
                let _ = stream.write_all(&encoded);
                let _ = stream.flush();
            }

            // Start confirmation session on inviter side
            println!(
                "[bazq-pairing] Incoming PairingRequest verified from '{}' ({}); starting confirmation session",
                req.joiner_device_name, req.joiner_device_id
            );
            let peer_info = PublicDeviceInfo {
                device_id: req.joiner_device_id.clone(),
                fingerprint: DeviceIdentity::format_fingerprint(&joiner_vk),
                device_name: req.joiner_device_name.clone(),
                public_key_hex: req.joiner_public_key_hex.clone(),
                created_at: Utc::now().to_rfc3339(),
            };

            let _ = pairing_mgr.start_confirmation_session_with_secret(
                &session_id,
                &req.invitation_code,
                &inviter_pk_hex,
                &req.joiner_public_key_hex,
                &inviter_ecdh_hex,
                &req.joiner_ecdh_public_hex,
                &inviter_nonce_hex,
                &req.joiner_nonce_hex,
                peer_info,
                true,
                Some(shared_secret.to_bytes()),
            );

            // Execute mutual dual confirmation exchange on this connection
            run_pairing_dual_confirmation_loop(stream, session_id, local_identity, joiner_vk);
        }
        NetworkWireMessage::Encrypted(enc_frame) => {
            let store = match PairedDeviceStore::load() {
                Ok(s) => s,
                Err(_) => return,
            };
            if store.get(&enc_frame.sender_device_id).is_none() {
                return;
            }

            let conn_mgr = get_connection_manager();
            let session_key = match conn_mgr.verify_and_advance_recv_sequence(&enc_frame.sender_device_id, enc_frame.sequence) {
                Ok(k) => k,
                Err(_) => return,
            };

            if let Ok(payload) = enc_frame.decrypt(&session_key, 1) {
                match payload {
                    TransportFramePayload::Ping { timestamp_ms } => {
                        if let Some((seq, key)) = conn_mgr.advance_send_sequence(&enc_frame.sender_device_id) {
                            let pong_payload = TransportFramePayload::Pong { timestamp_ms };
                            if let Ok(pong_frame) = EncryptedTransportFrame::encrypt(
                                seq,
                                &local_identity.device_id(),
                                &pong_payload,
                                &key,
                            ) {
                                let wire = NetworkWireMessage::Encrypted(pong_frame);
                                if let Ok(bytes) = wire.encode() {
                                    let _ = stream.write_all(&bytes);
                                    let _ = stream.flush();
                                    conn_mgr.set_status(&enc_frame.sender_device_id, PeerConnectionStatus::Connected);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        NetworkWireMessage::PlaintextFrame(frame) => {
            let store = match PairedDeviceStore::load() {
                Ok(s) => s,
                Err(_) => return,
            };
            let peer = match store.get(&frame.sender_device_id) {
                Some(p) => p,
                None => {
                    eprintln!(
                        "[bazq-network] Inbound plaintext frame rejected: sender '{}' is not in trusted device store",
                        frame.sender_device_id
                    );
                    return;
                }
            };

            let vk_bytes = match hex::decode(&peer.public_key_hex) {
                Ok(b) if b.len() == 32 => b,
                _ => {
                    eprintln!("[bazq-network] Pinned public key for '{}' has invalid byte length", frame.sender_device_id);
                    return;
                }
            };
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&vk_bytes);
            let vk = match VerifyingKey::from_bytes(&arr) {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("[bazq-network] Failed to parse pinned verifying key for '{}'", frame.sender_device_id);
                    return;
                }
            };

            if frame.verify_signature(&vk).is_err() {
                eprintln!("[bazq-network] Cryptographic signature check failed for inbound frame from '{}'", frame.sender_device_id);
                return;
            }

            match frame.payload {
                TransportFramePayload::Ping { timestamp_ms } => {
                    let pong = TransportFramePayload::Pong { timestamp_ms };
                    if let Ok(resp_frame) = TransportFrame::new_signed(frame.sequence + 1, &local_identity, pong) {
                        let wire = NetworkWireMessage::PlaintextFrame(resp_frame);
                        if let Ok(bytes) = wire.encode() {
                            let _ = stream.write_all(&bytes);
                            let _ = stream.flush();
                            get_connection_manager().set_status(&frame.sender_device_id, PeerConnectionStatus::Connected);
                        }
                    }
                }
                TransportFramePayload::UnpairRevocation { target_device_id: _, reason: _ } => {
                    // Authenticated revocation from trusted peer: remove peer from local store idempotently
                    if let Ok(mut store) = PairedDeviceStore::load() {
                        let _ = store.remove(&frame.sender_device_id);
                    }
                    get_connection_manager().set_status(&frame.sender_device_id, PeerConnectionStatus::Disconnected);
                    println!(
                        "[bazq-network] Authenticated trust revocation received and committed for peer {}",
                        frame.sender_device_id
                    );

                    let ack = TransportFramePayload::UnpairRevocationAck { success: true };
                    if let Ok(resp_frame) = TransportFrame::new_signed(frame.sequence + 1, &local_identity, ack) {
                        let wire = NetworkWireMessage::PlaintextFrame(resp_frame);
                        if let Ok(bytes) = wire.encode() {
                            let _ = stream.write_all(&bytes);
                            let _ = stream.flush();
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Runs the dual confirmation signaling loop over an active pairing TCP connection
fn run_pairing_dual_confirmation_loop(
    mut stream: TcpStream,
    session_id: String,
    local_identity: DeviceIdentity,
    peer_vk: VerifyingKey,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let pairing_mgr = get_pairing_manager();
    let mut our_sent = false;
    let mut final_ack_sent = false;
    let mut final_ack_received = false;
    let mut completed_time: Option<Instant> = None;

    let start_time = std::time::Instant::now();
    let max_duration = Duration::from_secs(INVITATION_EXPIRATION_SECS);

    while start_time.elapsed() < max_duration {
        let view = pairing_mgr.get_session_view();

        // 1. Send explicit rejection frame if user rejected in UI
        if !our_sent && view.status == PairingStatus::Rejected {
            if let Some((sess_id, sas_code, _peer_id)) = pairing_mgr.get_session_pairing_details() {
                if sess_id == session_id {
                    let confirm_data = format!("{}:{}:{}:false", sess_id, local_identity.device_id(), sas_code);
                    let sig = local_identity.sign(confirm_data.as_bytes());
                    let confirm = PairingConfirm {
                        session_id: sess_id,
                        sender_device_id: local_identity.device_id(),
                        sas_code,
                        accepted: false,
                        signature_hex: hex::encode(sig),
                    };
                    let msg = NetworkWireMessage::PairingConfirm(confirm);
                    if let Ok(bytes) = msg.encode() {
                        let _ = stream.write_all(&bytes);
                        let _ = stream.flush();
                    }
                }
            }
            let _ = stream.shutdown(std::net::Shutdown::Both);
            break;
        }

        // 2. Send local confirmation if user approved in UI
        if !our_sent && pairing_mgr.is_our_confirmed() {
            if let Some((sess_id, sas_code, _peer_id)) = pairing_mgr.get_session_pairing_details() {
                if sess_id == session_id {
                    let confirm_data = format!("{}:{}:{}:true", sess_id, local_identity.device_id(), sas_code);
                    let sig = local_identity.sign(confirm_data.as_bytes());
                    let confirm = PairingConfirm {
                        session_id: sess_id,
                        sender_device_id: local_identity.device_id(),
                        sas_code,
                        accepted: true,
                        signature_hex: hex::encode(sig),
                    };
                    let msg = NetworkWireMessage::PairingConfirm(confirm);
                    if let Ok(bytes) = msg.encode() {
                        let _ = stream.write_all(&bytes);
                        let _ = stream.flush();
                        our_sent = true;
                    }
                }
            }
        }

        // 3. Read incoming frames from peer
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf) {
            Ok(()) => {
                let len = u32::from_be_bytes(len_buf) as usize;
                if len <= MAX_FRAME_SIZE {
                    let mut frame_buf = vec![0u8; len];
                    match stream.read_exact(&mut frame_buf) {
                        Ok(()) => {
                            let mut full_buf = Vec::with_capacity(4 + len);
                            full_buf.extend_from_slice(&len_buf);
                            full_buf.extend_from_slice(&frame_buf);

                            if let Ok(wire_msg) = NetworkWireMessage::decode(&full_buf) {
                                match wire_msg {
                                    NetworkWireMessage::PairingConfirm(confirm) => {
                                        let expected_confirm_data = format!(
                                            "{}:{}:{}:{}",
                                            confirm.session_id,
                                            confirm.sender_device_id,
                                            confirm.sas_code,
                                            confirm.accepted
                                        );
                                        if let Ok(sig_bytes) = hex::decode(&confirm.signature_hex) {
                                            if DeviceIdentity::verify(&peer_vk, expected_confirm_data.as_bytes(), &sig_bytes).is_ok() {
                                                if confirm.accepted {
                                                    if let Ok(mut store) = PairedDeviceStore::load() {
                                                        let _ = pairing_mgr.confirm_remote(&confirm.sas_code, &mut store);
                                                    }
                                                } else {
                                                    let _ = pairing_mgr.reject();
                                                }
                                            }
                                        }
                                    }
                                    NetworkWireMessage::PairingFinalAck(ack) => {
                                        let ack_data = format!("FINAL_ACK:{}:{}", ack.session_id, ack.sender_device_id);
                                        if let Ok(sig_bytes) = hex::decode(&ack.signature_hex) {
                                            if DeviceIdentity::verify(&peer_vk, ack_data.as_bytes(), &sig_bytes).is_ok() {
                                                if ack.session_id == session_id {
                                                    final_ack_received = true;
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(ref e) => {
                            eprintln!("[bazq-pairing] TCP stream closed while reading frame payload of {} bytes: kind={:?}", len, e.kind());
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                // Read timeout is expected while polling
            }
            Err(ref e) => {
                // Peer disconnected
                eprintln!("[bazq-pairing] TCP connection closed during dual-confirmation exchange: kind={:?}", e.kind());
                let current_view = pairing_mgr.get_session_view();
                if current_view.status != PairingStatus::Completed {
                    // Pre-completion disconnect: fail closed!
                    eprintln!("[bazq-pairing] Peer disconnected before dual confirmation completed; rejecting session");
                    let _ = pairing_mgr.reject();
                }
                break;
            }
        }

        // 4. Send PairingFinalAck once our side has reached Completed
        let view_now = pairing_mgr.get_session_view();
        if view_now.status == PairingStatus::Completed && !final_ack_sent {
            let ack_data = format!("FINAL_ACK:{}:{}", session_id, local_identity.device_id());
            let sig = local_identity.sign(ack_data.as_bytes());
            let ack = PairingFinalAck {
                session_id: session_id.clone(),
                sender_device_id: local_identity.device_id(),
                signature_hex: hex::encode(sig),
            };
            let msg = NetworkWireMessage::PairingFinalAck(ack);
            if let Ok(bytes) = msg.encode() {
                let _ = stream.write_all(&bytes);
                let _ = stream.flush();
                final_ack_sent = true;
                completed_time = Some(Instant::now());
            }
        }

        // 5. Termination checks
        match view_now.status {
            PairingStatus::Completed => {
                // Graceful socket drain period: allow up to 2.0s for the peer to receive our confirm and final ack,
                // or exit cleanly if final ack was bidirectionally exchanged and 300ms has elapsed.
                if let Some(c_time) = completed_time {
                    if (final_ack_received && c_time.elapsed() >= Duration::from_millis(300))
                        || c_time.elapsed() >= Duration::from_millis(2000)
                    {
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        break;
                    }
                }
            }
            PairingStatus::Rejected | PairingStatus::Expired => {
                let _ = stream.shutdown(std::net::Shutdown::Both);
                break;
            }
            _ => {}
        }

        thread::sleep(Duration::from_millis(50));
    }
}

static GLOBAL_NETWORK_SERVICE: OnceLock<NetworkService> = OnceLock::new();

pub fn get_network_service() -> &'static NetworkService {
    GLOBAL_NETWORK_SERVICE.get_or_init(NetworkService::new)
}
