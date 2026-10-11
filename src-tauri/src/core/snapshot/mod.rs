pub mod manager;
pub mod manifest;

pub use manager::VersionedSnapshotManager;
pub use manifest::{
    SnapshotComparison, SnapshotFileEntry, SnapshotIntegrityResult, SnapshotReason,
    SnapshotSaveMetadata, SnapshotSourceType, VersionedSnapshotManifest, VersionedSnapshotSummary,
};
