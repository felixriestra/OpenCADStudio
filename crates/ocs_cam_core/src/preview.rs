use crate::{verify_program, CamError, Motion, Program};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    Rapid,
    Cut,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreviewSegment {
    pub start: Point3,
    pub end: Point3,
    pub kind: SegmentKind,
    /// Index of the motion that produced this segment. Arcs can produce many
    /// segments, all mapped to the same G-code source line.
    pub motion_index: usize,
}

pub fn preview_segments(program: &Program) -> Result<Vec<PreviewSegment>, CamError> {
    verify_program(program)?;
    let mut current = Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut segments = Vec::new();
    for (motion_index, motion) in program.motions.iter().enumerate() {
        match motion {
            Motion::Rapid { x, y, z } => {
                let end = resolve(current, *x, *y, *z);
                segments.push(PreviewSegment {
                    start: current,
                    end,
                    kind: SegmentKind::Rapid,
                    motion_index,
                });
                current = end;
            }
            Motion::Linear { x, y, z, .. } => {
                let end = resolve(current, *x, *y, *z);
                segments.push(PreviewSegment {
                    start: current,
                    end,
                    kind: SegmentKind::Cut,
                    motion_index,
                });
                current = end;
            }
            Motion::Arc {
                clockwise,
                end,
                center_offset,
                ..
            } => {
                let center_x = current.x + center_offset.x;
                let center_y = current.y + center_offset.y;
                let radius = center_offset.x.hypot(center_offset.y);
                let start_angle = (current.y - center_y).atan2(current.x - center_x);
                let end_angle = (end.y - center_y).atan2(end.x - center_x);
                let mut sweep = end_angle - start_angle;
                if *clockwise {
                    if sweep >= -1.0e-9 {
                        sweep -= std::f64::consts::TAU;
                    }
                } else if sweep <= 1.0e-9 {
                    sweep += std::f64::consts::TAU;
                }
                let steps = (sweep.abs() / 0.1).ceil().max(1.0) as usize;
                for step in 1..=steps {
                    let fraction = step as f64 / steps as f64;
                    let angle = start_angle + sweep * fraction;
                    let next = if step == steps {
                        Point3 {
                            x: end.x,
                            y: end.y,
                            z: current.z,
                        }
                    } else {
                        Point3 {
                            x: center_x + radius * angle.cos(),
                            y: center_y + radius * angle.sin(),
                            z: current.z,
                        }
                    };
                    segments.push(PreviewSegment {
                        start: current,
                        end: next,
                        kind: SegmentKind::Cut,
                        motion_index,
                    });
                    current = next;
                }
            }
            Motion::SpindleOn { .. } | Motion::SpindleOff | Motion::End => {}
        }
    }
    Ok(segments)
}

fn resolve(current: Point3, x: Option<f64>, y: Option<f64>, z: Option<f64>) -> Point3 {
    Point3 {
        x: x.unwrap_or(current.x),
        y: y.unwrap_or(current.y),
        z: z.unwrap_or(current.z),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bore, Point2, ProfileParameters};

    #[test]
    fn tessellates_full_circle_bore_for_preview() {
        let program = bore(
            Point2::new(10.0, 10.0),
            16.0,
            ProfileParameters {
                depth: 1.0,
                step_down: 1.0,
                ..ProfileParameters::default()
            },
        )
        .unwrap();
        let segments = preview_segments(&program).unwrap();
        assert!(segments.len() > 60);
        assert!(segments
            .iter()
            .any(|segment| segment.kind == SegmentKind::Rapid));
        assert!(segments
            .iter()
            .any(|segment| segment.kind == SegmentKind::Cut));
    }
}
