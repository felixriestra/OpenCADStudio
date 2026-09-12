use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CamLibrary {
    pub templates: Vec<ocs_cam_core::SetupTemplate>,
    pub materials: Vec<ocs_cam_core::MaterialPreset>,
    pub tools: Vec<ocs_cam_core::ToolDefinition>,
    /// Recoverable deletions, matching 2DCam's library/trash workflow.
    pub trashed_tools: Vec<ocs_cam_core::ToolDefinition>,
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
        }
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
