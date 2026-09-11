use super::*;
use acadrust::EntityType;
use ocs_cam_core::{Contour, ContourVertex, ProfileParameters, Units};

impl OpenCADStudio {
    pub(super) fn dispatch_cam(&mut self, cmd: &str, i: usize) -> Option<Task<Message>> {
        let verb = cmd.split_whitespace().next().unwrap_or_default();
        match verb {
            "CAMINFO" => {
                let detail = self.tabs[i].cam_program.as_ref().map_or_else(
                    || "No generated program. Select one closed circle or polyline, then run CAMPROFILE.".to_string(),
                    |(revision, program)| {
                        if *revision == self.tabs[i].edit_revision {
                            format!("Generated program ready: {} lines. Run CAMEXPORT to save it.", program.lines().count())
                        } else {
                            "The drawing changed after toolpath generation. Regenerate before export.".to_string()
                        }
                    },
                );
                self.command_line.push_output(&detail);
            }
            "CAMPROFILE" | "CAMINSIDE" | "CAMENGRAVE" => {
                let handles = self.tabs[i].scene.selected_handles_in_order();
                if handles.len() != 1 {
                    self.command_line.push_error(
                        "CAM: select exactly one line, circle, or lightweight polyline.",
                    );
                    return Some(Task::none());
                }
                let Some(entity) = self.tabs[i].scene.document.get_entity(handles[0]) else {
                    self.command_line
                        .push_error("CAM: the selected entity no longer exists.");
                    return Some(Task::none());
                };
                let contour = match contour_from_entity(entity, verb != "CAMENGRAVE") {
                    Ok(contour) => contour,
                    Err(error) => {
                        self.command_line.push_error(&format!("{verb}: {error}"));
                        return Some(Task::none());
                    }
                };
                let mut parameters =
                    defaults_for_drawing_units(self.tabs[i].scene.document.header.insertion_units);
                if let Err(error) = apply_profile_arguments(&mut parameters, cmd) {
                    self.command_line.push_error(&format!("{verb}: {error}"));
                    return Some(Task::none());
                }
                let generated = match verb {
                    "CAMINSIDE" => ocs_cam_core::inside_profile(&contour, parameters),
                    "CAMENGRAVE" => ocs_cam_core::engrave(&contour, parameters),
                    _ => ocs_cam_core::outside_profile(&contour, parameters),
                };
                match generated {
                    Ok(program) => {
                        let motion_count = program.motions.len();
                        let gcode = match ocs_cam_core::post_grbl_checked(&program) {
                            Ok(gcode) => gcode,
                            Err(error) => {
                                self.command_line.push_error(&format!("{verb}: {error}"));
                                return Some(Task::none());
                            }
                        };
                        let line_count = gcode.lines().count();
                        self.tabs[i].cam_program = Some((self.tabs[i].edit_revision, gcode));
                        self.command_line.push_output(&format!(
                            "{verb}: generated {motion_count} motions / {line_count} G-code lines. Run CAMEXPORT to save."
                        ));
                    }
                    Err(error) => self.command_line.push_error(&format!("{verb}: {error}")),
                }
            }
            "CAMDRILL" => {
                let handles = self.tabs[i].scene.selected_handles_in_order();
                let points = handles
                    .iter()
                    .filter_map(|handle| self.tabs[i].scene.document.get_entity(*handle))
                    .map(drill_point_from_entity)
                    .collect::<Result<Vec<_>, _>>();
                let points = match points {
                    Ok(points) if !points.is_empty() => points,
                    Ok(_) => {
                        self.command_line
                            .push_error("CAMDRILL: select one or more point or circle entities.");
                        return Some(Task::none());
                    }
                    Err(error) => {
                        self.command_line.push_error(&format!("CAMDRILL: {error}"));
                        return Some(Task::none());
                    }
                };
                let mut parameters =
                    defaults_for_drawing_units(self.tabs[i].scene.document.header.insertion_units);
                if let Err(error) = apply_profile_arguments(&mut parameters, cmd) {
                    self.command_line.push_error(&format!("CAMDRILL: {error}"));
                    return Some(Task::none());
                }
                match ocs_cam_core::drill(&points, parameters) {
                    Ok(program) => {
                        let motion_count = program.motions.len();
                        let gcode = match ocs_cam_core::post_grbl_checked(&program) {
                            Ok(gcode) => gcode,
                            Err(error) => {
                                self.command_line.push_error(&format!("CAMDRILL: {error}"));
                                return Some(Task::none());
                            }
                        };
                        let line_count = gcode.lines().count();
                        self.tabs[i].cam_program = Some((self.tabs[i].edit_revision, gcode));
                        self.command_line.push_output(&format!(
                            "CAMDRILL: generated {motion_count} motions / {line_count} G-code lines. Run CAMEXPORT to save."
                        ));
                    }
                    Err(error) => self.command_line.push_error(&format!("CAMDRILL: {error}")),
                }
            }
            "CAMEXPORT" => {
                let Some((revision, program)) = self.tabs[i].cam_program.clone() else {
                    self.command_line
                        .push_error("CAMEXPORT: generate a toolpath with CAMPROFILE first.");
                    return Some(Task::none());
                };
                if revision != self.tabs[i].edit_revision {
                    self.command_line.push_error(
                        "CAMEXPORT: the drawing changed; regenerate the toolpath before export.",
                    );
                    return Some(Task::none());
                }
                return Some(Task::done(Message::CamExport(program)));
            }
            _ => return None,
        }
        Some(Task::none())
    }
}

