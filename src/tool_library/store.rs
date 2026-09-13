//! App-facing wrapper mirroring 2DCam's `ToolLibraryStore.swift`. Mac2CAM's
//! `iced` update loop is single-threaded and synchronous, so this is a plain
//! struct read directly by the view — no `@Observable`/`@MainActor` needed.

use std::path::PathBuf;

use super::csv_import::{self, ImportPlan, ImportResult};
use super::model::{CuttingPreset, LibraryTool, MachineRow, MaterialClassInfo, ToolType};
use super::repository::ToolLibraryRepository;
use super::resolver::ResolvedCuttingData;

/// The unit the Tool Database sidebar/detail form *displays* values in.
/// Storage is always mm (`LibraryTool`'s fields) — this is purely a
/// view-session preference, not persisted, distinct from
/// `LibraryTool::DisplayUnits` (a per-tool provenance hint: "this vendor
/// published this tool in inches").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayUnit {
    Mm,
    Inch,
}

impl DisplayUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Inch => "inch",
        }
    }

    pub fn from_mm(self, value: f64) -> f64 {
        match self {
            Self::Mm => value,
            Self::Inch => value / 25.4,
        }
    }
}

pub struct ToolLibraryStore {
    repository: Option<ToolLibraryRepository>,

    pub tools: Vec<LibraryTool>,
    pub trash: Vec<LibraryTool>,
    pub material_classes: Vec<MaterialClassInfo>,

    /// The material and machine resolved feeds are computed for.
    pub material_class_id: String,
    pub machine: MachineRow,
    /// Depth of cut the user is planning, in mm. `None` shows the fastest
    /// (shallow) end of a vendor feed curve.
    pub depth_of_cut_mm: Option<f64>,

    /// Sidebar filters — session-only UI state, not persisted (2DCam's own
    /// filters reset on relaunch too).
    pub vendor_filter: Option<String>,
    pub type_filter: Option<ToolType>,
    pub display_unit: DisplayUnit,
    pub incomplete_only: bool,

    pub error_message: Option<String>,
    pub status_message: Option<String>,

    /// A staged import awaiting review. `Some` drives the review sheet.
    pub pending_import: Option<ImportPlan>,
    last_import_batch_id: Option<String>,
}

impl ToolLibraryStore {
    pub fn new(database_path: Option<PathBuf>) -> Self {
        let mut store = Self {
            repository: None,
            tools: Vec::new(),
            trash: Vec::new(),
            material_classes: Vec::new(),
            material_class_id: "hardwood".to_string(),
            machine: MachineRow::default(),
            depth_of_cut_mm: None,
            vendor_filter: None,
            type_filter: None,
            display_unit: DisplayUnit::Mm,
            incomplete_only: false,
            error_message: None,
            status_message: None,
            pending_import: None,
            last_import_batch_id: None,
        };

        let path = database_path.unwrap_or_else(Self::default_database_path);
        match ToolLibraryRepository::open(&path, true) {
            Ok(mut repo) => {
                let healthy = repo.quick_check().unwrap_or(false);
                if !healthy {
                    match Self::restore_from_backup(&path) {
                        Ok(restored) => {
                            repo = restored;
                            store.status_message =
                                Some("The tool library was damaged and has been restored from the most recent backup.".into());
                        }
                        Err(error) => store.error_message = Some(error.to_string()),
                    }
                }
                if let Err(error) = repo.upsert_machine(&store.machine) {
                    store.error_message = Some(error.to_string());
                }
                let _ = repo.rotating_backup(10);
                store.repository = Some(repo);
                if let Err(error) = store.refresh() {
                    store.error_message = Some(error.to_string());
                }
            }
            Err(error) => store.error_message = Some(error.to_string()),
        }
        store
    }

