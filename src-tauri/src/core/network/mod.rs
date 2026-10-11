pub mod discovery;
pub mod service;
pub mod transport;

pub use discovery::{
    get_discovery_manager, DiscoveredPeer, DiscoveryManager, LanBeacon, DEFAULT_DISCOVERY_PORT,
    DISCOVERY_BEACON_MAGIC, DISCOVERY_PROTOCOL_VERSION,
};
pub use service::{get_network_service, NetworkService};
pub use transport::{
    derive_session_key, get_connection_manager, ConnectionManager, EncryptedTransportFrame,
    NetworkWireMessage, PeerConnectionStatus, TransportFrame, TransportFramePayload,
    MAX_FRAME_SIZE, TRANSPORT_PROTOCOL_VERSION,
};
