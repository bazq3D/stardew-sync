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

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            // Setup Native System Tray
            core::tray::setup_system_tray(&app.handle())?;

            // Check if application should start minimized
            let args: Vec<String> = std::env::args().collect();
            let has_minimized_arg = args.iter().any(|arg| arg == "--minimized");
            let settings = core::settings::SettingsManager::get_cached();
            if has_minimized_arg || settings.start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Spawn background periodic update poller
            core::updater::BackgroundUpdatePoller::spawn_periodic_poller(app.handle().clone());

            // Initialize and start background save monitoring engine
            let monitoring_service = commands::get_monitoring_service();
            monitoring_service.start_background_loop();

            // Initialize device identity and start local network services
            if let Ok(identity) = core::identity::DeviceIdentity::load_or_create() {
                let net = core::network::get_network_service();
                net.start_tcp_listener(core::network::DEFAULT_DISCOVERY_PORT, identity.clone());
                net.start_discovery_listener(identity.device_id());
                net.start_discovery_announcer(identity.clone());
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let settings = core::settings::SettingsManager::get_cached();
                if settings.minimize_to_tray_on_close {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
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
            commands::get_app_settings,
            commands::save_app_settings,
            commands::get_autostart_status,
            commands::set_autostart_status,
            commands::trigger_background_update_check,
            commands::get_background_update_status,
            commands::get_farm_characters,
            commands::get_all_characters,
            commands::create_manual_snapshot,
            commands::list_versioned_snapshots,
            commands::get_versioned_snapshot_manifest,
            commands::verify_versioned_snapshot,
            commands::rename_versioned_snapshot_label,
            commands::delete_versioned_snapshot,
            commands::compare_versioned_snapshots,
            commands::get_farm_monitoring_status,
            commands::trigger_monitoring_check,
            commands::get_farm_observation_journal,
            commands::get_all_monitoring_statuses,
            commands::set_monitoring_enabled,
            commands::get_local_device_info,
            commands::set_local_device_name,
            commands::list_paired_devices,
            commands::unpair_device,
            commands::start_pairing_invitation,
            commands::cancel_pairing_invitation,
            commands::get_active_pairing_session,
            commands::confirm_pairing_code,
            commands::reject_pairing,
            commands::connect_to_peer,
            commands::get_discovered_peers,
            commands::broadcast_discovery_beacon,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

