# Stardew Sync v0.2.0 — Two-PC Pairing & P2P Transport Specification

**Document Version:** 1.0.0  
**Author:** bazq  
**Target Milestone:** v0.2.0  
**Status:** TECHNICAL ARCHITECTURE SPECIFICATION

---

## 1. Transport Feasibility & Reality Check

Establishing a direct peer-to-peer connection between two home Windows PCs over the internet is notoriously difficult due to:
- **Carrier-Grade NAT (CGNAT):** Widespread among modern residential ISPs, cellular hotspots, and fiber providers, rendering incoming port forwarding impossible.
- **Symmetric NATs:** Router implementations where port mapping changes with each external destination, causing standard STUN hole punching to fail in ~15–25% of connections.
- **Windows Firewall:** Blocks unsolicited incoming TCP/UDP connections by default unless explicit inbound rules are configured.

### Transport Strategy Comparison

| Transport Architecture | Feasibility | NAT Traversal Success | Zero Third-Party Plaintext | Dependency Weight | Verdict |
|---|---|---|---|---|---|
| **Direct IP / Manual Port Forwarding** | High | Fails behind NAT/CGNAT | Yes | Low | **Unacceptable UX** for regular users. Requires router config. |
| **Pure LAN mDNS / UDP Beacon** | High | 100% on same subnet | Yes | Low | **Approved for LAN**, but cannot sync when players are in different homes. |
| **Pure WebRTC DataChannels (`webrtc-rs`)** | Medium | ~85% (Fails on Symmetric NAT without TURN) | Yes (DTLS) | Extremely High (Large C/Rust bindings) | High maintenance burden and requires hosted TURN servers. |
| **Iroh / QUIC (`iroh`)** | Medium | ~95% (Built-in DERP relay) | Yes | Medium | Excellent, but evolving API; high churn for desktop app. |
| **Hybrid LAN + E2EE WebSocket Relay** | **Very High** | **100% across all network topologies** | **Absolute (E2EE AES-256-GCM)** | **Minimal** | **RECOMMENDED & SELECTED.** Direct LAN when available, seamless E2EE relay fallback. |

### The Selected Hybrid Transport Architecture

1. **LAN Mode (Primary on Same Network):**
   - Discovers paired peers on the local subnet via **Multicast DNS (mDNS)** or authenticated UDP broadcast on port `42890`.
   - Directly transfers encrypted archives over local TLS/TCP. Maximum speed (Gbps), zero internet bandwidth used.
2. **E2EE Relay Fallback (Cross-Network / Internet):**
   - When peers are on different networks, they connect to a lightweight serverless rendezvous relay (e.g. Cloudflare Worker WebSocket or minimal Rust relay).
   - **Zero Knowledge Guarantee:** The relay **only** forwards opaque encrypted blobs (`AES-256-GCM`). Keys are derived locally on the two PCs using `X25519` Diffie-Hellman. The relay has zero cryptographic capability to decrypt save data, inspect character names, or view farm contents.

---

## 2. Cryptographic Identity Model

On first initialization, Stardew Sync generates an immutable device identity stored in `%LOCALAPPDATA%\StardewSync\identity.json`:

```json
{
  "device_id": "7c8e3124-b15c-482a-9f5b-59d436c645bc",
  "friendly_name": "Kubilay-Desktop",
  "created_at": "2026-10-10T12:00:00Z",
  "signing_public_key": "MCowBQYDK2VwAyEAXv... (Ed25519)",
  "exchange_public_key": "MCowBQYDK2VuAyEA9k... (X25519)"
}
```

- **Identity Keypair (Ed25519):** Used to sign pairing invitations, sync proposals, and revocation notices. Fingerprint is `SHA-256(Ed25519_Pub)`.
- **Exchange Keypair (X25519):** Used for ephemeral Elliptic-Curve Diffie-Hellman (ECDH) key exchange to establish end-to-end encryption keys.
- **Private Key Storage:** Private keys are stored with Windows Data Protection API (DPAPI) or restricted user-only filesystem ACLs. They never leave the host machine.

