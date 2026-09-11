use super::*;
use acadrust::EntityType;
use ocs_cam_core::{Contour, ContourVertex, ProfileParameters, Units};

impl OpenCADStudio {
    pub(super) fn dispatch_cam(&mut self, cmd: &str, i: usize) -> Option<Task<Message>> {
        let verb = cmd.split_whitespace().next().unwrap_or_default();
        match verb {
            "CAMINFO" => {
                let count = self.tabs[i].cam_job.operations.len();
                let detail = if count == 0 {
                    "No CAM operations. Select geometry and choose a CAM toolpath.".to_string()
                } else if self.tabs[i].cam_job_revision != Some(self.tabs[i].edit_revision) {
                    format!("{count} CAM operation(s), but the drawing changed. Regenerate before export.")
                } else {
                    format!("{count} CAM operation(s) ready. Run CAMEXPORT to save G-code and the job sidecar.")
                };
                self.command_line.push_output(&detail);
            }
            "CAMCLEAR" => {
                let units =
                    defaults_for_drawing_units(self.tabs[i].scene.document.header.insertion_units)
                        .units;
                self.tabs[i].cam_job =
                    ocs_cam_core::CamJob::new(self.tabs[i].tab_title.clone(), units);
                self.tabs[i].cam_job_revision = None;
                self.command_line
                    .push_output("CAMCLEAR: all CAM operations removed.");
            }
            "CAMPREVIEW" => {
                if self.tabs[i].cam_job_revision != Some(self.tabs[i].edit_revision) {
                    self.command_line
                        .push_error("CAMPREVIEW: regenerate toolpaths after drawing changes.");
                    return Some(Task::none());
                }
                let preview = self.tabs[i]
                    .cam_job
                    .compile()
                    .and_then(|program| ocs_cam_core::preview_segments(&program));
                match preview {
                    Ok(segments) => {
                        let cutting = segments
                            .iter()
                            .filter(|segment| segment.kind == ocs_cam_core::SegmentKind::Cut)
                            .count();
                        self.command_line.push_output(&format!(
                            "CAMPREVIEW: {cutting} cutting segments and {} rapid segments are ready for canvas playback.",
                            segments.len() - cutting
                        ));
                    }
                    Err(error) => self
                        .command_line
                        .push_error(&format!("CAMPREVIEW: {error}")),
                }
            }
            "CAMPROFILE" | "CAMINSIDE" | "CAMPOCKET" | "CAMFACE" | "CAMENGRAVE" => {
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
                let overrides = match apply_profile_arguments(&mut parameters, cmd) {
                    Ok(overrides) => overrides,
                    Err(error) => {
                        self.command_line.push_error(&format!("{verb}: {error}"));
                        return Some(Task::none());
                    }
                };
                let step_over = overrides
                    .step_over
                    .unwrap_or(parameters.tool_diameter * 0.5);
                let generated = match verb {
                    "CAMINSIDE" => ocs_cam_core::inside_profile(&contour, parameters),
                    "CAMPOCKET" => ocs_cam_core::pocket(&contour, parameters, step_over),
                    "CAMFACE" => {
                        ocs_cam_core::facing(contour_bounds(&contour), parameters, step_over)
                    }
                    "CAMENGRAVE" => ocs_cam_core::engrave(&contour, parameters),
                    _ => ocs_cam_core::outside_profile(&contour, parameters),
                };
                match generated {
                    Ok(program) => {
                        if let Err(error) =
                            self.record_cam_operation(i, verb, &handles, parameters, program)
                        {
                            self.command_line.push_error(&format!("{verb}: {error}"));
                        }
                    }
                    Err(error) => self.command_line.push_error(&format!("{verb}: {error}")),
                }
            }
            "CAMBORE" | "CAMSLOT" => {
                let handles = self.tabs[i].scene.selected_handles_in_order();
                if handles.len() != 1 {
                    self.command_line.push_error(if verb == "CAMBORE" {
                        "CAMBORE: select exactly one circle."
                    } else {
                        "CAMSLOT: select exactly one line."
                    });
                    return Some(Task::none());
                }
                let Some(entity) = self.tabs[i].scene.document.get_entity(handles[0]) else {
                    self.command_line
                        .push_error(&format!("{verb}: the selected entity no longer exists."));
                    return Some(Task::none());
                };
                let mut parameters =
                    defaults_for_drawing_units(self.tabs[i].scene.document.header.insertion_units);
                let overrides = match apply_profile_arguments(&mut parameters, cmd) {
                    Ok(overrides) => overrides,
                    Err(error) => {
                        self.command_line.push_error(&format!("{verb}: {error}"));
                        return Some(Task::none());
                    }
                };
                let generated = match (verb, entity) {
                    ("CAMBORE", EntityType::Circle(circle)) => {
                        let center = circle.center_wcs();
                        if center.z.abs() > 1.0e-6 {
                            Err(ocs_cam_core::CamError::InvalidParameters)
                        } else {
                            ocs_cam_core::bore(
                                ocs_cam_core::Point2::new(center.x, center.y),
                                circle.radius * 2.0,
                                parameters,
                            )
                        }
                    }
                    ("CAMSLOT", EntityType::Line(line))
                        if line.start.z.abs() <= 1.0e-6 && line.end.z.abs() <= 1.0e-6 =>
                    {
                        ocs_cam_core::slot(
                            ocs_cam_core::Point2::new(line.start.x, line.start.y),
                            ocs_cam_core::Point2::new(line.end.x, line.end.y),
                            overrides.width.unwrap_or(parameters.tool_diameter),
                            parameters,
                            overrides
                                .step_over
                                .unwrap_or(parameters.tool_diameter * 0.6),
                        )
                    }
                    _ => {
                        self.command_line.push_error(if verb == "CAMBORE" {
                            "CAMBORE: the selected entity must be a circle in the world XY plane."
                        } else {
                            "CAMSLOT: the selected entity must be a line in the world XY plane."
                        });
                        return Some(Task::none());
                    }
                };
                match generated {
                    Ok(program) => {
                        if let Err(error) =
                            self.record_cam_operation(i, verb, &handles, parameters, program)
                        {
                            self.command_line.push_error(&format!("{verb}: {error}"));
                        }
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
                        if let Err(error) =
                            self.record_cam_operation(i, verb, &handles, parameters, program)
                        {
                            self.command_line.push_error(&format!("CAMDRILL: {error}"));
                        }
                    }
                    Err(error) => self.command_line.push_error(&format!("CAMDRILL: {error}")),
                }
            }
            "CAMEXPORT" => {
                if self.tabs[i].cam_job.operations.is_empty() {
                    self.command_line
                        .push_error("CAMEXPORT: generate a toolpath with CAMPROFILE first.");
                    return Some(Task::none());
                }
                if self.tabs[i].cam_job_revision != Some(self.tabs[i].edit_revision) {
                    self.command_line.push_error(
                        "CAMEXPORT: the drawing changed; regenerate the toolpath before export.",
                    );
                    return Some(Task::none());
                }
                let program = match self.tabs[i].cam_job.compile() {
                    Ok(program) => program,
                    Err(error) => {
                        self.command_line.push_error(&format!("CAMEXPORT: {error}"));
                        return Some(Task::none());
                    }
                };
                let gcode = match ocs_cam_core::post_grbl_checked(&program) {
                    Ok(gcode) => gcode,
                    Err(error) => {
                        self.command_line.push_error(&format!("CAMEXPORT: {error}"));
                        return Some(Task::none());
                    }
                };
                let job = match self.tabs[i].cam_job.to_json_pretty() {
                    Ok(job) => job,
                    Err(error) => {
                        self.command_line.push_error(&format!("CAMEXPORT: {error}"));
                        return Some(Task::none());
                    }
                };
                return Some(Task::done(Message::CamExport(gcode, job)));
            }
            _ => return None,
        }
        Some(Task::none())
    }

