use crate::{
    verify_program, CamError, CamSetup, ManufacturingGeometry, Motion, ProfileParameters, Program,
    Units,
};
use serde::{Deserialize, Serialize};

pub const CAM_JOB_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub diameter: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    pub spindle_rpm: u32,
}

impl ToolDefinition {
    pub fn from_parameters(id: impl Into<String>, parameters: ProfileParameters) -> Self {
        Self {
            id: id.into(),
            name: format!("{} unit end mill", parameters.tool_diameter),
            diameter: parameters.tool_diameter,
            feed: parameters.feed,
            plunge_feed: parameters.plunge_feed,
            spindle_rpm: parameters.spindle_rpm,
        }
    }

    fn validate(&self) -> bool {
        !self.id.trim().is_empty()
            && !self.name.trim().is_empty()
            && self.diameter.is_finite()
            && self.diameter > 0.0
            && self.feed.is_finite()
            && self.feed > 0.0
            && self.plunge_feed.is_finite()
            && self.plunge_feed > 0.0
            && self.spindle_rpm > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    OutsideProfile,
    InsideProfile,
    Pocket,
    Facing,
    Bore,
    Slot,
    Engrave,
    Drill,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrillCycle {
    Simple,
    #[default]
    Peck,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedParameters {
    /// Lateral cut spacing. Zero means derive it from the selected tool.
    pub step_over: f64,
    /// Slot cut width. Zero means use the selected tool diameter.
    pub slot_width: f64,
    pub tab_count: u32,
    pub tab_height: f64,
    pub lead_in: f64,
    pub lead_out: f64,
    pub ramp_length: f64,
    pub finish_allowance: f64,
    pub finish_pass: bool,
    pub preserve_pocket_islands: bool,
    pub drill_cycle: DrillCycle,
}

impl Default for AdvancedParameters {
    fn default() -> Self {
        Self {
            step_over: 0.0,
            slot_width: 0.0,
            tab_count: 0,
            tab_height: 1.0,
            lead_in: 0.0,
            lead_out: 0.0,
            ramp_length: 0.0,
            finish_allowance: 0.0,
            finish_pass: false,
            preserve_pocket_islands: true,
            drill_cycle: DrillCycle::Peck,
        }
    }
}

impl AdvancedParameters {
    pub fn validate(self) -> Result<(), CamError> {
        if [
            self.step_over,
            self.slot_width,
            self.tab_height,
            self.lead_in,
            self.lead_out,
            self.ramp_length,
            self.finish_allowance,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(CamError::InvalidParameters);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CamOperation {
    pub id: String,
    pub name: String,
    pub kind: OperationKind,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_id: Option<String>,
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<ManufacturingGeometry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_fingerprint: Option<String>,
    pub tool: ToolDefinition,
    pub parameters: ProfileParameters,
    #[serde(default)]
    pub advanced: AdvancedParameters,
    pub program: Program,
}

impl CamOperation {
    pub fn validate(&self) -> Result<(), CamError> {
        if self.id.trim().is_empty()
            || self.name.trim().is_empty()
            || self.source_ids.is_empty()
            || !self.tool.validate()
            || self.parameters.units != self.program.units
        {
            return Err(CamError::InvalidJob);
        }
        match (&self.geometry, &self.geometry_fingerprint) {
            (Some(geometry), Some(fingerprint)) => {
                geometry.validate()?;
                if geometry.fingerprint()? != *fingerprint {
                    return Err(CamError::InvalidJob);
                }
            }
            (None, None) => {}
            _ => return Err(CamError::InvalidJob),
        }
        self.parameters.validate()?;
        self.advanced.validate()?;
        verify_program(&self.program)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CamJob {
    pub schema_version: u32,
    pub name: String,
    pub units: Units,
    #[serde(default)]
    pub setups: Vec<CamSetup>,
    #[serde(default)]
    pub tool_library: Vec<ToolDefinition>,
    pub operations: Vec<CamOperation>,
}

impl CamJob {
    pub fn new(name: impl Into<String>, units: Units) -> Self {
        Self {
            schema_version: CAM_JOB_SCHEMA_VERSION,
            name: name.into(),
            units,
            setups: vec![CamSetup::default_for(units)],
            tool_library: Vec::new(),
            operations: Vec::new(),
        }
    }

    pub fn add_operation(&mut self, operation: CamOperation) -> Result<(), CamError> {
        operation.validate()?;
        if operation.program.units != self.units {
            return Err(CamError::MixedUnits);
        }
        if let Some(existing) = self
            .tool_library
            .iter_mut()
            .find(|tool| tool.id == operation.tool.id)
        {
            *existing = operation.tool.clone();
        } else {
            self.tool_library.push(operation.tool.clone());
        }
        if let Some(existing) = self
            .operations
            .iter_mut()
            .find(|item| item.id == operation.id)
        {
            *existing = operation;
        } else {
            self.operations.push(operation);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), CamError> {
        if self.schema_version != CAM_JOB_SCHEMA_VERSION || self.name.trim().is_empty() {
            return Err(CamError::InvalidJob);
        }
        for operation in &self.operations {
            if operation.program.units != self.units {
                return Err(CamError::MixedUnits);
            }
            operation.validate()?;
            if let Some(setup_id) = &operation.setup_id {
                let setup = self
                    .setups
                    .iter()
                    .find(|setup| setup.id == *setup_id)
                    .ok_or(CamError::InvalidJob)?;
                verify_operation_envelope(operation, setup)?;
            }
        }
        if self.tool_library.iter().any(|tool| !tool.validate()) {
            return Err(CamError::InvalidJob);
        }
        for setup in &self.setups {
            setup.validate()?;
            if setup.units != self.units {
                return Err(CamError::MixedUnits);
            }
        }
        Ok(())
    }

    pub fn compile(&self) -> Result<Program, CamError> {
        self.validate()?;
        let mut motions = Vec::new();
        for operation in self.operations.iter().filter(|operation| operation.enabled) {
            motions.extend(
                operation
                    .program
                    .motions
                    .iter()
                    .filter(|motion| !matches!(motion, Motion::End))
                    .cloned(),
            );
        }
        if motions.is_empty() {
            return Err(CamError::InvalidJob);
        }
        motions.push(Motion::End);
        let program = Program {
            name: self.name.clone(),
            units: self.units,
            motions,
        };
        verify_program(&program)?;
        Ok(program)
    }

    pub fn to_json_pretty(&self) -> Result<String, CamError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|_| CamError::InvalidJob)
    }

    pub fn from_json(json: &str) -> Result<Self, CamError> {
        let job: Self = serde_json::from_str(json).map_err(|_| CamError::InvalidJob)?;
        job.validate()?;
        Ok(job)
    }
}

fn verify_operation_envelope(operation: &CamOperation, setup: &CamSetup) -> Result<(), CamError> {
    if operation.tool.spindle_rpm > setup.machine.maximum_spindle_rpm
        || operation.parameters.feed > setup.machine.maximum_feed
        || operation.parameters.plunge_feed > setup.machine.maximum_feed
        || operation.parameters.depth > setup.stock.thickness
    {
        return Err(CamError::InvalidProgram);
    }
    let segments = crate::preview_segments(&operation.program)?;
    if segments
        .iter()
        .flat_map(|segment| [segment.start, segment.end])
        .any(|point| {
            (point.x - setup.work_origin.x).abs() > setup.machine.travel_x
                || (point.y - setup.work_origin.y).abs() > setup.machine.travel_y
                || point.z.abs() > setup.machine.travel_z
        })
    {
        return Err(CamError::InvalidProgram);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{outside_profile, Contour, ContourVertex};

    fn operation(id: &str) -> CamOperation {
        let parameters = ProfileParameters {
            depth: 1.0,
            step_down: 1.0,
            ..ProfileParameters::default()
        };
        let contour = Contour {
            closed: true,
            vertices: vec![
                ContourVertex::line(0.0, 0.0),
                ContourVertex::line(20.0, 0.0),
                ContourVertex::line(20.0, 10.0),
                ContourVertex::line(0.0, 10.0),
            ],
        };
        CamOperation {
            id: id.to_string(),
            name: "Profile".to_string(),
            kind: OperationKind::OutsideProfile,
            enabled: true,
            setup_id: None,
            source_ids: vec!["AB".to_string()],
            geometry: None,
            geometry_fingerprint: None,
            tool: ToolDefinition::from_parameters("tool-1", parameters),
            parameters,
            advanced: AdvancedParameters::default(),
            program: outside_profile(&contour, parameters).unwrap(),
        }
    }

    #[test]
    fn job_round_trips_and_compiles_multiple_operations() {
        let mut job = CamJob::new("Part", Units::Millimeters);
        job.add_operation(operation("op-1")).unwrap();
        job.add_operation(operation("op-2")).unwrap();
        let json = job.to_json_pretty().unwrap();
        let restored = CamJob::from_json(&json).unwrap();
        assert_eq!(restored, job);
        let compiled = restored.compile().unwrap();
        assert_eq!(
            compiled
                .motions
                .iter()
                .filter(|motion| matches!(motion, Motion::End))
                .count(),
            1
        );
    }

    #[test]
    fn legacy_operation_json_without_geometry_snapshot_still_loads() {
        let mut job = CamJob::new("Legacy", Units::Millimeters);
        job.add_operation(operation("legacy-op")).unwrap();
        let json = job.to_json_pretty().unwrap();
        assert!(!json.contains("geometry_fingerprint"));
        assert!(!json.contains("\"geometry\""));
        let restored = CamJob::from_json(&json).unwrap();
        assert!(restored.operations[0].geometry.is_none());
    }

    #[test]
    fn operation_ids_replace_in_place() {
        let mut job = CamJob::new("Part", Units::Millimeters);
        job.add_operation(operation("op-1")).unwrap();
        job.add_operation(operation("op-1")).unwrap();
        assert_eq!(job.operations.len(), 1);
    }
}