    fn restore_from_backup(path: &std::path::Path) -> Result<ToolLibraryRepository, super::repository::ToolLibraryError> {
        let backups_dir = path.parent().map(|p| p.join("Backups"));
        let newest = backups_dir.as_ref().and_then(|dir| {
            std::fs::read_dir(dir).ok().and_then(|entries| {
                let mut names: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| n.starts_with("ToolLibrary-"))
                            .unwrap_or(false)
                    })
                    .collect();
                names.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                names.into_iter().next()
            })
        });

        match newest {
            Some(newest) => {
                let _ = std::fs::remove_file(path);
                std::fs::copy(&newest, path).map_err(|e| super::repository::ToolLibraryError::Open(e.to_string()))?;
                ToolLibraryRepository::open(path, true)
            }
            None => {
                // No backup: move the damaged file aside rather than deleting it.
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let quarantine = path.with_extension(format!("damaged-{stamp}"));
                let _ = std::fs::rename(path, quarantine);
                ToolLibraryRepository::open(path, true)
            }
        }
    }

    pub fn default_database_path() -> PathBuf {
        let base = crate::config::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("ToolLibrary.sqlite3")
    }

    fn require_repository(&mut self) -> Result<&mut ToolLibraryRepository, String> {
        self.repository.as_mut().ok_or_else(|| "The tool library is unavailable.".to_string())
    }

    // MARK: - Reading

    pub fn refresh(&mut self) -> Result<(), super::repository::ToolLibraryError> {
        let Some(repository) = &self.repository else { return Ok(()) };
        self.tools = repository.tools(false)?;
        self.trash = repository.trashed_tools()?;
        self.material_classes = repository.material_classes()?;
        if !self.material_classes.iter().any(|c| c.id == self.material_class_id) {
            self.material_class_id = self.material_classes.first().map(|c| c.id.clone()).unwrap_or_else(|| "hardwood".into());
        }
        Ok(())
    }

    pub fn vendors(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.iter().filter_map(|t| t.vendor_name.clone()).collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn tool_types_present(&self) -> Vec<ToolType> {
        let mut types: Vec<ToolType> = ToolType::all()
            .iter()
            .copied()
            .filter(|t| self.tools.iter().any(|tool| tool.tool_type == *t))
            .collect();
        types.sort_by_key(|t| t.display_name());
        types
    }

    /// Tools matching the sidebar's search text plus the vendor/type/
    /// incomplete-only filters. `search` is pre-lowercased by the caller
    /// (the view owns the live text buffer).
    pub fn filtered_tools<'a>(&'a self, search_lower: &str) -> Vec<&'a LibraryTool> {
        self.tools
            .iter()
            .filter(|t| self.vendor_filter.is_none() || t.vendor_name.as_deref() == self.vendor_filter.as_deref())
            .filter(|t| self.type_filter.is_none() || Some(t.tool_type) == self.type_filter)
            .filter(|t| !self.incomplete_only || !t.is_machinable())
            .filter(|t| {
                search_lower.is_empty()
                    || t.name.to_lowercase().contains(search_lower)
                    || t.vendor_name.as_deref().unwrap_or_default().to_lowercase().contains(search_lower)
                    || t.product_id.as_deref().unwrap_or_default().to_lowercase().contains(search_lower)
            })
            .collect()
    }

    // MARK: - Cutting data

    /// What this tool should cut the selected material at, on the selected
    /// machine, at the chosen depth of cut. `None` means: not enough
    /// information, and we will not invent it.
    pub fn resolved(&self, tool: &LibraryTool) -> Option<ResolvedCuttingData> {
        let repository = self.repository.as_ref()?;
        repository
            .resolve(tool, &self.material_class_id, Some(&self.machine), self.depth_of_cut_mm)
            .ok()
            .flatten()
    }

    pub fn presets_for(&self, tool_id: &str) -> Vec<CuttingPreset> {
        self.repository
            .as_ref()
            .and_then(|r| r.presets(Some(tool_id)).ok())
            .unwrap_or_default()
    }

    /// Persist a resolved (derived) recommendation as a real preset the user owns.
    pub fn save_preset(&mut self, preset: &CuttingPreset) {
        self.perform(|store| {
            store.require_repository()?.upsert(preset, None, None).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            store.status_message = Some("Saved cutting data.".into());
            Ok(())
        });
    }

    pub fn delete_preset(&mut self, id: &str) {
        self.perform(|store| {
            store.require_repository()?.delete_preset(id).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    // MARK: - CRUD

    pub fn create(&mut self) -> Option<LibraryTool> {
        let mut created = None;
        self.perform(|store| {
            let mut tool = LibraryTool::new("New End Mill", ToolType::EndMill);
            tool.diameter_mm = Some(6.0);
            tool.flute_count = Some(2);
            tool.flute_length_mm = Some(20.0);
            tool.shank_dia_mm = Some(6.0);
            tool.overall_length_mm = Some(50.0);
            let repository = store.require_repository()?;
            repository.insert(&tool, None, None).map_err(|e| e.to_string())?;
            repository.add_to_group(&tool.id, "g-mine").map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            created = Some(tool);
            Ok(())
        });
        created
    }

    /// Inserts a fully-formed tool (e.g. one exported from a sketch) and
    /// files it under "My Tools".
    pub fn add(&mut self, tool: &LibraryTool) -> bool {
        self.perform(|store| {
            let repository = store.require_repository()?;
            repository.insert(tool, None, None).map_err(|e| e.to_string())?;
            repository.add_to_group(&tool.id, "g-mine").map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    /// `edited_fields` are stamped `origin='user'` so a later catalogue
    /// re-import cannot overwrite them.
    pub fn save(&mut self, tool: &LibraryTool, edited_fields: &[&str]) -> bool {
        let name = tool.name.clone();
        self.perform(|store| {
            store.require_repository()?.update(tool, edited_fields).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            store.status_message = Some(format!("Saved {name}."));
            Ok(())
        })
    }

    pub fn delete(&mut self, id: &str) {
        self.perform(|store| {
            store.require_repository()?.delete(id).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            store.status_message = Some("Moved to Trash. It can be restored.".into());
            Ok(())
        });
    }

    pub fn restore(&mut self, id: &str) {
        self.perform(|store| {
            store.require_repository()?.restore(id).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    pub fn purge(&mut self, id: &str) {
        self.perform(|store| {
            store.require_repository()?.purge(id).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    pub fn duplicate(&mut self, id: &str) -> Option<LibraryTool> {
        let mut duplicated = None;
        self.perform(|store| {
            duplicated = store.require_repository()?.duplicate(id).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            Ok(())
        });
        duplicated
    }

    // MARK: - Import

    /// Parse, validate and diff — but write nothing. The review UI decides.
    pub fn plan_import(&mut self, csv_text: &str, file_name: &str) {
        self.perform(|store| {
            let repository = store.require_repository()?;
            store.pending_import = Some(csv_import::plan(repository, csv_text, file_name));
            Ok(())
        });
    }

    pub fn commit_import(&mut self) {
        let Some(plan) = self.pending_import.take() else { return };
        self.perform(|store| {
            let repository = store.require_repository()?;
            let _ = repository.rotating_backup(10); // before, not after: this is the risky step
            let ImportResult { batch_id, inserted, updated, rejected } =
                csv_import::commit(repository, plan, Some("g-mine")).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            store.status_message = Some(format!(
                "Imported {inserted} new, updated {updated}, rejected {rejected}. Undo is available from the batch history."
            ));
            store.last_import_batch_id = Some(batch_id);
            Ok(())
        });
    }

    pub fn cancel_import(&mut self) {
        self.pending_import = None;
    }

    /// Undo the whole last import.
    pub fn rollback_last_import(&mut self) {
        let Some(batch_id) = self.last_import_batch_id.clone() else { return };
        self.perform(|store| {
            store.require_repository()?.rollback(&batch_id).map_err(|e| e.to_string())?;
            store.refresh().map_err(|e| e.to_string())?;
            store.last_import_batch_id = None;
            store.status_message = Some("Import rolled back.".into());
            Ok(())
        });
    }

    // MARK: - Plumbing

    fn perform(&mut self, operation: impl FnOnce(&mut Self) -> Result<(), String>) -> bool {
        match operation(self) {
            Ok(()) => true,
            Err(error) => {
                self.error_message = Some(error);
                false
            }
        }
    }
}