    fn record_cam_operation(
        &mut self,
        i: usize,
        verb: &str,
        handles: &[acadrust::Handle],
        parameters: ProfileParameters,
        program: ocs_cam_core::Program,
    ) -> Result<(), String> {
        let kind = match verb {
            "CAMPROFILE" => ocs_cam_core::OperationKind::OutsideProfile,
            "CAMINSIDE" => ocs_cam_core::OperationKind::InsideProfile,
            "CAMPOCKET" => ocs_cam_core::OperationKind::Pocket,
            "CAMFACE" => ocs_cam_core::OperationKind::Facing,
            "CAMBORE" => ocs_cam_core::OperationKind::Bore,
            "CAMSLOT" => ocs_cam_core::OperationKind::Slot,
            "CAMENGRAVE" => ocs_cam_core::OperationKind::Engrave,
            "CAMDRILL" => ocs_cam_core::OperationKind::Drill,
            _ => return Err("unknown operation type".to_string()),
        };
        let revision = self.tabs[i].edit_revision;
        let mut geometry = ocs_cam_core::ManufacturingGeometry::new(
            handles
                .iter()
                .map(|handle| ocs_cam_core::GeometrySource {
                    id: format!("{handle}"),
                    document_revision: revision,
                })
                .collect(),
        );
        match verb {
            "CAMDRILL" => {
                for handle in handles {
                    let entity = self.tabs[i]
                        .scene
                        .document
                        .get_entity(*handle)
                        .ok_or_else(|| "CAM source entity no longer exists".to_string())?;
                    geometry.drill_locations.push(ocs_cam_core::DrillLocation {
                        source_id: format!("{handle}"),
                        point: drill_point_from_entity(entity).map_err(str::to_string)?,
                    });
                }
            }
            "CAMENGRAVE" | "CAMSLOT" => {
                for handle in handles {
                    let entity = self.tabs[i]
                        .scene
                        .document
                        .get_entity(*handle)
                        .ok_or_else(|| "CAM source entity no longer exists".to_string())?;
                    geometry.engraving_paths.push(ocs_cam_core::EngravingPath {
                        source_ids: vec![format!("{handle}")],
                        contour: contour_from_entity(entity, false).map_err(str::to_string)?,
                    });
                }
            }
            _ => {
                for handle in handles {
                    let entity = self.tabs[i]
                        .scene
                        .document
                        .get_entity(*handle)
                        .ok_or_else(|| "CAM source entity no longer exists".to_string())?;
                    geometry.regions.push(ocs_cam_core::MachiningRegion {
                        outer: contour_from_entity(entity, true).map_err(str::to_string)?,
                        islands: Vec::new(),
                    });
                }
            }
        }
        let geometry_fingerprint = geometry.fingerprint().map_err(|error| error.to_string())?;
        if self.tabs[i].cam_job_revision.is_some()
            && self.tabs[i].cam_job_revision != Some(revision)
        {
            self.tabs[i].cam_job =
                ocs_cam_core::CamJob::new(self.tabs[i].tab_title.clone(), parameters.units);
        }
        if self.tabs[i].cam_job.operations.is_empty() {
            self.tabs[i].cam_job.units = parameters.units;
            self.tabs[i].cam_job.name = self.tabs[i].tab_title.clone();
        }
        let sequence = self.tabs[i].cam_job.operations.len() + 1;
        let operation = ocs_cam_core::CamOperation {
            id: format!("{verb}-{sequence}"),
            name: program.name.clone(),
            kind,
            enabled: true,
            source_ids: handles.iter().map(|handle| format!("{handle}")).collect(),
            geometry: Some(geometry),
            geometry_fingerprint: Some(geometry_fingerprint),
            tool: ocs_cam_core::ToolDefinition::from_parameters(
                format!("tool-{:.4}", parameters.tool_diameter),
                parameters,
            ),
            parameters,
            program,
        };
        self.tabs[i]
            .cam_job
            .add_operation(operation)
            .map_err(|error| error.to_string())?;
        self.tabs[i].cam_job_revision = Some(revision);
        let compiled = self.tabs[i]
            .cam_job
            .compile()
            .map_err(|error| error.to_string())?;
        let motion_count = compiled.motions.len();
        let line_count = ocs_cam_core::post_grbl_checked(&compiled)
            .map_err(|error| error.to_string())?
            .lines()
            .count();
        let operation_count = self.tabs[i].cam_job.operations.len();
        self.command_line.push_output(&format!(
            "{verb}: job now has {operation_count} operation(s), {motion_count} motions / {line_count} G-code lines."
        ));
        Ok(())
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

fn contour_bounds(contour: &Contour) -> ocs_cam_core::Bounds2 {
    let points = contour.tessellated_points(0.15);
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    ocs_cam_core::Bounds2 {
        min: ocs_cam_core::Point2::new(min_x, min_y),
        max: ocs_cam_core::Point2::new(max_x, max_y),
    }
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

#[derive(Default)]
struct CamOverrides {
    step_over: Option<f64>,
    width: Option<f64>,
}

fn apply_profile_arguments(
    parameters: &mut ProfileParameters,
    cmd: &str,
) -> Result<CamOverrides, String> {
    let mut overrides = CamOverrides::default();
    for argument in cmd.split_whitespace().skip(1) {
        let Some((key, value)) = argument.split_once('=') else {
            return Err(format!("expected key=value, got '{argument}'"));
        };
        match key.to_ascii_lowercase().as_str() {
            "tool" | "diameter" => parameters.tool_diameter = parse_number(key, value)?,
            "top" => parameters.stock_top = parse_number(key, value)?,
            "depth" => parameters.depth = parse_number(key, value)?,
            "stepdown" | "doc" => parameters.step_down = parse_number(key, value)?,
            "stepover" => overrides.step_over = Some(parse_number(key, value)?),
            "width" => overrides.width = Some(parse_number(key, value)?),
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
    parameters.validate().map_err(|error| error.to_string())?;
    Ok(overrides)
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
