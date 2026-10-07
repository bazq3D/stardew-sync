# Stardew Valley Co-op Save Sync / Host Switcher Architecture

## 1. System Overview

`stardew-sync` is a lightweight, local-first Windows desktop application and serverless synchronization service designed to let trusted players share an existing Stardew Valley multiplayer farm with seamless host switching, versioned cloud backup, and conflict protection.

```
┌────────────────────────────────────────────────────────┐
│                     Desktop Client                     │
│  ┌───────────────────────┐   ┌──────────────────────┐  │
│  │   React + TypeScript  │   │      Rust Core       │  │
│  │       (Tauri UI)      │◄──┤   (Tauri Backend)    │  │
│  └───────────────────────┘   └──────────┬───────────┘  │
│                                         │              │
│       ┌─────────────────────────┬───────┴────────┐     │
│       ▼                         ▼                ▼     │
│  Local SQLite           Stardew Saves      Local Backup│
│  (Config & Cache)       (%APPDATA%)        (%LOCALAPP%)│
└─────────────────────────────────┬──────────────────────┘
                                  │ HTTPS (Signed Tokens)
                                  ▼
┌────────────────────────────────────────────────────────┐
│                   Cloudflare Backend                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │             Cloudflare Worker (API)              │  │
│  └──────────────┬────────────────────────────┬──────┘  │
│                 ▼                            ▼         │
│          Cloudflare D1                 Cloudflare R2   │
│       (Metadata, Versions,          (Encrypted Save    │
│        Presence, Invites)               Archives)      │
└────────────────────────────────────────────────────────┘
```

---

## 2. Desktop Client Architecture

### 2.1 Technology Stack
- **Framework:** Tauri v2
- **Frontend:** React 18, TypeScript, TailwindCSS / CSS tokens for sleek dark theme, Lucide icons.
- **Backend (Native):** Rust
  - XML Parser: `quick-xml` (fast, streaming, strict validation)
  - Process Monitoring: `sysinfo` / Windows API
  - Compression: `zip` / `flate2`
  - Cryptography: `ring` / `aes-gcm` (AES-256-GCM) / `ed25519-dalek`
  - Local Database: `rusqlite`
  - HTTP Client: `reqwest`

### 2.2 Core Rust Modules
```
src-tauri/src/
├── core/
│   ├── save/
│   │   ├── discovery.rs     # Discovers Stardew saves in %APPDATA%
│   │   ├── parser.rs        # Parses SaveGame XML and extracts metadata
│   │   ├── validator.rs     # Validates XML completeness and player integrity
│   │   └── host_migrator.rs # Executes safe, bidirectional host conversions
│   ├── backup/
│   │   ├── manager.rs       # Creates, verifies, restores snapshots
│   │   └── retention.rs     # Enforces retention policy (protecting Original Import)
│   ├── process/
│   │   └── monitor.rs       # Detects Stardew/SMAPI running, stopped, settle timing
│   ├── sync/
│   │   ├── engine.rs        # Sync state machine (pull, push, conflict)
│   │   └── crypto.rs        # Client-side AES-256-GCM encryption
│   └── db/
│       └── schema.rs        # Local SQLite migrations and models
├── api/
│   └── client.rs            # Authenticated Worker API client
└── commands.rs              # Tauri IPC commands exposed to React
```

---

## 3. Cloud Backend Architecture

The backend runs entirely on Cloudflare serverless edge infrastructure. No long-running servers or Node.js daemons are required.

### 3.1 Components
1. **Cloudflare Worker:**
   - Stateless TypeScript Worker handling REST API routes.
   - Validates device credentials and farm membership tokens.
   - Generates presigned R2 upload/download URLs or streams objects through verified endpoints.
   - Rate limiting and input validation on all payloads.
2. **Cloudflare D1 (SQL Database):**
   - Relational metadata store for farms, paired devices, version history, invites, and presence.
3. **Cloudflare R2 (Object Storage):**
   - Stores encrypted save archive packages (`.sdz`).
   - Objects are keyed by `farms/{farm_id}/versions/{version_id}.sdz`.

