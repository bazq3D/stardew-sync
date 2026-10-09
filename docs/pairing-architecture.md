# STARDEW SYNC — DEVICE PAIRING & P2P TRANSPORT ARCHITECTURE (PHASE 6.0 DESIGN)

**Author:** bazq  
**Project:** `stardew-sync-p2p`  
**Status:** ARCHITECTURAL SPECIFICATION (DESIGN ONLY)  
**Target:** Two or More Arbitrary Windows PCs  

---

## 1. Executive Summary & Design Goals

This document specifies the device pairing, mutual authentication, cryptographic transport, and synchronization protocol for **Stardew Sync**.

### Fundamental Principles:
1. **Zero Hardcoded Identities:** The system functions for any arbitrary player accounts, farm names, or machine configurations.
2. **Explicit Consent & Mutual Approval:** No device can push, pull, or inspect saves without explicit cryptographic authorization by the farm owner.
3. **Default-Deny Filesystem Safety:** Live saves in `%APPDATA%\StardewValley\Saves` are never overwritten in-place without transactional staging, integrity validation, and settle detection.
4. **End-to-End Encryption:** All save archives and metadata are encrypted with AES-256-GCM using ephemeral session keys derived via X25519 key exchange.
5. **Xbox WGS Resilience:** Designed specifically around Microsoft Store / Xbox Connected Storage synchronization characteristics (Strategy F isolation).

---

## 2. Cryptographic Identity & Trust Model

Each installation of Stardew Sync generates an immutable device identity upon first launch, stored in `%LOCALAPPDATA%\StardewSync\identity.json`:

```
Device Identity:
├── Device UUID (UUIDv4)
├── Device Friendly Name (e.g. "Desktop-PC", "Laptop")
├── Identity Keypair (Ed25519) - for signing sync manifests and pairing approvals
└── Exchange Keypair (X25519) - for Diffie-Hellman key agreement
```

### Key Security Properties:
- The private signing and exchange keys NEVER leave the local machine.
- Device fingerprints are derived from `SHA-256(Ed25519_PublicKey)`.
- Re-installing the application or revoking a device invalidates old authorizations.

---

## 3. Device Pairing Protocol

Pairing links two devices (e.g. Host Device A and Client Device B) without requiring third-party accounts or centralized servers.

```
Device A (Owner / Initiator)                   Device B (Guest / Joiner)
         │                                              │
         │─── Generates 6-character Code (TTL: 10m) ───│
         │    (or QR Code / Direct LAN Beacon)          │
         │                                              │
         │◄── Enters Code / Sends Join Request ─────────│
         │    Includes: DeviceB_UUID, Ed25519_Pub,      │
         │    X25519_Pub, Timestamp, Signature          │
         │                                              │
         │─── UI Modal Prompt on Device A ─────────────│
         │    "Allow [Device B] to access [Farm X]?"    │
         │                                              │
         │─── Owner Approves ──────────────────────────►│
         │    Sends Signed Pairing Certificate          │
         │    Includes: FarmID, AllowedRoles, Expiry    │
         │                                              │
         ▼                                              ▼
   [State: PAIRED]                               [State: PAIRED]
```

### Step 1: Pairing Invitation Generation
1. Owner clicks **"Pair New Device"** in Stardew Sync.
2. App generates a short-lived, high-entropy pairing code (e.g., `SD-8492-FK`).
3. App spins up an authenticated local rendezvous listener (or signals via encrypted relay).
4. Code expires automatically after **10 minutes**.

### Step 2: Join Request & Mutual Key Exchange
1. Guest enters the pairing code on their machine.
2. Both devices execute an X25519 ECDH exchange to establish an ephemeral shared secret:
   $$\text{Secret} = \text{X25519}(\text{Private}_A, \text{Public}_B) = \text{X25519}(\text{Private}_B, \text{Public}_A)$$
3. Key derivation (HKDF-SHA256) derives:
   - `AES-256-GCM` transport encryption key.
   - `HMAC-SHA256` integrity and message verification key.

### Step 3: Explicit Owner Approval & Certificate Issuance
1. Device A displays a confirmation dialog:
   - Requesting Device Name: `LivingRoom-PC`
   - Public Key Fingerprint: `8F:3A:91:...`
   - Selected Farm: `Meadow Farm (Game ID: 948192019)`