---

## 3. Mutual Pairing Protocol

Pairing permanently links two specific computers for a designated farm without central user accounts:

```
Device A (Owner / Initiator)                   Device B (Guest / Joiner)
         │                                              │
         │─── Generates Code: "SD-8492-FK" (TTL: 10m) ─│ (Shared out-of-band)
         │                                              │
         │◄── Submits Join Request ─────────────────────│
         │    Includes: DeviceB_UUID, FriendlyName,     │
         │    Ed25519_Pub, X25519_Pub, Signature        │
         │                                              │
         │─── UI Modal Prompt on Device A ─────────────│
         │    "Allow 'Laptop-Elbi' to pair with        │
         │     'Türk Farm'?"                            │
         │                                              │
         │─── Owner Approves ──────────────────────────►│
         │    Sends Signed Pairing Certificate          │
         │                                              │
         ▼                                              ▼
    [PAIRED STATE SAVED]                          [PAIRED STATE SAVED]
```

### Protocol Steps:
1. **Initiation:** Owner selects a farm and clicks "Pair New Device". App displays a 6-character alphanumeric code (`SD-XXXX-XX`) valid for 10 minutes.
2. **Discovery & Request:** The guest enters the code. Guest client transmits its public identity keys and signature to the owner.
3. **Owner Authorization:** Owner PC displays an interactive confirmation dialog with the guest's friendly name and cryptographic fingerprint.
4. **Certificate Issuance:** Upon approval, Device A signs a `PairingCertificate` containing:
   - `certificate_id`, `farm_id`, `guest_device_id`, `permissions: ["read", "propose_sync"]`, `issued_at`.
5. **Key Derivation:** Both devices derive a mutual symmetric encryption key using:
   $$\text{SharedSecret} = \text{X25519}(\text{Private}_A, \text{Public}_B) = \text{X25519}(\text{Private}_B, \text{Public}_A)$$
   $$\text{SessionKey} = \text{HKDF-SHA256}(\text{SharedSecret}, \text{Salt}, \text{"stardew-sync-v0.2.0"})$$

---

## 4. Transfer Integrity, Chunking & Replay Protection

To ensure corrupt or incomplete save files never touch disk:

1. **Monotonic Generation Envelope:**
   - Every sync proposal carries an incremental `generation_number`, `parent_generation_hash`, and the signer's Ed25519 signature.
   - Out-of-order or duplicate generation numbers are rejected to prevent replay attacks.
2. **Chunked Streaming (64 KiB chunks):**
   - Save archives are compressed (`zip` / `flate2`) and broken into 64 KiB encrypted frames.
   - Each frame includes: `[frame_index, payload_bytes, aes_gcm_tag, sha256_chunk_hash]`.
   - If a transfer is interrupted by network drop, the receiving client resumes from the last confirmed frame rather than restarting from scratch.
3. **End-to-End Validation:**
   - After reassembly in the recipient's isolated staging area, the package hash must match the signed generation manifest exactly before transactional staging begins.

---

## 5. Device Revocation

- Farm owners can revoke any paired device from **Settings > Paired Devices**.
- Revocation generates an Ed25519-signed `RevocationManifest`.
- The revoked device's public key is added to the local blacklist and broadcasted.
- Any future connection attempts or sync proposals from that device ID are immediately rejected.

---

## 6. Phase 5A Implementation Architecture

### 6.1 Threat Model
- **Adversary Capabilities:**
  - *Passive LAN Eavesdropper:* Can inspect unencrypted UDP broadcast beacons and local TCP frames on the local subnet.
  - *Active Network Attacker / MITM:* Can inject, alter, or replay packets on the local network or between peer endpoints.
  - *Malicious / Compromised Peer on LAN:* Can attempt unsolicited pairing, spoof host identity, or send unauthorized RPC frames.
  - *Local Unprivileged User/Process:* Can attempt to read key material if saved in plaintext files or browser storage.