---

## 4. Database & Storage Schemas

### 4.1 Cloudflare D1 Schema

```sql
-- Shared Farms
CREATE TABLE farms (
    id TEXT PRIMARY KEY,                       -- UUIDv4 or Nanoid (e.g. farm_c8f192...)
    name TEXT NOT NULL,                        -- Farm name (e.g. "Willow Farm")
    created_at INTEGER NOT NULL,               -- Unix timestamp ms
    owner_device_id TEXT NOT NULL,             -- Device ID of creator
    current_version_id TEXT,                   -- Pointer to current canonical version
    encryption_test_hash TEXT NOT NULL         -- Known ciphertext check for key verification
);

-- Paired Devices / Members
CREATE TABLE devices (
    device_id TEXT PRIMARY KEY,                -- UUIDv4 generated on client install
    display_name TEXT NOT NULL,                -- User display name (e.g. "Kubilay", "Elbi")
    public_key TEXT NOT NULL,                  -- Ed25519 public key (hex)
    created_at INTEGER NOT NULL
);

CREATE TABLE farm_members (
    farm_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    player_name TEXT NOT NULL,                 -- Linked in-game farmer name
    unique_multiplayer_id TEXT,                -- Stardew UniqueMultiplayerID
    role TEXT NOT NULL CHECK(role IN ('OWNER', 'MEMBER')),
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (farm_id, device_id),
    FOREIGN KEY (farm_id) REFERENCES farms(id) ON DELETE CASCADE,
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

-- Version History (Append-Only)
CREATE TABLE versions (
    id TEXT PRIMARY KEY,                       -- UUIDv4
    farm_id TEXT NOT NULL,
    version_number INTEGER NOT NULL,           -- Monotonic integer: 1, 2, 3...
    parent_version_id TEXT,                    -- NULL for v1; references previous version
    sha256_hash TEXT NOT NULL,                 -- SHA-256 of encrypted archive
    plaintext_save_hash TEXT NOT NULL,         -- SHA-256 of decrypted primary XML file
    active_host_player TEXT NOT NULL,          -- Who was host when this save was captured
    in_game_season TEXT,                       -- "spring", "summer", "fall", "winter"
    in_game_day INTEGER,                       -- 1 - 28
    in_game_year INTEGER,                      -- 1, 2, 3...
    uploader_device_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    reason TEXT NOT NULL,                      -- INITIAL_IMPORT, SESSION_COMPLETE, HOST_SWITCH, RESTORE, CONFLICT_RESOLUTION
    r2_object_key TEXT NOT NULL,
    FOREIGN KEY (farm_id) REFERENCES farms(id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_device_id) REFERENCES devices(device_id)
);

CREATE INDEX idx_versions_farm_number ON versions(farm_id, version_number DESC);

-- Pending Invites / Pairing Codes
CREATE TABLE invites (
    invite_code TEXT PRIMARY KEY,              -- Format: "SDV-XXXX-XXXX" (high entropy token)
    farm_id TEXT NOT NULL,
    creator_device_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'MEMBER',
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,               -- e.g. 48 hours
    used_at INTEGER,
    used_by_device_id TEXT,
    FOREIGN KEY (farm_id) REFERENCES farms(id) ON DELETE CASCADE
);

-- Ephemeral Player Presence (Heartbeats)
CREATE TABLE presence (
    farm_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('ONLINE', 'PLAYING', 'SYNCING')),
    last_heartbeat INTEGER NOT NULL,           -- Unix timestamp ms
    PRIMARY KEY (farm_id, device_id),
    FOREIGN KEY (farm_id) REFERENCES farms(id) ON DELETE CASCADE
);
```

### 4.2 Local SQLite Schema (`%LOCALAPPDATA%\stardew-sync\data.db`)