2. Once approved, Device A signs a **Pairing Certificate** containing:
   - `farm_id`, `guest_device_id`, `permissions: [Read, ProposeSync]`, `issued_at`.
3. Device B stores the certificate. Both devices now recognize each other as trusted peers.

---

## 4. Farm Selection & Dynamic Membership

- **Multi-Farm Support:** Users can pair different devices for different farms.
- **Dynamic Player Mapping:** The application inspects the actual `SaveGame` XML:
  - Host player: read from root `<player>`.
  - Farmhands: read from `<farmhands>` and `<locations>` cabins.
- **Role Assignment:** Roles (who plays as host, who plays as farmhand) are assigned dynamically by selecting character identities from the discovered save, with zero hardcoding.

---

## 5. Sync Generation Tracking & Conflict Detection

To prevent save rollbacks or out-of-order writes, Stardew Sync implements a **Monotonic Generation Chain**:

### Generation Manifest (`sync_manifest.json`):
```json
{
  "farm_id": "Meadow_948192019",
  "generation_number": 42,
  "parent_generation_hash": "A8F902BC...31",
  "current_generation_hash": "D91E44A1...7B",
  "timestamp": "2026-10-12T14:30:00Z",
  "last_saved_by_device": "Desktop-Kubilay",
  "game_calendar": "Fall, Day 24 (Year 1)",
  "game_funds": 18450,
  "files": {
    "SaveGame": "SHA256_HASH_HERE",
    "SaveGameInfo": "SHA256_HASH_HERE"
  },
  "signature": "ED25519_SIGNATURE_BY_SAVER"
}
```

### Conflict Scenarios & Detection Matrix:

| Scenario | Local State | Incoming State | Action |
| :--- | :--- | :--- | :--- |
| **Clean Forward Sync** | Gen 41 | Gen 42 (Parent = Gen 41) | Fast-forward sync allowed. |
| **Already Up-to-Date** | Gen 42 | Gen 42 (Identical hash) | No action needed. |
| **Stale State Rejected** | Gen 42 | Gen 41 | **Rejected immediately.** Stale generations cannot overwrite newer progress. |
| **Divergent Fork (Conflict)** | Gen 42 (Hash X) | Gen 42 (Hash Y) | **Conflict Blocked.** User is prompted to inspect both dates/funds and choose which generation to branch or preserve. |

---

## 6. Safe Transactional Staging & Application

Live saves are protected by strict transactional guarantees:

1. **Pre-Flight Lock Check:** Ensure `Stardew Valley.exe` and `StardewModdingAPI.exe` are completely dormant via `SystemProcessChecker`.
2. **Pre-Sync Snapshot:** Create an immutable baseline snapshot in `%LOCALAPPDATA%\StardewSync\snapshots` before touching any file.
3. **Isolated Staging Area:** Unpack and decrypt the incoming save archive into an isolated temporary folder: `%LOCALAPPDATA%\StardewSync\staging\{txn_id}`.
4. **Validation Pipeline:**
   - XML well-formedness validation.
   - Farmer integrity & unique ID validation.
   - Settle detector verification.
5. **Atomic Swap:** Replace files transactionally. If anything fails, rollback instantly from the pre-sync snapshot.

---

## 7. Microsoft Store / Xbox Connected Storage (WGS) Compatibility

For players using Xbox Game Pass / Microsoft Store PC versions:
1. **Direct WGS Modification Prohibited:** The desktop app never directly writes into the obfuscated `SystemAppData\wgs` binary containers.
2. **Strategy F (Fresh Identity Migration):** When switching host roles across machines on the Microsoft Store version, the application generates a distinct slot identity (`{FarmName}_{NewGameID}`) to ensure Xbox Connected Storage captures it cleanly without rollbacks to stale cloud generations.
3. **Settle Observation:** After launching or exiting the game, the application waits for WGS index updates to settle before initiating any export or sync operations.

---

## 8. Device Revocation & Security Audit

- Any farm owner can revoke a paired device at any time from the **Settings > Paired Devices** menu.
- Revocation generates a signed `RevocationManifest` broadcast to all connected devices.
- Once revoked, all incoming requests or signatures from that device ID are permanently rejected.
- Audit logs of every sync proposal and signature verification are stored locally in `%LOCALAPPDATA%\StardewSync\logs\audit.log`.
