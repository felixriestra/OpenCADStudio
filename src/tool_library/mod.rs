//! The tool database: a real embedded SQLite catalog of reusable cutters,
//! cutting presets, vendor feed curves and chipload bands, ported from
//! 2DCam's `Sources/TwoDCamCore/ToolLibrary/` (a sibling Rust/Swift app by
//! the same author). Replaces the earlier flat-JSON `cam_library` module.
//!
//! Distinct from `ocs_cam_core::ToolDefinition`, which stays exactly as it
//! is: the project's own embedded copy of a tool baked into a `CamOperation`.
//! `LibraryTool` here is the reusable catalog row that copies *into* that.
//!
//! SQLite access needs a native SQLite build (via `rusqlite`'s `bundled`
//! feature), so the repository/store live behind `not(target_arch =
//! "wasm32")`; the web build gets an in-memory-only stand-in with the same
//! public surface, matching how `cam_library` handled wasm before it.

pub mod model;
pub mod resolver;

#[cfg(not(target_arch = "wasm32"))]
pub mod csv_import;
#[cfg(not(target_arch = "wasm32"))]
pub mod repository;
#[cfg(not(target_arch = "wasm32"))]
pub mod schema;

#[cfg(not(target_arch = "wasm32"))]
mod store;
#[cfg(not(target_arch = "wasm32"))]
pub use csv_import::ImportPlan;
#[cfg(not(target_arch = "wasm32"))]
pub use store::{DisplayUnit, ToolLibraryStore};

#[cfg(target_arch = "wasm32")]
mod store_web;
#[cfg(target_arch = "wasm32")]
pub use store_web::{DisplayUnit, ImportPlan, ToolLibraryStore};

/// A random-enough id for tools, vendors, presets, and import batches.
/// Timestamp-seeded rather than a full UUID crate dependency, matching the
/// style `cam_library::unique_id` already used in this codebase. A bare
/// timestamp collides when called in a tight loop (e.g. seeding several
/// starter tools back to back) — actual clock resolution can be coarser than
/// one nanosecond — so a per-process atomic counter is folded in too.
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{stamp:x}-{sequence:x}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

impl ValidationSeverity {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValidationIssue {
    pub row_index: usize,
    pub severity: ValidationSeverity,
    pub code: String,
    pub field: Option<String>,
    pub raw_value: Option<String>,
    pub message: String,
}
