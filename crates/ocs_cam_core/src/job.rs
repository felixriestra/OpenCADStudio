use crate::{verify_program, CamError, Motion, ProfileParameters, Program, Units};
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CamOperation {
    pub id: String,
    pub name: String,
    pub kind: OperationKind,
    pub enabled: bool,
    pub source_ids: Vec<String>,
    pub tool: ToolDefinition,
    pub parameters: ProfileParameters,
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
        self.parameters.validate()?;
        verify_program(&self.program)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CamJob {
    pub schema_version: u32,
    pub name: String,
    pub units: Units,
    pub operations: Vec<CamOperation>,
}

impl CamJob {
    pub fn new(name: impl Into<String>, units: Units) -> Self {
        Self {
            schema_version: CAM_JOB_SCHEMA_VERSION,
            name: name.into(),
            units,
            operations: Vec::new(),
        }
    }

    pub fn add_operation(&mut self, operation: CamOperation) -> Result<(), CamError> {
        operation.validate()?;
        if operation.program.units != self.units {
            return Err(CamError::MixedUnits);
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
            source_ids: vec!["AB".to_string()],
            tool: ToolDefinition::from_parameters("tool-1", parameters),
            parameters,
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
    fn operation_ids_replace_in_place() {
        let mut job = CamJob::new("Part", Units::Millimeters);
        job.add_operation(operation("op-1")).unwrap();
        job.add_operation(operation("op-1")).unwrap();
        assert_eq!(job.operations.len(), 1);
    }
}
