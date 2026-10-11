use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use crate::core::errors::CoreError;
use crate::core::identity::PublicDeviceInfo;

pub const DISCOVERY_BEACON_MAGIC: &str = "STARDEW_SYNC_BEACON";
pub const DISCOVERY_PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_DISCOVERY_PORT: u16 = 42890;
pub const PEER_EXPIRATION_SECS: u64 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanBeacon {
    pub magic: String,
    pub protocol_version: u32,
    pub device_id: String,
    pub device_name: String,
    pub fingerprint: String,
    pub tcp_port: u16,
    pub is_inviting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub device_id: String,
    pub device_name: String,
    pub fingerprint: String,
    pub address: String,
    pub is_inviting: bool,
    pub is_paired: bool,
    pub last_seen_secs_ago: u64,
}

struct PeerRecord {
    pub peer: DiscoveredPeer,
    pub last_seen: Instant,
}

pub struct DiscoveryManager {
    peers: Mutex<HashMap<String, PeerRecord>>,
    broadcaster_socket: Mutex<Option<Arc<UdpSocket>>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            peers: Mutex::new(HashMap::new()),
            broadcaster_socket: Mutex::new(None),
        }
    }

    /// Records or updates a discovered peer from an incoming beacon
    pub fn record_beacon(
        &self,
        beacon: LanBeacon,
        sender_ip: std::net::IpAddr,
        local_device_id: &str,
        paired_device_ids: &[String],
    ) {
        // Ignore self-broadcasts
        if beacon.device_id == local_device_id {
            return;
        }

        // Ignore unknown magic or mismatched protocol
        if beacon.magic != DISCOVERY_BEACON_MAGIC || beacon.protocol_version != DISCOVERY_PROTOCOL_VERSION {
            return;
        }

        let address = format!("{}:{}", sender_ip, beacon.tcp_port);
        let is_paired = paired_device_ids.contains(&beacon.device_id);

        let peer = DiscoveredPeer {
            device_id: beacon.device_id.clone(),
            device_name: beacon.device_name,
            fingerprint: beacon.fingerprint,
            address,
            is_inviting: beacon.is_inviting,
            is_paired,
            last_seen_secs_ago: 0,
        };

        if let Ok(mut lock) = self.peers.lock() {
            lock.insert(
                beacon.device_id,
                PeerRecord {
                    peer,
                    last_seen: Instant::now(),
                },
            );
        }
    }

    /// Lists currently discovered peers, pruning any peer not seen within PEER_EXPIRATION_SECS
    pub fn list_discovered_peers(&self, paired_device_ids: &[String]) -> Vec<DiscoveredPeer> {
        let mut result = Vec::new();
        let now = Instant::now();

        if let Ok(mut lock) = self.peers.lock() {
            // Prune expired peers
            lock.retain(|_, record| {
                now.duration_since(record.last_seen).as_secs() < PEER_EXPIRATION_SECS
            });

            for record in lock.values_mut() {
                let secs_ago = now.duration_since(record.last_seen).as_secs();
                record.peer.last_seen_secs_ago = secs_ago;
                record.peer.is_paired = paired_device_ids.contains(&record.peer.device_id);
                result.push(record.peer.clone());
            }
        }

        result.sort_by(|a, b| a.device_name.cmp(&b.device_name));
        result
    }

    /// Looks up an active discovered peer by device ID if seen within PEER_EXPIRATION_SECS
    pub fn get_peer(&self, device_id: &str) -> Option<DiscoveredPeer> {
        let now = Instant::now();
        if let Ok(mut lock) = self.peers.lock() {
            if let Some(record) = lock.get_mut(device_id) {
                let secs_ago = now.duration_since(record.last_seen).as_secs();
                if secs_ago < PEER_EXPIRATION_SECS {
                    record.peer.last_seen_secs_ago = secs_ago;
                    return Some(record.peer.clone());
                } else {
                    lock.remove(device_id);
                }
            }
        }
        None
    }

    #[doc(hidden)]
    pub fn simulate_peer_age(&self, device_id: &str, age_secs: u64) {
        if let Ok(mut lock) = self.peers.lock() {
            if let Some(record) = lock.get_mut(device_id) {
                record.last_seen = Instant::now()
                    .checked_sub(std::time::Duration::from_secs(age_secs))
                    .unwrap_or_else(Instant::now);
            }
        }
    }

    /// Broadcasts an announce beacon over UDP to the local subnet
    pub fn broadcast_beacon(
        &self,
        local_device: &PublicDeviceInfo,
        tcp_port: u16,
        is_inviting: bool,
    ) -> Result<(), CoreError> {
        let beacon = LanBeacon {
            magic: DISCOVERY_BEACON_MAGIC.to_string(),
            protocol_version: DISCOVERY_PROTOCOL_VERSION,
            device_id: local_device.device_id.clone(),
            device_name: local_device.device_name.clone(),
            fingerprint: local_device.fingerprint.clone(),
            tcp_port,
            is_inviting,
        };

        let json_bytes = serde_json::to_vec(&beacon)?;

        let socket = {
            let mut lock = self.broadcaster_socket.lock().map_err(|_| {
                CoreError::Network("Discovery socket lock poisoned".to_string())
            })?;
            if lock.is_none() {
                let s = UdpSocket::bind("0.0.0.0:0")
                    .map_err(|e| CoreError::Network(format!("Failed to bind UDP discovery broadcaster: {}", e)))?;
                s.set_broadcast(true)
                    .map_err(|e| CoreError::Network(format!("Failed to set UDP broadcast mode: {}", e)))?;
                *lock = Some(Arc::new(s));
            }
            lock.as_ref().unwrap().clone()
        };

        let targets = get_broadcast_targets();
        for target in targets {
            let _ = socket.send_to(&json_bytes, &target);
        }

        Ok(())
    }
}

/// Discovers broadcast targets across all active network interfaces
pub fn get_broadcast_targets() -> Vec<String> {
    let mut targets = vec![
        format!("255.255.255.255:{}", DEFAULT_DISCOVERY_PORT),
        format!("127.0.0.1:{}", DEFAULT_DISCOVERY_PORT),
    ];

    let networks = sysinfo::Networks::new_with_refreshed_list();
    for (_name, data) in &networks {
        for net in data.ip_networks() {
            if let std::net::IpAddr::V4(ipv4) = net.addr {
                if !ipv4.is_loopback() && net.prefix > 0 && net.prefix < 32 {
                    let ip_u32 = u32::from(ipv4);
                    let mask = !((1u64 << (32 - net.prefix)) - 1) as u32;
                    let bcast_u32 = ip_u32 | !mask;
                    let bcast = std::net::Ipv4Addr::from(bcast_u32);
                    let target = format!("{}:{}", bcast, DEFAULT_DISCOVERY_PORT);
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
            }
        }
    }

    targets
}

static GLOBAL_DISCOVERY_MANAGER: OnceLock<DiscoveryManager> = OnceLock::new();

pub fn get_discovery_manager() -> &'static DiscoveryManager {
    GLOBAL_DISCOVERY_MANAGER.get_or_init(DiscoveryManager::new)
}
