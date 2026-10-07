# Phase 3 & Phase 4 Implementation & Safety Verification Report

**Author:** bazq  
**Project:** `stardew-sync` (`stardew-sync-p2p`)  
**Scope Completed:** Phase 3 (Local Core Prototype) + Phase 4 (Host Switching Engine & Automated Safety Verification)  
**Date:** 2026-10-07  
**Build Status:** Passing (14 / 14 Automated Integration Tests)

---

## 1. Executive Summary

This report documents the completion of **Phase 3 (Local Core Prototype)** and **Phase 4 (Host Switching Engine)** for `stardew-sync`.

All engineering goals for these two phases have been accomplished:
1. **Windows Rust Development Environment Verified:** Installed and confirmed `rustc 1.99.0` (MSVC x86_64) and `cargo 1.99.0` via Scoop, functioning seamlessly with native MSVC linking.
2. **Local Core Implemented:** Process monitoring, write-settle detection, structured XML parsing, SHA-256 fingerprinting, backup creation & verification, retention management with protected import, transactional staging, atomic swapping, and crash recovery.
3. **Host Migration Engine Implemented:** Lossless, reversible host switching by `UniqueMultiplayerID`, Farmer fingerprint preservation, strict allowed-diff validation, and `SaveGameInfo` synchronization.
4. **Automated Safety Suite Verified:** 14 automated integration tests across process monitoring, safe discovery, tamper detection, transactional staging, crash recovery, host migration ($A \to B$), roundtrip reversibility ($A \to B \to A$), 30-cycle stress testing, and mod data retention.
5. **Absolute Safety Guarantee Maintained:** The user's live farm in `%APPDATA%\StardewValley\Saves` was **never accessed, scanned, read, or modified**. All operations and tests executed strictly within isolated synthetic fixtures in temporary directories.

---

## 2. Environment Verification

The Windows MSVC Rust toolchain was audited and confirmed:
- **Rust Compiler:** `rustc 1.99.0 (b940084d7 2026-09-28)`
- **Cargo:** `cargo 1.99.0 (5f94df478 2026-08-27)`
- **Target Host:** `x86_64-pc-windows-msvc`
- **Linker & SDK:** Visual C++ MSVC Linker available and verified.
- **Cargo Build Status:** Clean compilation (`dev` and `test` profiles).

---

## 3. Architecture & Implemented Modules

All core modules are organized under `src-tauri/src/core/`:

```
src-tauri/
├── Cargo.toml
├── src/
│   ├── lib.rs                       # Public API re-exports
│   └── core/
│       ├── errors.rs                # Comprehensive CoreError enum
│       ├── process/
│       │   └── monitor.rs           # ProcessMonitor (Mock & System) + SaveSettleDetector
│       ├── save/
│       │   ├── discovery.rs         # discover_saves() strictly scoped to given root
│       │   ├── parser.rs            # Lossless DOM parsing, SaveMetadata extraction
│       │   ├── fingerprint.rs       # Farmer full & migration-stable SHA-256 hashing, verify_allowed_diff
│       │   ├── validator.rs         # 4-tier validation (basic, structural, multiplayer, post-migration)
│       │   └── host_migrator.rs     # HostMigrator::migrate pipeline
│       ├── backup/
│       │   ├── manifest.rs          # BackupManifest, BackupType enum, file checksums
│       │   └── manager.rs           # Backup creation, verification, prune with protection
│       └── transaction/
│           ├── staging.rs           # Isolated StagingArea with drop cleanup
│           ├── replace.rs           # SafeReplacer (snapshot -> swap -> post-validate -> rollback)
│           └── recovery.rs          # RecoveryManager (manifest logging & crash rollback)
└── tests/
    ├── test_fixtures.rs             # Synthetic Stardew 1.6 multiplayer saves
    ├── core_safety_tests.rs         # 7 local core safety & transaction tests
    └── host_migration_tests.rs      # 7 migration, reversibility & stress tests
```

---

## 4. Test Suite Execution & Results

All 14 integration tests passed with 0 failures:

