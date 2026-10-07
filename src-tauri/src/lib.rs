pub mod core;

// Re-export common symbols for library consumers
pub use core::errors::CoreError;
pub use core::save::discovery::{discover_saves, DiscoveredSave};
pub use core::save::parser::{ParsedSave, PlayerSummary, CabinSummary, SaveMetadata};
pub use core::save::host_migrator::{HostMigrator, MigrationResult};
pub use core::save::validator::SaveValidator;
pub use core::save::fingerprint::{
    compute_full_farmer_fingerprint, compute_migration_stable_fingerprint, verify_allowed_diff,
};
pub use core::backup::manager::BackupManager;
pub use core::backup::manifest::{BackupManifest, BackupType};
pub use core::process::monitor::{
    ProcessMonitor, ProcessChecker, SystemProcessChecker, MockProcessChecker, SaveSettleDetector, SaveSettleConfig,
};
pub use core::transaction::staging::StagingArea;
pub use core::transaction::replace::SafeReplacer;
pub use core::transaction::recovery::{RecoveryManager, RecoveryManifest, TransactionState};
