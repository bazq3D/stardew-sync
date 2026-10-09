use std::path::{Path, PathBuf};

use crate::core::errors::CoreError;
use crate::core::runtime::observer::DirectoryObservationSnapshot;
use crate::core::save::discovery::DiscoveredSave;

/// Identifies the platform edition / distribution of Stardew Valley.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SavePlatform {
    /// Standard Win32 / Steam / GOG release (direct %APPDATA%\StardewValley\Saves authority)
    SteamWin32,
    /// Microsoft Store / Xbox Game Pass for PC (Two-layer: %APPDATA% working cache + SystemAppData\wgs container backing)
    MicrosoftStoreXbox,
    /// Linux native / Snap / Flatpak (~/.config/StardewValley/Saves or snap path)
    LinuxDesktop,
    /// macOS native (~/.config/StardewValley/Saves)
    MacOSDesktop,
}

/// Characteristics and capabilities of a save platform persistence layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub platform: SavePlatform,
    /// Whether the platform has an external container/cloud sync provider (e.g. Xbox WGS / Connected Storage).
    pub has_platform_storage_manager: bool,
    /// Whether external tools can safely overwrite an existing save slot in-place without platform restore collisions.
    pub supports_in_place_slot_replacement: bool,
    /// Whether the platform provides a supported 3rd party API for save manipulation.
    pub supports_third_party_storage_api: bool,
    /// Architecture persistence classification:
    /// "single_layer_authoritative" or "two_layer_coordinated"
    pub persistence_model: &'static str,
    /// Recommended sync and migration test strategy.
    pub recommended_sync_strategy: &'static str,
}

impl SavePlatform {
    /// Returns the capabilities and safety constraints for this platform.
    pub fn capabilities(&self) -> PlatformCapabilities {
        match self {
            SavePlatform::SteamWin32 => PlatformCapabilities {
                platform: *self,
                has_platform_storage_manager: false,
                supports_in_place_slot_replacement: true,
                supports_third_party_storage_api: false,
                persistence_model: "single_layer_authoritative",
                recommended_sync_strategy: "Direct filesystem transactional replacement (%APPDATA% is authoritative)",
            },
            SavePlatform::MicrosoftStoreXbox => PlatformCapabilities {
                platform: *self,
                has_platform_storage_manager: true,
                supports_in_place_slot_replacement: false, // WGS container restores over existing slot names on launch
                supports_third_party_storage_api: false, // Connected Storage requires Title ID, SCID, and Package Identity
                persistence_model: "two_layer_coordinated",
                recommended_sync_strategy: "Two-layer coordination: Fresh disposable identity per sync session or post-settle external observation",
            },
            SavePlatform::LinuxDesktop => PlatformCapabilities {
                platform: *self,
                has_platform_storage_manager: false,
                supports_in_place_slot_replacement: true,
                supports_third_party_storage_api: false,
                persistence_model: "single_layer_authoritative",
                recommended_sync_strategy: "Direct filesystem transactional replacement",
            },
            SavePlatform::MacOSDesktop => PlatformCapabilities {
                platform: *self,
                has_platform_storage_manager: false,
                supports_in_place_slot_replacement: true,
                supports_third_party_storage_api: false,
                persistence_model: "single_layer_authoritative",
                recommended_sync_strategy: "Direct filesystem transactional replacement",
            },
        }
    }

    /// Evaluates filesystem paths to classify the installed platform.
    pub fn classify_from_paths(
        app_data_saves_exist: bool,
        xbox_packages_dir_exists: bool,
    ) -> Self {
        if xbox_packages_dir_exists {
            SavePlatform::MicrosoftStoreXbox
        } else if app_data_saves_exist {
            SavePlatform::SteamWin32
        } else {
            SavePlatform::SteamWin32
        }
    }
}

/// Abstract storage adapter trait decoupling host migration from platform-specific save persistence mechanics.
pub trait SaveStorageAdapter {
    /// Returns the active platform.
    fn platform(&self) -> SavePlatform;

    /// Discovers all available save slots on the platform.
    fn discover(&self) -> Result<Vec<DiscoveredSave>, CoreError>;

    /// Prepares a save slot for reading.
    fn prepare_for_read(&self, save_path: &Path) -> Result<(), CoreError>;

    /// Prepares a safe staging directory for writing a target slot.
    fn prepare_for_write(&self, target_slot_name: &str) -> Result<PathBuf, CoreError>;

    /// Commits the staged save files into the platform's persistence layer.
    fn commit(&self, staged_path: &Path, target_slot_name: &str) -> Result<(), CoreError>;

    /// Verifies that the platform storage holds the expected state snapshot.
    fn verify(
        &self,
        slot_name: &str,
        expected_snapshot: &DirectoryObservationSnapshot,
    ) -> Result<bool, CoreError>;

    /// Observes external changes (e.g. WGS restores, cloud downloads) to a save slot.
    fn observe_external_change(
        &self,
        slot_name: &str,
    ) -> Result<Option<DirectoryObservationSnapshot>, CoreError>;
}