```sql
CREATE TABLE local_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE local_farms (
    farm_id TEXT PRIMARY KEY,
    farm_name TEXT NOT NULL,
    game_id TEXT NOT NULL,                     -- Stardew numeric ID
    save_folder_name TEXT NOT NULL,            -- e.g. "WillowFarm_12345678"
    farm_encryption_key TEXT NOT NULL,         -- Base64 256-bit AES key
    local_role TEXT NOT NULL,
    current_synced_version_id TEXT,
    last_known_hash TEXT,
    is_active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE local_backups (
    id TEXT PRIMARY KEY,                       -- Snapshot UUID
    farm_id TEXT NOT NULL,
    backup_type TEXT NOT NULL,                 -- ORIGINAL_IMPORT, PRE_SYNC, PRE_HOST_SWITCH, POST_SESSION, MANUAL
    version_id TEXT,
    created_at INTEGER NOT NULL,
    archive_path TEXT NOT NULL,
    sha256_hash TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    is_protected INTEGER NOT NULL DEFAULT 0    -- 1 = Cannot be auto-purged
);
```

---

## 5. Security & Pairing Model

### 5.1 Threat Model
- **Untrusted Clients:** The desktop client cannot hold master Cloudflare API keys or R2 admin credentials.
- **Untrusted Cloud:** S3/R2 storage holds personal game save files. S3 buckets could theoretically be breached or misconfigured. Therefore, save archives are encrypted client-side using authenticated encryption (AES-256-GCM) with a farm-specific key before leaving the device.
- **Eavesdropping on Invites:** A simple 6-letter invite code alone has low entropy. The pairing protocol combines a human-readable invite code with a high-entropy key exchange.

### 5.2 Key Architecture & Client-Side Encryption
1. **Farm Encryption Key ($K_{farm}$):** A 256-bit cryptographically secure random key generated via OS CSPRNG when a farm is first created/shared.
2. **Device Identity:** Each device creates an Ed25519 keypair on first launch.
3. **Save Encryption:**
   - Archives are compressed into a `.zip` stream.
   - Encrypted with AES-256-GCM using $K_{farm}$ and a random 96-bit nonce.
   - Nonce and authentication tag are prefixed to the ciphertext payload.
4. **Emergency Recovery Guarantee:** Local snapshots stored in `%LOCALAPPDATA%\stardew-sync\backups\` are stored unencrypted (or with clear extraction instructions) so that in an extreme emergency, a user can copy the save folder straight into `%APPDATA%\StardewValley\Saves\` without needing the application or any cryptographic keys.

### 5.3 Pairing Flow

```
   Host PC (Kubilay)                           Server (Worker)                     Client PC (Elbi)
          │                                           │                                   │
  1. Generate K_farm                                  │                                   │
  2. Request Invite Code ────────────────────────────►│                                   │
     (POST /farms/:id/invites)                        │                                   │
     ◄────────────────────────────────── Return Invite Token (Code)                       │
  3. Form Combined Invite Payload:                    │                                   │
     "SDV-9A4F-28B1#<Base64(K_farm)>"                │                                   │
     Displayed as clickable/copyable package          │                                   │
          │                                           │                                   │
          │ ─── Transferred securely (Discord / Signal / Local) ─────────────────────────►│
          │                                           │                                   │
          │                                           │   4. Paste invite package         │
          │                                           │   5. Parse Code + K_farm          │
          │                                           │   6. Register Device + Join Farm ─┼─►
          │                                           │      (POST /invites/claim)        │
          │                                           │◄──────────────────────────────────│
          │                                           │   7. Validate Token               │
          │                                           │   8. Return Farm Metadata ────────┼─►
```

---

## 6. Host Switching Engine

### 6.1 Bidirectional Transformation Algorithm

Host switching transforms the save file so that either player can open the farm as the primary host while preserving full player state.

```
       [Canonical Save State: Host = A, Cabin = B]
                            │
               Do we want to play as B?
                     ├── YES ──► Execute Host Switch (A ⇄ B)
                     └── NO  ──► Launch directly with A
                            │
       [Transformed Save State: Host = B, Cabin = A]
