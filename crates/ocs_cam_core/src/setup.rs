use crate::{CamError, Point2, Units};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialPreset {
    pub name: String,
    pub feed_factor: f64,
    #[serde(default = "default_material_color")]
    pub color: [f32; 3],
}

fn default_material_color() -> [f32; 3] {
    [0.72, 0.50, 0.22]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetupTemplate {
    pub id: String,
    pub name: String,
    pub units: Units,
    pub stock: StockDefinition,
    pub material: MaterialPreset,
    pub clearance_z: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockDefinition {
    pub origin: Point2,
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    pub top_z: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineEnvelope {
    pub travel_x: f64,
    pub travel_y: f64,
    pub travel_z: f64,
    pub maximum_spindle_rpm: u32,
    pub maximum_feed: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CamSetup {
    pub id: String,
    pub name: String,
    pub units: Units,
    pub work_origin: Point2,
    pub stock: StockDefinition,
    pub machine: MachineEnvelope,
    pub material: MaterialPreset,
    pub clearance_z: f64,
}

impl CamSetup {
    pub fn default_for(units: Units) -> Self {
        let scale = if units == Units::Inches {
            1.0 / 25.4
        } else {
            1.0
        };
        Self {
            id: "setup-1".to_string(),
            name: "Setup 1".to_string(),
            units,
            work_origin: Point2::new(0.0, 0.0),
            stock: StockDefinition {
                origin: Point2::new(0.0, 0.0),
                width: 100.0 * scale,
                height: 100.0 * scale,
                thickness: 12.0 * scale,
                top_z: 0.0,
            },
            machine: MachineEnvelope {
                travel_x: 300.0 * scale,
                travel_y: 180.0 * scale,
                travel_z: 45.0 * scale,
                maximum_spindle_rpm: 24_000,
                maximum_feed: 3_000.0 * scale,
            },
            material: MaterialPreset {
                name: "Generic".to_string(),
                feed_factor: 1.0,
                color: default_material_color(),
            },
            clearance_z: 5.0 * scale,
        }
    }

    pub fn validate(&self) -> Result<(), CamError> {
        let positive = [
            self.stock.width,
            self.stock.height,
            self.stock.thickness,
            self.machine.travel_x,
            self.machine.travel_y,
            self.machine.travel_z,
            self.machine.maximum_feed,
            self.material.feed_factor,
            self.clearance_z,
        ];
        if self.id.trim().is_empty()
            || self.name.trim().is_empty()
            || self.material.name.trim().is_empty()
            || positive
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || self.machine.maximum_spindle_rpm == 0
        {
            return Err(CamError::InvalidJob);
        }
        Ok(())
    }
}