fn contour_from_entity(entity: &EntityType, require_closed: bool) -> Result<Contour, &'static str> {
    match entity {
        EntityType::LwPolyline(polyline) => {
            if require_closed && !polyline.is_closed {
                return Err("the selected lightweight polyline is open");
            }
            if polyline.normal.z < 0.999_999
                || polyline.normal.x.abs() > 1.0e-6
                || polyline.normal.y.abs() > 1.0e-6
                || polyline.elevation.abs() > 1.0e-6
            {
                return Err("only contours in the world XY plane are supported in this milestone");
            }
            Ok(Contour {
                closed: polyline.is_closed,
                vertices: polyline
                    .vertices
                    .iter()
                    .map(|vertex| ContourVertex {
                        point: ocs_cam_core::Point2::new(vertex.location.x, vertex.location.y),
                        bulge: vertex.bulge,
                    })
                    .collect(),
            })
        }
        EntityType::Circle(circle) => {
            if circle.normal.z < 0.999_999
                || circle.normal.x.abs() > 1.0e-6
                || circle.normal.y.abs() > 1.0e-6
            {
                return Err("only circles in the world XY plane are supported in this milestone");
            }
            let center = circle.center_wcs();
            if center.z.abs() > 1.0e-6 {
                return Err("only contours in the world XY plane are supported in this milestone");
            }
            let bulge = (std::f64::consts::FRAC_PI_2 / 4.0).tan();
            Ok(Contour {
                closed: true,
                vertices: [
                    (center.x + circle.radius, center.y),
                    (center.x, center.y + circle.radius),
                    (center.x - circle.radius, center.y),
                    (center.x, center.y - circle.radius),
                ]
                .into_iter()
                .map(|(x, y)| ContourVertex {
                    point: ocs_cam_core::Point2::new(x, y),
                    bulge,
                })
                .collect(),
            })
        }
        EntityType::Line(line) if !require_closed => {
            if line.start.z.abs() > 1.0e-6 || line.end.z.abs() > 1.0e-6 {
                return Err("only engraving geometry in the world XY plane is supported");
            }
            Ok(Contour {
                closed: false,
                vertices: vec![
                    ContourVertex::line(line.start.x, line.start.y),
                    ContourVertex::line(line.end.x, line.end.y),
                ],
            })
        }
        _ => Err("supported geometry is a lightweight polyline, circle, or engraving line"),
    }
}

fn drill_point_from_entity(entity: &EntityType) -> Result<ocs_cam_core::Point2, &'static str> {
    let (x, y, z) = match entity {
        EntityType::Point(point) => (point.location.x, point.location.y, point.location.z),
        EntityType::Circle(circle) => {
            let center = circle.center_wcs();
            (center.x, center.y, center.z)
        }
        _ => return Err("drilling locations must be point or circle entities"),
    };
    if z.abs() > 1.0e-6 {
        return Err("only drilling locations in the world XY plane are supported");
    }
    Ok(ocs_cam_core::Point2::new(x, y))
}

fn defaults_for_drawing_units(insertion_units: i16) -> ProfileParameters {
    if insertion_units == 1 {
        let scale = 1.0 / 25.4;
        ProfileParameters {
            tool_diameter: 6.0 * scale,
            depth: 3.0 * scale,
            step_down: 1.0 * scale,
            safe_z: 5.0 * scale,
            feed: 500.0 * scale,
            plunge_feed: 200.0 * scale,
            units: Units::Inches,
            ..ProfileParameters::default()
        }
    } else {
        ProfileParameters::default()
    }
}

fn apply_profile_arguments(parameters: &mut ProfileParameters, cmd: &str) -> Result<(), String> {
    for argument in cmd.split_whitespace().skip(1) {
        let Some((key, value)) = argument.split_once('=') else {
            return Err(format!("expected key=value, got '{argument}'"));
        };
        match key.to_ascii_lowercase().as_str() {
            "tool" | "diameter" => parameters.tool_diameter = parse_number(key, value)?,
            "top" => parameters.stock_top = parse_number(key, value)?,
            "depth" => parameters.depth = parse_number(key, value)?,
            "stepdown" | "doc" => parameters.step_down = parse_number(key, value)?,
            "safe" => parameters.safe_z = parse_number(key, value)?,
            "feed" => parameters.feed = parse_number(key, value)?,
            "plunge" => parameters.plunge_feed = parse_number(key, value)?,
            "rpm" => {
                parameters.spindle_rpm = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid rpm '{value}'"))?
            }
            _ => return Err(format!("unknown parameter '{key}'")),
        }
    }
    parameters.validate().map_err(|error| error.to_string())
}

fn parse_number(key: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("invalid {key} value '{value}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_parameters_and_drawing_units() {
        let mut parameters = defaults_for_drawing_units(1);
        assert_eq!(parameters.units, Units::Inches);
        apply_profile_arguments(
            &mut parameters,
            "CAMPROFILE tool=0.25 depth=0.5 stepdown=0.1 feed=20 plunge=8 rpm=10000",
        )
        .unwrap();
        assert_eq!(parameters.tool_diameter, 0.25);
        assert_eq!(parameters.depth, 0.5);
        assert_eq!(parameters.spindle_rpm, 10_000);
    }

    #[test]
    fn line_is_only_accepted_for_engraving() {
        let line = EntityType::Line(acadrust::entities::Line::from_coords(
            1.0, 2.0, 0.0, 4.0, 6.0, 0.0,
        ));
        assert!(contour_from_entity(&line, false).is_ok());
        assert!(contour_from_entity(&line, true).is_err());
    }

    #[test]
    fn drill_adapter_uses_point_locations() {
        let point = EntityType::Point(acadrust::entities::Point::from_coords(3.0, 7.0, 0.0));
        assert_eq!(
            drill_point_from_entity(&point).unwrap(),
            ocs_cam_core::Point2::new(3.0, 7.0)
        );
    }
}