```

### 6.2 Step-by-Step Execution Sequence

1. **Safety Snapshot:** Take a verified backup of the current save before touching any XML.
2. **XML Parse & Tree Validation:** Parse `<SaveGame>` into DOM.
   - Locate root `<player>` (Farmer A).
   - Find cabin containing target player (Farmer B) in `<locations> -> Farm -> buildings -> Cabin -> indoors -> farmhand`.
   - Assert both players match expected `UniqueMultiplayerID` and `name`.
3. **Capture Residence Attributes:**
   - FarmHouse attributes: `homeLocation = "FarmHouse"`, `houseUpgradeLevel = A.houseUpgradeLevel`.
   - Cabin attributes: `homeLocation = B.homeLocation`, `houseUpgradeLevel = B.houseUpgradeLevel`.
4. **Swap Nodes:**
   - Detach Farmer A and Farmer B subtrees.
   - Farmer B is updated: `homeLocation = "FarmHouse"`, `houseUpgradeLevel = Farmhouse.upgradeLevel`.
   - Farmer A is updated: `homeLocation = Cabin.homeLocation`, `houseUpgradeLevel = Cabin.upgradeLevel`.
   - Insert Farmer B into `<SaveGame><player>`.
   - Insert Farmer A into `<indoors xsi:type="Cabin"><farmhand>`.
5. **Update `SaveGameInfo`:**
   - Build new `SaveGameInfo` XML root `<Farmer>` matching Farmer B.
6. **Integrity Checks:**
   - Ensure well-formed XML.
   - Ensure Farmer A and Farmer B both exist with identical inventories and UniqueMultiplayerIDs.
   - Ensure root `<player><name>` is Farmer B.
7. **Atomic Replace:** Atomically stage and swap the modified files into the live save directory.

---

## 7. Synchronization & Versioning Model

### 7.1 Monotonic Versions with Graph Lineage

Versions are strictly monotonic integers with parent references:
- $V_1$: Initial Import (Parent: `null`)
- $V_2$: Session by Kubilay (Parent: $V_1$)
- $V_3$: Session by Elbi (Parent: $V_2$)

### 7.2 Conflict Detection (Branching)

If two clients start from the same parent version and both produce new saves:
```
           ┌──► V_4A (Kubilay, 19:30)
V_3 ───────┤
           └──► V_4B (Elbi, 20:00)
```
- Server rejects automated fast-forward when `parent_version_id != current_version_id`.
- Server records both versions in D1.
- Both clients receive a `CONFLICT_DETECTED` state.
- **Zero Data Loss:** Both versions exist in R2 and in local backups. The UI prompts the user to select which branch becomes canonical ($V_5$ with `reason: CONFLICT_RESOLUTION`), keeping the rejected branch accessible in the history list.

---

## 8. Presence & Advisory Locking

To prevent accidental conflicts without causing hard lockouts when a PC crashes:
- **Soft Advisory Presence:**
  - Client sends a heartbeat every 30 seconds (`status: 'PLAYING' | 'ONLINE'`).
  - Worker automatically marks sessions `OFFLINE` if no heartbeat is received for 90 seconds.
- **Launch Warning:**
  - If Kubilay clicks "PLAY" while Elbi's status is `PLAYING`, the client displays:
    > "Elbi appears to be playing Stardew Valley right now. Starting from this save may create conflicting progress."
  - Options: `[ Cancel ]` or `[ Play Anyway ]`.

---

## 9. Implementation Roadmap

- **Phase 0:** Research & Validation (`docs/research.md`)
- **Phase 1:** Architectural Blueprint (`docs/architecture.md`)
- **Phase 2:** Save Safety & Invariants Specification (`docs/save-safety.md`)
- **Phase 3:** Desktop Core Prototype (Rust save discovery, parsing, backup manager, process monitor)
- **Phase 4:** Host Switching Engine & Automated Test Suite (Isolated fixture roundtrips: $A \to B \to A$)
- **Phase 5:** Cloudflare Backend Implementation (Worker API, D1 schema, R2 integration)
- **Phase 6:** End-to-End Synchronization & Conflict Engine
- **Phase 7:** Presence & Advisory Warnings
- **Phase 8:** User Interface (Modern React UI, status cards, backup timeline)
- **Phase 9:** Packaging & Verification (Windows executable/installer generation)
