use crate::{CamError, Contour, Point2};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const MANUFACTURING_GEOMETRY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeometrySource {
    pub id: String,
    pub document_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachiningRegion {
    pub outer: Contour,
    pub islands: Vec<Contour>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngravingPath {
    pub source_ids: Vec<String>,
    pub contour: Contour,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrillLocation {
    pub source_id: String,
    pub point: Point2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManufacturingGeometry {
    pub schema_version: u32,
    pub sources: Vec<GeometrySource>,
    pub regions: Vec<MachiningRegion>,
    pub engraving_paths: Vec<EngravingPath>,
    pub drill_locations: Vec<DrillLocation>,
}

impl ManufacturingGeometry {
    pub fn new(sources: Vec<GeometrySource>) -> Self {
        Self {
            schema_version: MANUFACTURING_GEOMETRY_SCHEMA_VERSION,
            sources,
            regions: Vec::new(),
            engraving_paths: Vec::new(),
            drill_locations: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), CamError> {
        if self.schema_version != MANUFACTURING_GEOMETRY_SCHEMA_VERSION
            || self.sources.is_empty()
            || (self.regions.is_empty()
                && self.engraving_paths.is_empty()
                && self.drill_locations.is_empty())
        {
            return Err(CamError::InvalidJob);
        }

        let mut source_ids = HashSet::new();
        for source in &self.sources {
            if source.id.trim().is_empty() || !source_ids.insert(source.id.as_str()) {
                return Err(CamError::InvalidJob);
            }
        }

        for region in &self.regions {
            region.outer.validate_closed()?;
            for island in &region.islands {
                island.validate_closed()?;
            }
        }
        for path in &self.engraving_paths {
            if path.source_ids.is_empty()
                || path
                    .source_ids
                    .iter()
                    .any(|id| !source_ids.contains(id.as_str()))
            {
                return Err(CamError::InvalidJob);
            }
            path.contour.validate_path()?;
        }
        for drill in &self.drill_locations {
            if !source_ids.contains(drill.source_id.as_str())
                || !drill.point.x.is_finite()
                || !drill.point.y.is_finite()
            {
                return Err(CamError::InvalidJob);
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<String, CamError> {
        self.validate()?;
        let encoded = serde_json::to_vec(self).map_err(|_| CamError::InvalidJob)?;
        let digest = Sha256::digest(encoded);
        Ok(format!("{digest:x}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContourVertex;

    fn rectangle(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Contour {
        Contour {
            closed: true,
            vertices: vec![
                ContourVertex::line(min_x, min_y),
                ContourVertex::line(max_x, min_y),
                ContourVertex::line(max_x, max_y),
                ContourVertex::line(min_x, max_y),
            ],
        }
    }

    #[test]
    fn region_with_island_round_trips_and_fingerprints_stably() {
        let mut geometry = ManufacturingGeometry::new(vec![GeometrySource {
            id: "AB".to_string(),
            document_revision: 7,
        }]);
        geometry.regions.push(MachiningRegion {
            outer: rectangle(0.0, 0.0, 20.0, 10.0),
            islands: vec![rectangle(5.0, 3.0, 8.0, 6.0)],
        });

        let fingerprint = geometry.fingerprint().unwrap();
        let json = serde_json::to_string(&geometry).unwrap();
        let restored: ManufacturingGeometry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, geometry);
        assert_eq!(restored.fingerprint().unwrap(), fingerprint);
        assert_eq!(fingerprint.len(), 64);
    }

    #[test]
    fn rejects_unknown_sources_and_empty_snapshots() {
        let empty = ManufacturingGeometry::new(vec![GeometrySource {
            id: "AB".to_string(),
            document_revision: 1,
        }]);
        assert_eq!(empty.validate(), Err(CamError::InvalidJob));

        let mut invalid = empty;
        invalid.engraving_paths.push(EngravingPath {
            source_ids: vec!["MISSING".to_string()],
            contour: Contour {
                closed: false,
                vertices: vec![ContourVertex::line(0.0, 0.0), ContourVertex::line(1.0, 0.0)],
            },
        });
        assert_eq!(invalid.validate(), Err(CamError::InvalidJob));
    }
}
