//! Headless 2.5D CAM primitives for OpenCADStudio.
//!
//! This crate deliberately has no UI or DWG dependencies. The host converts
//! selected CAD entities into [`Contour`] values, then consumes the canonical
//! [`Program`] or posts it as controller-ready G-code.

use cadkernel::geom2d::{offset_polyline, BulgeArc, Polyline, PolylineVertex};
use serde::{Deserialize, Serialize};
use std::fmt;

const EPSILON: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContourVertex {
    pub point: Point2,
    /// DXF bulge for the segment leaving this vertex.
    pub bulge: f64,
}

impl ContourVertex {
    pub const fn line(x: f64, y: f64) -> Self {
        Self {
            point: Point2::new(x, y),
            bulge: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contour {
    pub vertices: Vec<ContourVertex>,
    pub closed: bool,
}

impl Contour {
    pub fn validate_path(&self) -> Result<(), CamError> {
        if self.vertices.len() < 2 {
            return Err(CamError::DegenerateContour);
        }
        if self
            .vertices
            .iter()
            .any(|v| !v.point.is_finite() || !v.bulge.is_finite())
        {
            return Err(CamError::NonFiniteGeometry);
        }
        Ok(())
    }

    pub fn validate_closed(&self) -> Result<(), CamError> {
        self.validate_path()?;
        if !self.closed {
            return Err(CamError::OpenContour);
        }
        if self.tessellated_points(0.15).len() < 3 || self.signed_area().abs() < EPSILON {
            return Err(CamError::DegenerateContour);
        }
        Ok(())
    }

    pub fn signed_area(&self) -> f64 {
        let points = self.tessellated_points(0.15);
        points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum::<f64>()
            * 0.5
    }

    pub fn tessellated_points(&self, maximum_angle: f64) -> Vec<Point2> {
        if self.vertices.is_empty() {
            return Vec::new();
        }
        let segment_count = if self.closed {
            self.vertices.len()
        } else {
            self.vertices.len().saturating_sub(1)
        };
        let mut points = Vec::new();
        for index in 0..segment_count {
            let start = self.vertices[index];
            let end = self.vertices[(index + 1) % self.vertices.len()];
            if let Some(arc) = BulgeArc::from_bulge(
                [start.point.x, start.point.y],
                [end.point.x, end.point.y],
                start.bulge,
            ) {
                let sampled = arc.tessellate_angle(maximum_angle);
                let take_count = sampled.len().saturating_sub(1);
                points.extend(
                    sampled
                        .into_iter()
                        .take(take_count)
                        .map(|p| Point2::new(p[0], p[1])),
                );
            } else {
                points.push(start.point);
            }
        }
        if !self.closed {
            if let Some(last) = self.vertices.last() {
                points.push(last.point);
            }
        }
        points
    }

    fn to_kernel(&self) -> Polyline {
        Polyline {
            vertices: self
                .vertices
                .iter()
                .map(|v| PolylineVertex {
                    position: [v.point.x, v.point.y],
                    bulge: v.bulge,
                })
                .collect(),
            closed: self.closed,
        }
    }

    fn from_kernel(polyline: Polyline) -> Self {
        Self {
            vertices: polyline
                .vertices
                .into_iter()
                .map(|v| ContourVertex {
                    point: Point2::new(v.position[0], v.position[1]),
                    bulge: v.bulge,
                })
                .collect(),
            closed: polyline.closed,
        }
    }

    fn outside_probe(&self, margin: f64) -> Point2 {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for point in self.tessellated_points(0.15) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }
        Point2::new(min_x - margin.max(1.0), (min_y + max_y) * 0.5)
    }

    fn inside_probe(&self, margin: f64) -> Point2 {
        let points = self.tessellated_points(0.15);
        let direction = if self.signed_area() >= 0.0 { 1.0 } else { -1.0 };
        for index in 0..points.len() {
            let start = points[index];
            let end = points[(index + 1) % points.len()];
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let length = dx.hypot(dy);
            if length > EPSILON {
                return Point2::new(
                    (start.x + end.x) * 0.5 - direction * dy / length * margin,
                    (start.y + end.y) * 0.5 + direction * dx / length * margin,
                );
            }
        }
        self.vertices[0].point
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Units {
    Millimeters,
    Inches,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfileParameters {
    pub tool_diameter: f64,
    pub stock_top: f64,
    pub depth: f64,
    pub step_down: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    pub spindle_rpm: u32,
    pub units: Units,
}

impl Default for ProfileParameters {
    fn default() -> Self {
        Self {
            tool_diameter: 6.0,
            stock_top: 0.0,
            depth: 3.0,
            step_down: 1.0,
            safe_z: 5.0,
            feed: 500.0,
            plunge_feed: 200.0,
            spindle_rpm: 12_000,
            units: Units::Millimeters,
        }
    }
}

impl ProfileParameters {
    pub fn validate(self) -> Result<(), CamError> {
        let positive = [
            self.tool_diameter,
            self.depth,
            self.step_down,
            self.feed,
            self.plunge_feed,
        ];
        if positive
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
            || !self.stock_top.is_finite()
            || !self.safe_z.is_finite()
            || self.safe_z <= self.stock_top
            || self.spindle_rpm == 0
        {
            return Err(CamError::InvalidParameters);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Motion {
    SpindleOn {
        rpm: u32,
    },
    Rapid {
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
    },
    Linear {
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        feed: f64,
    },
    Arc {
        clockwise: bool,
        end: Point2,
        center_offset: Point2,
        feed: f64,
    },
    SpindleOff,
    End,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub name: String,
    pub units: Units,
    pub motions: Vec<Motion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CamError {
    OpenContour,
    DegenerateContour,
    NonFiniteGeometry,
    InvalidParameters,
    OffsetCollapsed,
    NoDrillPoints,
    InvalidProgram,
}

impl fmt::Display for CamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OpenContour => "the selected contour is open",
            Self::DegenerateContour => "the selected contour has no machinable area",
            Self::NonFiniteGeometry => "the selected contour contains invalid coordinates",
            Self::InvalidParameters => "the CAM parameters are invalid",
            Self::OffsetCollapsed => "cutter compensation did not produce a usable contour",
            Self::NoDrillPoints => "no drill points were selected",
            Self::InvalidProgram => "the generated toolpath failed safety validation",
        })
    }
}

impl std::error::Error for CamError {}

pub fn verify_program(program: &Program) -> Result<(), CamError> {
    if program.name.trim().is_empty()
        || !matches!(program.motions.last(), Some(Motion::End))
        || program.motions.is_empty()
    {
        return Err(CamError::InvalidProgram);
    }
    let mut spindle_on = false;
    for motion in &program.motions {
        match motion {
            Motion::SpindleOn { rpm } => {
                if *rpm == 0 {
                    return Err(CamError::InvalidProgram);
                }
                spindle_on = true;
            }
            Motion::Rapid { x, y, z } => {
                if x.iter()
                    .chain(y.iter())
                    .chain(z.iter())
                    .any(|value| !value.is_finite())
                {
                    return Err(CamError::InvalidProgram);
                }
            }
            Motion::Linear { x, y, z, feed } => {
                if !spindle_on
                    || !feed.is_finite()
                    || *feed <= 0.0
                    || x.iter()
                        .chain(y.iter())
                        .chain(z.iter())
                        .any(|value| !value.is_finite())
                {
                    return Err(CamError::InvalidProgram);
                }
            }
            Motion::Arc {
                end,
                center_offset,
                feed,
                ..
            } => {
                if !spindle_on
                    || !end.is_finite()
                    || !center_offset.is_finite()
                    || center_offset.x.hypot(center_offset.y) <= EPSILON
                    || !feed.is_finite()
                    || *feed <= 0.0
                {
                    return Err(CamError::InvalidProgram);
                }
            }
            Motion::SpindleOff => spindle_on = false,
            Motion::End => {
                if spindle_on {
                    return Err(CamError::InvalidProgram);
                }
            }
        }
    }
    Ok(())
}

pub fn outside_profile(
    source: &Contour,
    parameters: ProfileParameters,
) -> Result<Program, CamError> {
    source.validate_closed()?;
    parameters.validate()?;

    let compensated = offset_contour(source, parameters.tool_diameter * 0.5, false)?;
    profile_program("Outside profile", &compensated, parameters)
}

pub fn inside_profile(
    source: &Contour,
    parameters: ProfileParameters,
) -> Result<Program, CamError> {
    source.validate_closed()?;
    parameters.validate()?;
    let compensated = offset_contour(source, parameters.tool_diameter * 0.5, true)?;
    profile_program("Inside profile", &compensated, parameters)
}

pub fn engrave(source: &Contour, parameters: ProfileParameters) -> Result<Program, CamError> {
    source.validate_path()?;
    parameters.validate()?;
    profile_program("Engrave", source, parameters)
}

pub fn drill(points: &[Point2], parameters: ProfileParameters) -> Result<Program, CamError> {
    parameters.validate()?;
    if points.is_empty() {
        return Err(CamError::NoDrillPoints);
    }
    if points.iter().any(|point| !point.is_finite()) {
        return Err(CamError::NonFiniteGeometry);
    }

    let mut motions = vec![
        Motion::SpindleOn {
            rpm: parameters.spindle_rpm,
        },
        Motion::Rapid {
            x: None,
            y: None,
            z: Some(parameters.safe_z),
        },
    ];
    for point in points {
        motions.push(Motion::Rapid {
            x: Some(point.x),
            y: Some(point.y),
            z: None,
        });
        let mut reached = 0.0;
        while reached + EPSILON < parameters.depth {
            reached = (reached + parameters.step_down).min(parameters.depth);
            motions.push(Motion::Linear {
                x: None,
                y: None,
                z: Some(parameters.stock_top - reached),
                feed: parameters.plunge_feed,
            });
            if reached + EPSILON < parameters.depth {
                motions.push(Motion::Rapid {
                    x: None,
                    y: None,
                    z: Some(parameters.stock_top),
                });
            }
        }
        motions.push(Motion::Rapid {
            x: None,
            y: None,
            z: Some(parameters.safe_z),
        });
    }
    motions.extend([Motion::SpindleOff, Motion::End]);
    Ok(Program {
        name: "Drill".to_string(),
        units: parameters.units,
        motions,
    })
}

fn offset_contour(source: &Contour, radius: f64, inside: bool) -> Result<Contour, CamError> {
    let probe = if inside {
        source.inside_probe((radius * 0.25).max(EPSILON * 10.0))
    } else {
        source.outside_probe(radius * 4.0)
    };
    let mut offsets = offset_polyline(&source.to_kernel(), radius, [probe.x, probe.y])
        .into_iter()
        .map(Contour::from_kernel)
        .filter(|contour| contour.validate_closed().is_ok())
        .collect::<Vec<_>>();
    offsets.sort_by(|a, b| a.signed_area().abs().total_cmp(&b.signed_area().abs()));
    offsets.pop().ok_or(CamError::OffsetCollapsed)
}

fn profile_program(
    name: &str,
    compensated: &Contour,
    parameters: ProfileParameters,
) -> Result<Program, CamError> {
    let start = compensated.vertices[0].point;

    let mut motions = vec![
        Motion::SpindleOn {
            rpm: parameters.spindle_rpm,
        },
        Motion::Rapid {
            x: None,
            y: None,
            z: Some(parameters.safe_z),
        },
        Motion::Rapid {
            x: Some(start.x),
            y: Some(start.y),
            z: None,
        },
    ];

    let mut reached = 0.0;
    while reached + EPSILON < parameters.depth {
        reached = (reached + parameters.step_down).min(parameters.depth);
        motions.push(Motion::Linear {
            x: None,
            y: None,
            z: Some(parameters.stock_top - reached),
            feed: parameters.plunge_feed,
        });
        append_contour_motions(&mut motions, compensated, parameters.feed);
    }
    motions.extend([
        Motion::Rapid {
            x: None,
            y: None,
            z: Some(parameters.safe_z),
        },
        Motion::SpindleOff,
        Motion::End,
    ]);

    Ok(Program {
        name: name.to_string(),
        units: parameters.units,
        motions,
    })
}

fn append_contour_motions(motions: &mut Vec<Motion>, contour: &Contour, feed: f64) {
    let count = contour.vertices.len();
    let segment_count = if contour.closed { count } else { count - 1 };
    for index in 0..segment_count {
        let start = contour.vertices[index];
        let end = contour.vertices[(index + 1) % count];
        if let Some(arc) = BulgeArc::from_bulge(
            [start.point.x, start.point.y],
            [end.point.x, end.point.y],
            start.bulge,
        ) {
            motions.push(Motion::Arc {
                clockwise: arc.sweep < 0.0,
                end: end.point,
                center_offset: Point2::new(
                    arc.center[0] - start.point.x,
                    arc.center[1] - start.point.y,
                ),
                feed,
            });
        } else {
            motions.push(Motion::Linear {
                x: Some(end.point.x),
                y: Some(end.point.y),
                z: None,
                feed,
            });
        }
    }
}

pub fn post_grbl(program: &Program) -> String {
    let mut output = String::new();
    output.push_str("(OpenCADStudio CAM)\n");
    output.push_str(match program.units {
        Units::Millimeters => "G21\n",
        Units::Inches => "G20\n",
    });
    output.push_str("G17 G90 G94\n");
    for motion in &program.motions {
        match motion {
            Motion::SpindleOn { rpm } => output.push_str(&format!("S{rpm} M3\n")),
            Motion::Rapid { x, y, z } => {
                output.push_str("G0");
                push_axis(&mut output, 'X', *x);
                push_axis(&mut output, 'Y', *y);
                push_axis(&mut output, 'Z', *z);
                output.push('\n');
            }
            Motion::Linear { x, y, z, feed } => {
                output.push_str("G1");
                push_axis(&mut output, 'X', *x);
                push_axis(&mut output, 'Y', *y);
                push_axis(&mut output, 'Z', *z);
                output.push_str(&format!(" F{}\n", format_number(*feed)));
            }
            Motion::Arc {
                clockwise,
                end,
                center_offset,
                feed,
            } => {
                output.push_str(if *clockwise { "G2" } else { "G3" });
                push_axis(&mut output, 'X', Some(end.x));
                push_axis(&mut output, 'Y', Some(end.y));
                push_axis(&mut output, 'I', Some(center_offset.x));
                push_axis(&mut output, 'J', Some(center_offset.y));
                output.push_str(&format!(" F{}\n", format_number(*feed)));
            }
            Motion::SpindleOff => output.push_str("M5\n"),
            Motion::End => output.push_str("M30\n"),
        }
    }
    output
}

pub fn post_grbl_checked(program: &Program) -> Result<String, CamError> {
    verify_program(program)?;
    Ok(post_grbl(program))
}

fn push_axis(output: &mut String, letter: char, value: Option<f64>) {
    if let Some(value) = value {
        output.push(' ');
        output.push(letter);
        output.push_str(&format_number(value));
    }
}

fn format_number(value: f64) -> String {
    let mut text = format!("{value:.4}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle() -> Contour {
        Contour {
            closed: true,
            vertices: vec![
                ContourVertex::line(0.0, 0.0),
                ContourVertex::line(40.0, 0.0),
                ContourVertex::line(40.0, 20.0),
                ContourVertex::line(0.0, 20.0),
            ],
        }
    }

    #[test]
    fn outside_profile_offsets_and_uses_depth_passes() {
        let parameters = ProfileParameters {
            tool_diameter: 6.0,
            depth: 3.0,
            step_down: 1.0,
            ..ProfileParameters::default()
        };
        let program = outside_profile(&rectangle(), parameters).unwrap();
        let plunges = program
            .motions
            .iter()
            .filter(|motion| matches!(motion, Motion::Linear { z: Some(_), .. }))
            .count();
        assert_eq!(plunges, 3);
        let gcode = post_grbl(&program);
        assert!(gcode.contains("X-3"), "{gcode}");
        assert!(gcode.contains("Z-3"), "{gcode}");
        assert!(gcode.ends_with("M30\n"));
    }

    #[test]
    fn inside_profile_offsets_toward_the_part_interior() {
        let parameters = ProfileParameters {
            tool_diameter: 6.0,
            depth: 1.0,
            step_down: 1.0,
            ..ProfileParameters::default()
        };
        let gcode = post_grbl(&inside_profile(&rectangle(), parameters).unwrap());
        assert!(gcode.contains("X3"), "{gcode}");
        assert!(!gcode.contains("X-3"), "{gcode}");
    }

    #[test]
    fn engraving_accepts_an_open_path_without_closing_it() {
        let path = Contour {
            closed: false,
            vertices: vec![
                ContourVertex::line(2.0, 3.0),
                ContourVertex::line(8.0, 3.0),
                ContourVertex::line(8.0, 7.0),
            ],
        };
        let parameters = ProfileParameters {
            depth: 1.0,
            step_down: 1.0,
            ..ProfileParameters::default()
        };
        let program = engrave(&path, parameters).unwrap();
        let xy_cuts = program
            .motions
            .iter()
            .filter(|motion| {
                matches!(
                    motion,
                    Motion::Linear {
                        x: Some(_),
                        y: Some(_),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(xy_cuts, 2);
    }

    #[test]
    fn drilling_pecks_each_selected_point() {
        let parameters = ProfileParameters {
            depth: 2.5,
            step_down: 1.0,
            ..ProfileParameters::default()
        };
        let program = drill(&[Point2::new(1.0, 2.0), Point2::new(4.0, 5.0)], parameters).unwrap();
        let plunges = program
            .motions
            .iter()
            .filter(|motion| matches!(motion, Motion::Linear { z: Some(_), .. }))
            .count();
        assert_eq!(plunges, 6);
        assert_eq!(drill(&[], parameters), Err(CamError::NoDrillPoints));
    }

    #[test]
    fn verifier_blocks_cutting_with_the_spindle_off() {
        let program = Program {
            name: "unsafe".to_string(),
            units: Units::Millimeters,
            motions: vec![
                Motion::Linear {
                    x: Some(1.0),
                    y: None,
                    z: None,
                    feed: 100.0,
                },
                Motion::End,
            ],
        };
        assert_eq!(verify_program(&program), Err(CamError::InvalidProgram));
        assert_eq!(post_grbl_checked(&program), Err(CamError::InvalidProgram));
    }

    #[test]
    fn rejects_open_contours() {
        let mut source = rectangle();
        source.closed = false;
        assert_eq!(
            outside_profile(&source, ProfileParameters::default()),
            Err(CamError::OpenContour)
        );
    }

    #[test]
    fn rejects_unsafe_parameters() {
        let parameters = ProfileParameters {
            safe_z: -1.0,
            ..ProfileParameters::default()
        };
        assert_eq!(
            outside_profile(&rectangle(), parameters),
            Err(CamError::InvalidParameters)
        );
    }
}
