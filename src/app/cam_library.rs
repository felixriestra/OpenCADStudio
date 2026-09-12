use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CamLibrary {
    pub templates: Vec<ocs_cam_core::SetupTemplate>,
    pub materials: Vec<ocs_cam_core::MaterialPreset>,
    pub tools: Vec<ocs_cam_core::ToolDefinition>,
    /// Recoverable deletions, matching 2DCam's library/trash workflow.
    pub trashed_tools: Vec<ocs_cam_core::ToolDefinition>,
    pub cutting_presets: Vec<CuttingPreset>,
}

impl CamLibrary {
    fn seeded() -> Self {
        Self {
            templates: Vec::new(),
            materials: vec![
                material("Aluminum", 0.55, [0.72, 0.76, 0.80]),
                material("Hardwood", 0.85, [0.58, 0.34, 0.16]),
                material("Plywood", 1.0, [0.76, 0.58, 0.31]),
                material("Acrylic", 0.70, [0.28, 0.68, 0.86]),
                material("MDF", 0.90, [0.55, 0.39, 0.24]),
            ],
            tools: starter_tools(),
            trashed_tools: Vec::new(),
            cutting_presets: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CuttingPreset {
    pub id: String,
    pub tool_id: String,
    pub material: String,
    pub name: String,
    pub feed: f64,
    pub plunge_feed: f64,
    pub spindle_rpm: u32,
    pub step_down: f64,
    pub step_over: f64,
}

#[derive(Clone, Debug)]
pub struct ResolvedCuttingData {
    pub feed: f64,
    pub plunge_feed: f64,
    pub spindle_rpm: u32,
    pub step_down: f64,
    pub step_over: f64,
    pub source: String,
    pub estimated: bool,
}

#[derive(Clone, Debug)]
pub struct ToolImportPlan {
    pub source_name: String,
    pub tools: Vec<ocs_cam_core::ToolDefinition>,
    pub rejected: Vec<String>,
}

impl CamLibrary {
    pub fn resolve(
        &self,
        tool: &ocs_cam_core::ToolDefinition,
        material: &ocs_cam_core::MaterialPreset,
        machine: &ocs_cam_core::MachineEnvelope,
    ) -> ResolvedCuttingData {
        if let Some(preset) = self.cutting_presets.iter().find(|preset| {
            preset.tool_id == tool.id && preset.material.eq_ignore_ascii_case(&material.name)
        }) {
            return ResolvedCuttingData {
                feed: preset.feed.min(machine.maximum_feed),
                plunge_feed: preset.plunge_feed.min(machine.maximum_feed),
                spindle_rpm: preset.spindle_rpm.min(machine.maximum_spindle_rpm),
                step_down: preset.step_down,
                step_over: preset.step_over,
                source: preset.name.clone(),
                estimated: false,
            };
        }
        ResolvedCuttingData {
            feed: (tool.feed * material.feed_factor).min(machine.maximum_feed),
            plunge_feed: (tool.plunge_feed * material.feed_factor).min(machine.maximum_feed),
            spindle_rpm: tool.spindle_rpm.min(machine.maximum_spindle_rpm),
            step_down: tool.diameter * 0.6,
            step_over: tool.diameter * 0.4,
            source: format!("Estimated from {} feed factor", material.name),
            estimated: true,
        }
    }
}

pub fn plan_csv_import(source_name: String, source: &str) -> ToolImportPlan {
    let mut lines = source.lines().filter(|line| !line.trim().is_empty());
    let headers: Vec<String> = lines
        .next()
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect();
    let column = |names: &[&str]| headers.iter().position(|h| names.contains(&h.as_str()));
    let name_col = column(&["name", "tool", "description"]);
    let diameter_col = column(&["diameter", "diameter_mm", "cutting diameter"]);
    let feed_col = column(&["feed", "feed_mm_min", "feed rate"]);
    let plunge_col = column(&["plunge", "plunge_feed", "plunge_mm_min"]);
    let rpm_col = column(&["rpm", "spindle_rpm", "spindle"]);
    let mut tools = Vec::new();
    let mut rejected = Vec::new();
    for (row, line) in lines.enumerate() {
        let values: Vec<&str> = line.split(',').map(str::trim).collect();
        let get = |index: Option<usize>| index.and_then(|i| values.get(i)).copied();
        let name = get(name_col).unwrap_or_default();
        let diameter = get(diameter_col).and_then(|v| v.parse::<f64>().ok());
        if name.is_empty() || diameter.is_none_or(|v| !v.is_finite() || v <= 0.0) {
            rejected.push(format!(
                "Row {}: name and positive diameter are required",
                row + 2
            ));
            continue;
        }
        tools.push(ocs_cam_core::ToolDefinition {
            id: unique_id("imported-tool"),
            name: name.into(),
            diameter: diameter.unwrap_or(6.0),
            feed: get(feed_col)
                .and_then(|v| v.parse().ok())
                .filter(|v: &f64| *v > 0.0)
                .unwrap_or(800.0),
            plunge_feed: get(plunge_col)
                .and_then(|v| v.parse().ok())
                .filter(|v: &f64| *v > 0.0)
                .unwrap_or(250.0),
            spindle_rpm: get(rpm_col)
                .and_then(|v| v.parse().ok())
                .filter(|v: &u32| *v > 0)
                .unwrap_or(18_000),
        });
    }
    ToolImportPlan {
        source_name,
        tools,
        rejected,
    }
}

fn material(name: &str, feed_factor: f64, color: [f32; 3]) -> ocs_cam_core::MaterialPreset {
    ocs_cam_core::MaterialPreset {
        name: name.into(),
        feed_factor,
        color,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> CamLibrary {
    let Some(path) = crate::config::config_dir().map(|dir| dir.join("cam_library.json")) else {
        return CamLibrary::seeded();
    };
    let mut library = std::fs::read_to_string(path)
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_else(CamLibrary::seeded);
    if library.tools.is_empty() && library.trashed_tools.is_empty() {
        library.tools = starter_tools();
    }
    library
}

#[cfg(target_arch = "wasm32")]
pub fn load() -> CamLibrary {
    CamLibrary::seeded()
}

pub fn save(library: &CamLibrary) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let directory = crate::config::config_dir().ok_or("CAM library directory unavailable")?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let body = serde_json::to_string_pretty(library).map_err(|error| error.to_string())?;
        let path = directory.join("cam_library.json");
        let backup = directory.join("cam_library.backup.json");
        let temporary = directory.join("cam_library.next.json");
        if path.exists() {
            std::fs::copy(&path, &backup).map_err(|error| error.to_string())?;
        }
        std::fs::write(&temporary, body).map_err(|error| error.to_string())?;
        std::fs::rename(temporary, path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn starter_tools() -> Vec<ocs_cam_core::ToolDefinition> {
    [
        ("16 mm End Mill", 16.0),
        ("8 mm End Mill", 8.0),
        ("6 mm End Mill", 6.0),
        ("3 mm Ball Nose", 3.0),
    ]
    .into_iter()
    .map(|(name, diameter)| ocs_cam_core::ToolDefinition {
        id: unique_id("starter-tool"),
        name: name.into(),
        diameter,
        feed: 800.0,
        plunge_feed: 250.0,
        spindle_rpm: 18_000,
    })
    .collect()
}

pub fn unique_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{stamp}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_import_stages_valid_rows_and_reports_bad_rows() {
        let plan = plan_csv_import(
            "tools.csv".into(),
            "name,diameter_mm,feed,plunge,rpm\nSix,6,900,250,18000\nBad,0,1,1,1\n",
        );
        assert_eq!(plan.tools.len(), 1);
        assert_eq!(plan.rejected.len(), 1);
        assert_eq!(plan.tools[0].diameter, 6.0);
    }

    #[test]
    fn explicit_material_preset_wins_and_machine_limits_are_applied() {
        let mut library = CamLibrary::seeded();
        let tool = library.tools[0].clone();
        let material = library.materials[1].clone();
        library.cutting_presets.push(CuttingPreset {
            id: "p1".into(),
            tool_id: tool.id.clone(),
            material: material.name.clone(),
            name: "Hardwood verified".into(),
            feed: 9_000.0,
            plunge_feed: 2_000.0,
            spindle_rpm: 30_000,
            step_down: 2.0,
            step_over: 3.0,
        });
        let machine = ocs_cam_core::MachineEnvelope {
            travel_x: 1.0,
            travel_y: 1.0,
            travel_z: 1.0,
            maximum_spindle_rpm: 24_000,
            maximum_feed: 3_000.0,
        };
        let resolved = library.resolve(&tool, &material, &machine);
        assert!(!resolved.estimated);
        assert_eq!(resolved.feed, 3_000.0);
        assert_eq!(resolved.spindle_rpm, 24_000);
    }
}
