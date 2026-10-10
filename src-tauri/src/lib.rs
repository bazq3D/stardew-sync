pub mod commands;
pub mod core;

pub use commands::{ActiveOperationGuard, ActiveUpdateGuard};

// Re-export common symbols for library consumers
pub use core::backup::manager::BackupManager;
pub use core::backup::manifest::{BackupManifest, BackupType};
pub use core::errors::CoreError;
pub use core::process::monitor::{
    MockProcessChecker, ProcessChecker, ProcessMonitor, SaveSettleConfig, SaveSettleDetector,
    SystemProcessChecker,
};
pub use core::runtime::analyzer::{PostRuntimeAnalyzer, PostRuntimeReport};
pub use core::runtime::disposable::{
    DisposableIdentity, DisposableSaveManager, DisposableState, RuntimeBaselineManifest,
};
pub use core::runtime::guard::{ProductionGuard, PRODUCTION_FARM_FOLDER, PRODUCTION_GAME_ID};
pub use core::runtime::observer::{
    CloudObserver, DirectoryObservationSnapshot, GenerationIntegrityStatus, ObservationDelta,
};
pub use core::save::discovery::{discover_saves, DiscoveredSave};
pub use core::save::fingerprint::{
    compute_full_farmer_fingerprint, compute_migration_stable_fingerprint, verify_allowed_diff,
};
pub use core::save::host_migrator::{HostMigrator, MigrationResult};
pub use core::save::parser::{
    restore_xml_schema_instance_attributes, serialize_element, CabinSummary, ParsedSave,
    PlayerSummary, SaveMetadata,
};
pub use core::save::platform::{PlatformCapabilities, SavePlatform, SaveStorageAdapter};
pub use core::save::validator::SaveValidator;
pub use core::transaction::recovery::{RecoveryManager, RecoveryManifest, TransactionState};
pub use core::transaction::replace::SafeReplacer;
pub use core::transaction::staging::StagingArea;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::get_process_status,
            commands::discover_farms,
            commands::get_farm_metadata,
            commands::list_snapshots,
            commands::verify_snapshot_integrity,
            commands::check_for_updates,
            commands::get_update_eligibility,
            commands::acquire_update_reservation,
            commands::release_update_reservation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
