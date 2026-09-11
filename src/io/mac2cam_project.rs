//! Native Mac2CAM project package persistence.
//!
//! A project is a ZIP container with stable, inspectable entry names.  The
//! DWG remains the authoritative editable drawing while CAM state is JSON.

use ocs_cam_core::{CamJob, ToolDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

pub const PROJECT_SCHEMA_VERSION: u32 = 1;
const MAX_DRAWING_BYTES: u64 = 512 * 1024 * 1024;
const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub product: String,
    pub drawing_entry: String,
    pub cam_entry: String,
    pub tools_entry: String,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            product: "Mac2CAM".to_string(),
            drawing_entry: "drawing.dwg".to_string(),
            cam_entry: "cam.json".to_string(),
            tools_entry: "tools.json".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectContents {
    pub manifest: ProjectManifest,
    pub drawing: Vec<u8>,
    pub cam_job: CamJob,
    pub tools: Vec<ToolDefinition>,
}

impl ProjectContents {
    pub fn new(drawing: Vec<u8>, cam_job: CamJob) -> Result<Self, String> {
        cam_job.validate().map_err(|error| error.to_string())?;
        let mut seen = HashSet::new();
        let tools = cam_job
            .operations
            .iter()
            .map(|operation| operation.tool.clone())
            .filter(|tool| seen.insert(tool.id.clone()))
            .collect();
        Ok(Self {
            manifest: ProjectManifest::default(),
            drawing,
            cam_job,
            tools,
        })
    }
}

pub fn encode_project(project: &ProjectContents) -> Result<Vec<u8>, String> {
    validate_project(project)?;
    let manifest = serde_json::to_vec_pretty(&project.manifest).map_err(|e| e.to_string())?;
    let cam = serde_json::to_vec_pretty(&project.cam_job).map_err(|e| e.to_string())?;
    let tools = serde_json::to_vec_pretty(&project.tools).map_err(|e| e.to_string())?;
    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for (name, bytes) in [
        ("manifest.json", manifest.as_slice()),
        (
            project.manifest.drawing_entry.as_str(),
            project.drawing.as_slice(),
        ),
        (project.manifest.cam_entry.as_str(), cam.as_slice()),
        (project.manifest.tools_entry.as_str(), tools.as_slice()),
    ] {
        archive
            .start_file(name, options)
            .map_err(|e| e.to_string())?;
        archive.write_all(bytes).map_err(|e| e.to_string())?;
    }
    archive
        .add_directory("toolpaths/", options)
        .map_err(|e| e.to_string())?;
    archive
        .add_directory("simulation/", options)
        .map_err(|e| e.to_string())?;
    Ok(archive.finish().map_err(|e| e.to_string())?.into_inner())
}

pub fn decode_project(bytes: &[u8]) -> Result<ProjectContents, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let manifest: ProjectManifest =
        serde_json::from_slice(&read_entry(&mut archive, "manifest.json", MAX_JSON_BYTES)?)
            .map_err(|e| format!("invalid manifest.json: {e}"))?;
    if manifest.schema_version != PROJECT_SCHEMA_VERSION || manifest.product != "Mac2CAM" {
        return Err(format!(
            "unsupported Mac2CAM project schema {}",
            manifest.schema_version
        ));
    }
    let drawing = read_entry(&mut archive, &manifest.drawing_entry, MAX_DRAWING_BYTES)?;
    let cam_job: CamJob = serde_json::from_slice(&read_entry(
        &mut archive,
        &manifest.cam_entry,
        MAX_JSON_BYTES,
    )?)
    .map_err(|e| format!("invalid cam.json: {e}"))?;
    let tools: Vec<ToolDefinition> = serde_json::from_slice(&read_entry(
        &mut archive,
        &manifest.tools_entry,
        MAX_JSON_BYTES,
    )?)
    .map_err(|e| format!("invalid tools.json: {e}"))?;
    let project = ProjectContents {
        manifest,
        drawing,
        cam_job,
        tools,
    };
    validate_project(&project)?;
    Ok(project)
}

pub fn read_project(path: &Path) -> Result<ProjectContents, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    decode_project(&bytes)
}

pub fn write_project_atomic(path: &Path, project: &ProjectContents) -> Result<(), String> {
    let bytes = encode_project(project)?;
    let temp = temporary_sibling(path);
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|e| format!("create {}: {e}", temp.display()))?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&temp, path).map_err(|e| format!("replace {}: {e}", path.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn validate_project(project: &ProjectContents) -> Result<(), String> {
    if project.drawing.is_empty() || project.drawing.len() as u64 > MAX_DRAWING_BYTES {
        return Err("project drawing is empty or too large".to_string());
    }
    project.cam_job.validate().map_err(|e| e.to_string())?;
    let tool_ids: HashSet<_> = project.tools.iter().map(|tool| tool.id.as_str()).collect();
    if project
        .cam_job
        .operations
        .iter()
        .any(|operation| !tool_ids.contains(operation.tool.id.as_str()))
    {
        return Err("tools.json is missing a tool used by an operation".to_string());
    }
    Ok(())
}

fn read_entry(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
    maximum: u64,
) -> Result<Vec<u8>, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| format!("project is missing {name}"))?;
    if entry.size() > maximum {
        return Err(format!("project entry {name} is too large"));
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn temporary_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocs_cam_core::Units;

    #[test]
    fn project_round_trips_all_authoritative_entries() {
        let project = ProjectContents::new(
            b"AC1032 sample drawing".to_vec(),
            CamJob::new("Bracket", Units::Millimeters),
        )
        .unwrap();
        let encoded = encode_project(&project).unwrap();
        let restored = decode_project(&encoded).unwrap();
        assert_eq!(restored, project);
        assert!(encoded.starts_with(b"PK"));
    }

    #[test]
    fn rejects_a_package_missing_the_drawing() {
        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        archive
            .start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive
            .write_all(&serde_json::to_vec(&ProjectManifest::default()).unwrap())
            .unwrap();
        let encoded = archive.finish().unwrap().into_inner();
        assert!(decode_project(&encoded)
            .unwrap_err()
            .contains("drawing.dwg"));
    }
}