- **Defenses & Guarantees:**
  - *Confidentiality & Storage:* Private keys never leave the machine. Stored protected via Windows DPAPI (`CryptProtectData`/`CryptUnprotectData` with `CRYPTPROTECT_UI_FORBIDDEN`). No plaintext private keys in JSON, localStorage, logs, or UI.
  - *MITM Resistance:* Dual out-of-band confirmation via a 6-digit Short Authentication String (SAS) computed as `HMAC-SHA256(Transcript, "StardewSync-SAS-v1")[0..3]`. Both users must visually verify matching codes before trust is committed.
  - *Replay & Stale Protection:* Ephemeral random nonces in every handshake; 5-minute single-use invitation expiry; incremental frame sequence numbers.
  - *Strict Isolation:* Zero save transfers, zero save writes, zero snapshot payloads, zero remote execution commands. Frame types restricted strictly to identity, pairing, heartbeat, and status.

### 6.2 Device Identity Model
- **Ed25519 Keypair:** Generated on first run using OS CSPRNG (`rand::rngs::OsRng`).
- **Device ID:** SHA-256 fingerprint of public key: hex formatted `SHA-256(Ed25519_Pub)`.
- **Public Fingerprint:** Human-friendly display string formatted in grouped hex (e.g. `a1b2-c3d4-e5f6-7890`).
- **Device Name:** Hostname or user-customized label, editable in UI.
- **Fail-Closed on Corruption:** If encrypted identity cannot be decrypted or is missing, explicitly report corruption/loss; never silently generate a new key and overwrite without warning.

### 6.3 Pairing Authentication Flow
1. **Device A (Inviter):** User starts pairing. Backend generates ephemeral 6-char invitation code (`SD-XXXX`) and 32-byte nonce, starts listening on TCP port 42890, sets 5-minute expiry.
2. **Device B (Joiner):** Discovers Device A via LAN beacon or manual IP entry. Connects and transmits `PairingRequest` signed with Device B's Ed25519 key, including Device B's ephemeral X25519 public key and invitation code.
3. **Handshake & SAS Derivation:** Device A validates invitation code, signature, and timestamp. Computes mutual transcript and derives 6-digit numeric SAS code. Device A replies with signed `PairingResponse` with its own ephemeral X25519 key. Device B verifies and derives the identical 6-digit SAS code.
4. **Dual Confirmation:** Both screens display the 6-digit SAS code. Both users click "Confirm". Only when both sides receive mutual cryptographic confirmation does each persist the other's public key in `%LOCALAPPDATA%\StardewSync\devices\paired_devices.json`.

### 6.4 Local Network Discovery Approach
- UDP broadcast beacon on port 42890 every 3 seconds.
- Payload contains only: `magic`, `protocol_version`, `device_id`, `device_name`, `tcp_port`, `is_inviting`.
- Zero sensitive data (no save names, farm metadata, or private keys).
- Manual IP entry (`host:port`) available for networks with broadcast filtering or firewall restrictions.
- Discovered devices are pruned after 15 seconds of inactivity.

### 6.5 Secret Storage Approach
- Windows DPAPI encrypts Ed25519 private key to `%LOCALAPPDATA%\StardewSync\identity\device_identity.enc`.
- Public metadata saved to `%LOCALAPPDATA%\StardewSync\identity\device_identity.json`.
- Paired peer store saved to `%LOCALAPPDATA%\StardewSync\devices\paired_devices.json` (public keys only).

### 6.6 Connection Lifecycle
- **States:** `Disconnected` -> `Connecting` -> `Connected` (Authenticated) -> `Error`.
- Only devices with an active, authenticated cryptographic session are marked `Connected`. Discovered unpaired devices are clearly labeled as `Discovered (Unpaired)`.
