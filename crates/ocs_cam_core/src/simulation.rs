use crate::{preview_segments, CamError, Point2, Program, SegmentKind, StockDefinition};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockHeightField {
    pub columns: usize,
    pub rows: usize,
    pub cell_size: f64,
    pub origin: Point2,
    pub heights: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct StockSimulation {
    pub field: StockHeightField,
    segments: Vec<crate::PreviewSegment>,
    next_segment: usize,
    tool_radius: f64,
}

impl StockSimulation {
    pub fn new(
        program: &Program,
        stock: &StockDefinition,
        tool_diameter: f64,
        cell_size: f64,
    ) -> Result<Self, CamError> {
        if !tool_diameter.is_finite()
            || tool_diameter <= 0.0
            || !cell_size.is_finite()
            || cell_size <= 0.0
        {
            return Err(CamError::InvalidParameters);
        }
        let columns = (stock.width / cell_size).ceil().max(1.0) as usize;
        let rows = (stock.height / cell_size).ceil().max(1.0) as usize;
        if columns.saturating_mul(rows) > 25_000_000 {
            return Err(CamError::InvalidParameters);
        }
        Ok(Self {
            field: StockHeightField {
                columns,
                rows,
                cell_size,
                origin: stock.origin,
                heights: vec![stock.top_z as f32; columns * rows],
            },
            segments: preview_segments(program)?,
            next_segment: 0,
            tool_radius: tool_diameter * 0.5,
        })
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
    pub fn position(&self) -> usize {
        self.next_segment
    }

    pub fn advance_to(&mut self, target: usize) {
        let target = target.min(self.segments.len());
        while self.next_segment < target {
            let segment = self.segments[self.next_segment];
            if segment.kind == SegmentKind::Cut {
                let length =
                    (segment.end.x - segment.start.x).hypot(segment.end.y - segment.start.y);
                let steps = (length / (self.field.cell_size * 0.5)).ceil().max(1.0) as usize;
                for step in 0..=steps {
                    let t = step as f64 / steps as f64;
                    remove_disc(
                        &mut self.field,
                        segment.start.x + (segment.end.x - segment.start.x) * t,
                        segment.start.y + (segment.end.y - segment.start.y) * t,
                        segment.start.z + (segment.end.z - segment.start.z) * t,
                        self.tool_radius,
                    );
                }
            }
            self.next_segment += 1;
        }
    }
}

pub fn simulate_stock(
    program: &Program,
    stock: &StockDefinition,
    tool_diameter: f64,
    cell_size: f64,
) -> Result<StockHeightField, CamError> {
    let mut simulation = StockSimulation::new(program, stock, tool_diameter, cell_size)?;
    simulation.advance_to(simulation.segment_count());
    Ok(simulation.field)
}

fn remove_disc(field: &mut StockHeightField, x: f64, y: f64, z: f64, radius: f64) {
    let min_column = ((x - radius - field.origin.x) / field.cell_size)
        .floor()
        .max(0.0) as usize;
    let max_column = ((x + radius - field.origin.x) / field.cell_size)
        .ceil()
        .min(field.columns as f64) as usize;
    let min_row = ((y - radius - field.origin.y) / field.cell_size)
        .floor()
        .max(0.0) as usize;
    let max_row = ((y + radius - field.origin.y) / field.cell_size)
        .ceil()
        .min(field.rows as f64) as usize;
    for row in min_row..max_row {
        for column in min_column..max_column {
            let cx = field.origin.x + (column as f64 + 0.5) * field.cell_size;
            let cy = field.origin.y + (row as f64 + 0.5) * field.cell_size;
            if (cx - x).hypot(cy - y) <= radius {
                let index = row * field.columns + column;
                field.heights[index] = field.heights[index].min(z as f32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engrave, Contour, ContourVertex, ProfileParameters};

    #[test]
    fn cutting_moves_lower_stock_but_rapids_do_not() {
        let program = engrave(
            &Contour {
                vertices: vec![ContourVertex::line(2.0, 5.0), ContourVertex::line(8.0, 5.0)],
                closed: false,
            },
            ProfileParameters {
                depth: 2.0,
                step_down: 2.0,
                ..ProfileParameters::default()
            },
        )
        .unwrap();
        let stock = crate::CamSetup::default_for(crate::Units::Millimeters).stock;
        let field = simulate_stock(&program, &stock, 2.0, 1.0).unwrap();
        assert!(field.heights.iter().any(|height| *height < 0.0));
        assert!(field.heights.iter().any(|height| *height == 0.0));
    }

    #[test]
    fn incremental_simulation_matches_one_shot_result() {
        let program = engrave(
            &Contour {
                vertices: vec![ContourVertex::line(2.0, 5.0), ContourVertex::line(8.0, 5.0)],
                closed: false,
            },
            ProfileParameters {
                depth: 2.0,
                step_down: 1.0,
                ..ProfileParameters::default()
            },
        )
        .unwrap();
        let stock = crate::CamSetup::default_for(crate::Units::Millimeters).stock;
        let expected = simulate_stock(&program, &stock, 2.0, 1.0).unwrap();
        let mut incremental = StockSimulation::new(&program, &stock, 2.0, 1.0).unwrap();
        for step in 1..=incremental.segment_count() {
            incremental.advance_to(step);
        }
        assert_eq!(incremental.field, expected);
    }
}
