//! Wasm32 stand-in: SQLite needs a native build (via `rusqlite`'s `bundled`
//! feature, which requires a C toolchain wasm doesn't have), so the web
//! build gets an in-memory-only tool list with the same public surface as
//! the native `ToolLibraryStore`. No persistence, no CSV import — matching
//! how `cam_library`'s old wasm stub (`CamLibrary::seeded()`, no real
//! storage) handled the same constraint.

use std::path::PathBuf;

use super::model::{CuttingPreset, LibraryTool, MachineRow, MaterialClassInfo, ToolType};
use super::resolver::{self, ResolvedCuttingData};

pub struct ImportPlan {
    pub source_file_name: String,
}
impl ImportPlan {
    pub fn new_count(&self) -> usize {
        0
    }
    pub fn changed_count(&self) -> usize {
        0
    }
    pub fn rejected_count(&self) -> usize {
        0
    }
}

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
    pub tools: Vec<LibraryTool>,
    pub trash: Vec<LibraryTool>,
    pub material_classes: Vec<MaterialClassInfo>,
    pub material_class_id: String,
    pub machine: MachineRow,
    pub depth_of_cut_mm: Option<f64>,
    pub vendor_filter: Option<String>,
    pub type_filter: Option<ToolType>,
    pub display_unit: DisplayUnit,
    pub incomplete_only: bool,
    pub error_message: Option<String>,
    pub status_message: Option<String>,
    pub pending_import: Option<ImportPlan>,
    presets: Vec<CuttingPreset>,
}

impl ToolLibraryStore {
    pub fn new(_database_path: Option<PathBuf>) -> Self {
        Self {
            tools: starter_tools(),
            trash: Vec::new(),
            material_classes: vec![MaterialClassInfo {
                id: "hardwood".into(),
                name: "Hardwood".into(),
                melts: false,
                prefers_downcut: false,
                max_rpm_hint: None,
                feed_factor: 0.9,
            }],
            material_class_id: "hardwood".into(),
            machine: MachineRow::default(),
            depth_of_cut_mm: None,
            vendor_filter: None,
            type_filter: None,
            display_unit: DisplayUnit::Mm,
            incomplete_only: false,
            error_message: None,
            status_message: Some("The tool database runs in memory only in the browser build.".into()),
            pending_import: None,
            presets: Vec::new(),
        }
    }

    pub fn vendors(&self) -> Vec<String> {
        Vec::new()
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

    pub fn filtered_tools<'a>(&'a self, search_lower: &str) -> Vec<&'a LibraryTool> {
        self.tools
            .iter()
            .filter(|t| self.vendor_filter.is_none() || t.vendor_name.as_deref() == self.vendor_filter.as_deref())
            .filter(|t| self.type_filter.is_none() || Some(t.tool_type) == self.type_filter)
            .filter(|t| !self.incomplete_only || !t.is_machinable())
            .filter(|t| search_lower.is_empty() || t.name.to_lowercase().contains(search_lower))
            .collect()
    }

    pub fn resolved(&self, tool: &LibraryTool) -> Option<ResolvedCuttingData> {
        let material_class = self.material_classes.iter().find(|c| c.id == self.material_class_id)?;
        resolver::resolve(tool, material_class, Some(&self.machine), &self.presets, &[], &[], self.depth_of_cut_mm)
    }

    pub fn presets_for(&self, tool_id: &str) -> Vec<CuttingPreset> {
        self.presets.iter().filter(|p| p.tool_id == tool_id).cloned().collect()
    }

    pub fn save_preset(&mut self, preset: &CuttingPreset) {
        self.presets.retain(|p| p.id != preset.id);
        self.presets.push(preset.clone());
    }

    pub fn delete_preset(&mut self, id: &str) {
        self.presets.retain(|p| p.id != id);
    }

    pub fn create(&mut self) -> Option<LibraryTool> {
        let mut tool = LibraryTool::new("New End Mill", ToolType::EndMill);
        tool.diameter_mm = Some(6.0);
        tool.flute_count = Some(2);
        tool.flute_length_mm = Some(20.0);
        tool.shank_dia_mm = Some(6.0);
        tool.overall_length_mm = Some(50.0);
        self.tools.push(tool.clone());
        Some(tool)
    }

    pub fn add(&mut self, tool: &LibraryTool) -> bool {
        self.tools.push(tool.clone());
        true
    }

    pub fn save(&mut self, tool: &LibraryTool, _edited_fields: &[&str]) -> bool {
        if let Some(existing) = self.tools.iter_mut().find(|t| t.id == tool.id) {
            *existing = tool.clone();
            true
        } else {
            false
        }
    }

    pub fn delete(&mut self, id: &str) {
        if let Some(pos) = self.tools.iter().position(|t| t.id == id) {
            let tool = self.tools.remove(pos);
            self.trash.push(tool);
        }
    }

    pub fn restore(&mut self, id: &str) {
        if let Some(pos) = self.trash.iter().position(|t| t.id == id) {
            let tool = self.trash.remove(pos);
            self.tools.push(tool);
        }
    }

    pub fn purge(&mut self, id: &str) {
        self.trash.retain(|t| t.id != id);
    }

    pub fn duplicate(&mut self, id: &str) -> Option<LibraryTool> {
        let mut copy = self.tools.iter().find(|t| t.id == id)?.clone();
        copy.id = super::new_id();
        copy.name = format!("{} copy", copy.name);
        self.tools.push(copy.clone());
        Some(copy)
    }

    pub fn plan_import(&mut self, _csv_text: &str, _file_name: &str) {
        self.error_message = Some("CSV import isn't available in the browser build.".into());
    }

    pub fn commit_import(&mut self) {
        self.pending_import = None;
    }

    pub fn cancel_import(&mut self) {
        self.pending_import = None;
    }

    pub fn rollback_last_import(&mut self) {}
}

fn starter_tools() -> Vec<LibraryTool> {
    let mut tools = Vec::new();
    for (name, diameter) in [("16 mm End Mill", 16.0), ("8 mm End Mill", 8.0), ("6 mm End Mill", 6.0)] {
        let mut tool = LibraryTool::new(name, ToolType::EndMill);
        tool.diameter_mm = Some(diameter);
        tool.flute_count = Some(2);
        tools.push(tool);
    }
    tools
}