```
running 7 tests (core_safety_tests.rs)
test test_process_monitor_mock                                ... ok
test test_save_discovery_on_isolated_temp_directory           ... ok
test test_transaction_crash_recovery_restores_live_save       ... ok
test test_backup_creation_verification_and_tamper_detection   ... ok
test test_staging_area_and_safe_transactional_replacement     ... ok
test test_save_settle_detector                                ... ok
test test_original_import_protection_from_retention           ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; finished in 0.15s

running 7 tests (host_migration_tests.rs)
test test_rejects_malformed_xml                               ... ok
test test_cannot_migrate_to_current_host                      ... ok
test test_cannot_migrate_to_nonexistent_player                ... ok
test test_host_migration_a_to_b                               ... ok
test test_mod_data_and_unknown_xml_nodes_preserved             ... ok
test test_host_migration_roundtrip_a_to_b_to_a                 ... ok
test test_repeated_roundtrip_stress_test                      ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; finished in 0.15s
```

### Detailed Breakdown of Tests:
1. `test_process_monitor_mock`: Validates exact process detection for `Stardew Valley.exe` and `StardewModdingAPI.exe`, ignoring other processes.
2. `test_save_discovery_on_isolated_temp_directory`: Verifies that only complete, valid save directories are enumerated; loose files and incomplete folders are ignored.
3. `test_backup_creation_verification_and_tamper_detection`: Verifies SHA-256 manifest generation across all save files, and proves that flipping a single byte in a backup causes instant tamper detection and rejection.
4. `test_original_import_protection_from_retention`: Verifies that `OriginalImport` backups marked `is_protected = true` are immune to automated retention pruning even when retention limit is exceeded.
5. `test_staging_area_and_safe_transactional_replacement`: Tests end-to-end atomic replacement via staging, including automated pre-switch safety backups.
6. `test_save_settle_detector`: Verifies multi-observation settle detection to prevent operating on saves undergoing active disk writes.
7. `test_transaction_crash_recovery_restores_live_save`: Simulates a power failure or crash during directory swap; verifies that `RecoveryManager` cleanly restores the original save from the rollback directory.
8. `test_rejects_malformed_xml`: Asserts that corrupted/truncated XML is rejected early during parsing without touching disk.
9. `test_cannot_migrate_to_current_host`: Rejects requests attempting to migrate a host to themselves.
10. `test_cannot_migrate_to_nonexistent_player`: Rejects invalid or nonexistent player IDs.
11. `test_host_migration_a_to_b`: Validates successful host migration from PlayerA to PlayerB, checking identity, house upgrade transfer, cabin residence reassignment, and `SaveGameInfo` synchronization.
12. `test_host_migration_roundtrip_a_to_b_to_a`: Proves algebraic reversibility: migrating $A \to B \to A$ restores PlayerA's complete, unmodified state with 100% bit-for-bit fingerprint equality.
13. `test_repeated_roundtrip_stress_test`: Executes **30 consecutive host migrations** back and forth, proving zero fingerprint drift or data loss under repeated transformations.
14. `test_mod_data_and_unknown_xml_nodes_preserved`: Confirms that SMAPI/custom mod elements and `<modData>` flags survive migration and roundtrip without loss.

---

## 5. Invariant & Safety Verification Summary

| Invariant / Requirement | Verification Method | Status |
|---|---|---|
| **Zero Real-Save Access** | Code audit: all tests use `tempfile::tempdir()`; no paths reference `%APPDATA%\StardewValley\Saves` | **100% Verified** |
| **Bit-for-Bit Player Preservation** | `compute_migration_stable_fingerprint` excludes only residence fields; verified in all migrations | **100% Verified** |
| **Zero World-State Leakage** | `verify_allowed_diff` asserts identical canonical XML for all non-player nodes | **100% Verified** |
| **Reversibility ($A \to B \to A$)** | Roundtrip test confirms original full fingerprint matches post-roundtrip fingerprint | **100% Verified** |
| **Long-Term Drift Resistance** | 30-cycle migration stress test confirms 0 cumulative changes | **100% Verified** |
| **Mod / Unknown XML Safety** | DOM preservation preserves mod tags (`modData`, custom attributes) | **100% Verified** |
| **Protected Original Import** | Retention prune algorithm filters out protected backups | **100% Verified** |
| **Tamper Detection** | SHA-256 file manifest verification rejects modified backups | **100% Verified** |
| **Crash Safety** | Transactional staging + rollback directory + recovery manifest | **100% Verified** |

---

## 6. Next Steps Gate

In accordance with strict safety instructions:
- **Phase 3 & Phase 4 are fully complete.**
- **No Cloudflare, R2, D1, or remote networking was introduced.**
- **Real-save testing must NOT be attempted until explicitly authorized by the user.**
