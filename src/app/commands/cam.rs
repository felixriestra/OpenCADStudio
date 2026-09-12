use super::*;
use acadrust::{CadDocument, EntityType, Handle};
use ocs_cam_core::{Contour, ContourVertex, ProfileParameters, Units};

impl Mac2CAM {
    pub(crate) fn refresh_cam_operation(&mut self, tab_index: usize, operation_index: usize) -> Result<(), String> {
        let operation = self.tabs[tab_index]
            .cam_job
            .operations
            .get(operation_index)
            .ok_or_else(|| "CAM operation no longer exists".to_string())?;
        let geometry = manufacturing_geometry(
            &self.tabs[tab_index].scene.document,
            operation.kind,
            &operation.source_ids,
        )?;
        let fingerprint = geometry.fingerprint().map_err(|error| error.to_string())?;
        let operation = &mut self.tabs[tab_index].cam_job.operations[operation_index];
        operation.geometry = Some(geometry);
        operation.geometry_fingerprint = Some(fingerprint);
        regenerate_cam_operation(operation)
    }

    pub(super) fn dispatch_cam(&mut self, cmd: &str, i: usize) -> Option<Task<Message>> {
        let verb = cmd.split_whitespace().next().unwrap_or_default();
        if verb == "CAMSETUP" {
            self.show_cam_setup_panel = true;
            self.dock_expanded = Some(crate::ui::dock::PanelId::CamSetup);
            if let Some(stock) = self.tabs[i].cam_job.setups.first().map(|setup| setup.stock.clone()) {
                self.tabs[i].scene.set_cam_stock_boundary(&stock);
            }
        } else {
            self.show_cam_panel = true;
            self.dock_expanded = Some(crate::ui::dock::PanelId::Cam);
        }
        match verb {
            "CAMTOOLS" => {
                self.command_line.push_output("CAMTOOLS: tool library opened.");
            }
            "CAMINFO" => {
                let count = self.tabs[i].cam_job.operations.len();
                let detail = if count == 0 {
                    "No CAM operations. Select geometry and choose a CAM toolpath.".to_string()
                } else if let Some(stale) =
                    first_stale_operation(&self.tabs[i].scene.document, &self.tabs[i].cam_job)
                {
                    format!("{count} CAM operation(s); '{stale}' has changed source geometry and must be regenerated.")
                } else {
                    format!("{count} CAM operation(s) ready. Save the Mac2CAM project or run CAMEXPORT for G-code.")
                };
                self.command_line.push_output(&detail);
            }
            "CAMLIST" => {
                if self.tabs[i].cam_job.operations.is_empty() {
                    self.command_line.push_output("CAMLIST: no operations.");
                }
                for (index, operation) in self.tabs[i].cam_job.operations.iter().enumerate() {
                    self.command_line.push_output(&format!(
                        "{}: [{}] {} ({:?})",
                        index + 1,
                        if operation.enabled { "on" } else { "off" },
                        operation.name,
                        operation.kind
                    ));
                }
            }
            "CAMDELETE" | "CAMENABLE" | "CAMDISABLE" | "CAMDUP" => {
                let Some(index) = cam_operation_index(cmd, self.tabs[i].cam_job.operations.len())
                else {
                    self.command_line.push_error(&format!(
                        "{verb}: provide an operation number from CAMLIST."
                    ));
                    return Some(Task::none());
                };
                match verb {
                    "CAMDELETE" => {
                        let removed = self.tabs[i].cam_job.operations.remove(index);
                        self.command_line
                            .push_output(&format!("CAMDELETE: removed '{}'.", removed.name));
                    }
                    "CAMDUP" => {
                        let mut copy = self.tabs[i].cam_job.operations[index].clone();
                        let sequence = self.tabs[i].cam_job.operations.len() + 1;
                        copy.id = format!("{}-copy-{sequence}", copy.id);
                        copy.name = format!("{} copy", copy.name);
                        self.tabs[i].cam_job.operations.insert(index + 1, copy);
                        self.command_line
                            .push_output("CAMDUP: operation duplicated.");
                    }
                    _ => {
                        self.tabs[i].cam_job.operations[index].enabled = verb == "CAMENABLE";
                        self.command_line.push_output(&format!(
                            "{verb}: operation {} is now {}.",
                            index + 1,
                            if verb == "CAMENABLE" {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        ));
                    }
                }
                self.tabs[i].dirty = true;
            }
            "CAMMOVE" => {
                let values: Vec<_> = cmd.split_whitespace().skip(1).collect();
                let parsed = values
                    .first()
                    .and_then(|value| value.parse::<usize>().ok())
                    .zip(values.get(1).and_then(|value| value.parse::<usize>().ok()));
                let count = self.tabs[i].cam_job.operations.len();
                let Some((from, to)) = parsed
                    .filter(|(from, to)| *from > 0 && *to > 0 && *from <= count && *to <= count)
                else {
                    self.command_line
                        .push_error("CAMMOVE: use CAMMOVE <from> <to> with numbers from CAMLIST.");
                    return Some(Task::none());
                };
                let operation = self.tabs[i].cam_job.operations.remove(from - 1);
                self.tabs[i].cam_job.operations.insert(to - 1, operation);
                self.tabs[i].dirty = true;
                self.command_line
                    .push_output(&format!("CAMMOVE: moved operation {from} to {to}."));
            }
            "CAMSETUP" => {
                let Some(setup) = self.tabs[i].cam_job.setups.first_mut() else {
                    return Some(Task::none());
                };
                if cmd.split_whitespace().count() == 1 {
                    self.command_line.push_output(&format!(
                        "CAMSETUP: stock {}×{}×{}, origin {},{}, clearance {}, machine {}×{}×{}.",
                        setup.stock.width,
                        setup.stock.height,
                        setup.stock.thickness,
                        setup.work_origin.x,
                        setup.work_origin.y,
                        setup.clearance_z,
                        setup.machine.travel_x,
                        setup.machine.travel_y,
                        setup.machine.travel_z
                    ));
                    return Some(Task::none());
                }
                let mut candidate = setup.clone();
                let result = apply_setup_arguments(&mut candidate, cmd);
                match result.and_then(|()| candidate.validate().map_err(|error| error.to_string()))
                {
                    Ok(()) => {
                        *setup = candidate;
                        self.tabs[i].dirty = true;
                        self.command_line.push_output("CAMSETUP: setup updated.");
                    }
                    Err(error) => self.command_line.push_error(&format!("CAMSETUP: {error}")),
                }
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
                if let Some(stale) =
                    first_stale_operation(&self.tabs[i].scene.document, &self.tabs[i].cam_job)
                {
                    self.command_line.push_error(&format!(
                        "CAMPREVIEW: regenerate '{stale}' after its source geometry changed."
                    ));
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
                if handles.is_empty() {
                    self.command_line.push_error(
                        "CAM: select a closed planar curve or connected planar line group.",
                    );
                    return Some(Task::none());
                }
                let (contour, pocket_region) = if verb == "CAMENGRAVE" {
                    if handles.len() != 1 {
                        self.command_line
                            .push_error("CAMENGRAVE: select exactly one planar path.");
                        return Some(Task::none());
                    }
                    let Some(entity) = self.tabs[i].scene.document.get_entity(handles[0]) else {
                        self.command_line
                            .push_error("CAM: the selected entity no longer exists.");
                        return Some(Task::none());
                    };
                    match contour_from_entity(entity, false) {
                        Ok(contour) => (contour, None),
                        Err(error) => {
                            self.command_line.push_error(&format!("{verb}: {error}"));
                            return Some(Task::none());
                        }
                    }
                } else {
                    let contours = match closed_contours_from_selection(
                        &self.tabs[i].scene.document,
                        &handles,
                    ) {
                        Ok(contours) => contours,
                        Err(error) => {
                            self.command_line.push_error(&format!("{verb}: {error}"));
                            return Some(Task::none());
                        }
                    };
                    if verb != "CAMPOCKET" && contours.len() != 1 {
                        self.command_line.push_error(&format!(
                            "{verb}: selection contains {} separate closed contours; select one.",
                            contours.len()
                        ));
                        return Some(Task::none());
                    }
                    let mut contours = contours.into_iter();
                    let outer = contours.next().expect("closed contour selection is non-empty");
                    let region = (verb == "CAMPOCKET").then(|| ocs_cam_core::MachiningRegion {
                        outer: outer.clone(),
                        islands: contours.collect(),
                    });
                    (outer, region)
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
                let mut advanced = ocs_cam_core::AdvancedParameters::default();
                if matches!(verb, "CAMPOCKET" | "CAMFACE") {
                    advanced.step_over = step_over;
                }
                let generated = match verb {
                    "CAMINSIDE" => ocs_cam_core::inside_profile(&contour, parameters),
                    "CAMPOCKET" => ocs_cam_core::pocket_region(
                        pocket_region.as_ref().expect("pocket region was created"),
                        parameters,
                        step_over,
                        true,
                    ),
                    "CAMFACE" => {
                        ocs_cam_core::facing(contour_bounds(&contour), parameters, step_over)
                    }
                    "CAMENGRAVE" => ocs_cam_core::engrave(&contour, parameters),
                    _ => ocs_cam_core::outside_profile(&contour, parameters),
                };
                match generated {
                    Ok(program) => {
                        if let Err(error) =
                            self.record_cam_operation(i, verb, &handles, parameters, advanced, program)
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
                let mut advanced = ocs_cam_core::AdvancedParameters::default();
                if verb == "CAMSLOT" {
                    advanced.slot_width = overrides.width.unwrap_or(parameters.tool_diameter);
                    advanced.step_over = overrides.step_over.unwrap_or(parameters.tool_diameter * 0.6);
                }
                match generated {
                    Ok(program) => {
                        if let Err(error) =
                            self.record_cam_operation(i, verb, &handles, parameters, advanced, program)
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
                            self.record_cam_operation(i, verb, &handles, parameters, ocs_cam_core::AdvancedParameters::default(), program)
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
                if let Some(stale) =
                    first_stale_operation(&self.tabs[i].scene.document, &self.tabs[i].cam_job)
                {
                    self.command_line.push_error(&format!(
                        "CAMEXPORT: regenerate '{stale}' because its source geometry changed."
                    ));
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
        advanced: ocs_cam_core::AdvancedParameters,
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
        let source_ids: Vec<String> = handles.iter().map(|handle| format!("{handle}")).collect();
        let geometry = manufacturing_geometry(&self.tabs[i].scene.document, kind, &source_ids)?;
        let geometry_fingerprint = geometry.fingerprint().map_err(|error| error.to_string())?;
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
            setup_id: self.tabs[i]
                .cam_job
                .setups
                .first()
                .map(|setup| setup.id.clone()),
            source_ids,
            geometry: Some(geometry),
            geometry_fingerprint: Some(geometry_fingerprint),
            tool: ocs_cam_core::ToolDefinition::from_parameters(
                format!("tool-{:.4}", parameters.tool_diameter),
                parameters,
            ),
            parameters,
            advanced,
            program,
        };
        self.tabs[i]
            .cam_job
            .add_operation(operation)
            .map_err(|error| error.to_string())?;
        // The operation editor is selection-driven. Newly-created toolpaths
        // must become active immediately so Face, Pocket, Profile, etc. expose
        // their own parameters instead of leaving the panel in its empty
        // "select an operation" state.
        let new_operation_index = self.tabs[i].cam_job.operations.len() - 1;
        self.cam_selected_operation = Some(new_operation_index);
        self.cam_editor.clear_operation();
        self.show_cam_panel = true;
        self.dock_expanded = Some(crate::ui::dock::PanelId::Cam);
        self.tabs[i].cam_job_revision = Some(self.tabs[i].edit_revision);
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

fn manufacturing_geometry(
    document: &CadDocument,
    kind: ocs_cam_core::OperationKind,
    source_ids: &[String],
) -> Result<ocs_cam_core::ManufacturingGeometry, String> {
    let mut geometry = ocs_cam_core::ManufacturingGeometry::new(
        source_ids
            .iter()
            .cloned()
            .map(|id| ocs_cam_core::GeometrySource { id })
            .collect(),
    );
    if matches!(
        kind,
        ocs_cam_core::OperationKind::OutsideProfile
            | ocs_cam_core::OperationKind::InsideProfile
            | ocs_cam_core::OperationKind::Pocket
            | ocs_cam_core::OperationKind::Facing
    ) {
        let handles = source_ids
            .iter()
            .map(|source_id| {
                u64::from_str_radix(source_id.trim_start_matches("0x"), 16)
                    .map(Handle::new)
                    .map_err(|_| format!("invalid CAM source handle '{source_id}'"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let contours = closed_contours_from_selection(document, &handles)?;
        if kind != ocs_cam_core::OperationKind::Pocket && contours.len() != 1 {
            return Err("CAM operation requires one closed contour".to_string());
        }
        let mut contours = contours.into_iter();
        geometry.regions.push(ocs_cam_core::MachiningRegion {
            outer: contours.next().ok_or("CAM selection has no closed boundary")?,
            islands: if kind == ocs_cam_core::OperationKind::Pocket {
                contours.collect()
            } else {
                Vec::new()
            },
        });
        geometry.validate().map_err(|error| error.to_string())?;
        return Ok(geometry);
    }
    for source_id in source_ids {
        let value = u64::from_str_radix(source_id.trim_start_matches("0x"), 16)
            .map_err(|_| format!("invalid CAM source handle '{source_id}'"))?;
        let entity = document
            .get_entity(Handle::new(value))
            .ok_or_else(|| format!("CAM source entity {source_id} no longer exists"))?;
        match kind {
            ocs_cam_core::OperationKind::Drill => {
                geometry.drill_locations.push(ocs_cam_core::DrillLocation {
                    source_id: source_id.clone(),
                    point: drill_point_from_entity(entity).map_err(str::to_string)?,
                });
            }
            ocs_cam_core::OperationKind::Engrave | ocs_cam_core::OperationKind::Slot => {
                geometry.engraving_paths.push(ocs_cam_core::EngravingPath {
                    source_ids: vec![source_id.clone()],
                    contour: contour_from_entity(entity, false).map_err(str::to_string)?,
                });
            }
            _ => geometry.regions.push(ocs_cam_core::MachiningRegion {
                outer: contour_from_entity(entity, true).map_err(str::to_string)?,
                islands: Vec::new(),
            }),
        }
    }
    geometry.validate().map_err(|error| error.to_string())?;
    Ok(geometry)
}

fn first_stale_operation(document: &CadDocument, job: &ocs_cam_core::CamJob) -> Option<String> {
    job.operations.iter().find_map(|operation| {
        let expected = operation.geometry_fingerprint.as_ref()?;
        let current = manufacturing_geometry(document, operation.kind, &operation.source_ids)
            .and_then(|geometry| geometry.fingerprint().map_err(|error| error.to_string()));
        (current.as_ref() != Ok(expected)).then(|| operation.name.clone())
    })
}

fn closed_contours_from_selection(
    document: &CadDocument,
    handles: &[Handle],
) -> Result<Vec<Contour>, String> {
    let mut contours = Vec::<(usize, Contour)>::new();
    let mut lines = Vec::<(usize, ocs_cam_core::Point2, ocs_cam_core::Point2)>::new();
    for (order, handle) in handles.iter().enumerate() {
        let entity = document
            .get_entity(*handle)
            .ok_or_else(|| format!("CAM source entity {handle} no longer exists"))?;
        if let EntityType::Line(line) = entity {
            if line.start.z.abs() > 1.0e-6 || line.end.z.abs() > 1.0e-6 {
                return Err("connected CAM lines must lie in the world XY plane".to_string());
            }
            lines.push((
                order,
                ocs_cam_core::Point2::new(line.start.x, line.start.y),
                ocs_cam_core::Point2::new(line.end.x, line.end.y),
            ));
        } else {
            contours.push((
                order,
                contour_from_entity(entity, true).map_err(str::to_string)?,
            ));
        }
    }

    while !lines.is_empty() {
        let (first_order, start, mut current) = lines.remove(0);
        let mut order = first_order;
        let mut vertices = vec![ContourVertex::line(start.x, start.y)];
        while !cam_points_match(current, start) {
            vertices.push(ContourVertex::line(current.x, current.y));
            let Some(index) = lines.iter().position(|(_, a, b)| {
                cam_points_match(*a, current) || cam_points_match(*b, current)
            }) else {
                return Err(
                    "selected lines do not form a closed, endpoint-connected contour".to_string(),
                );
            };
            let (segment_order, a, b) = lines.remove(index);
            order = order.min(segment_order);
            current = if cam_points_match(a, current) { b } else { a };
            if vertices.len() > handles.len() + 1 {
                return Err("selected lines contain an ambiguous branch".to_string());
            }
        }
        if vertices.len() < 3 {
            return Err("a CAM boundary needs at least three connected line segments".to_string());
        }
        let contour = Contour {
            closed: true,
            vertices,
        };
        contour
            .validate_closed()
            .map_err(|error| error.to_string())?;
        contours.push((order, contour));
    }

    contours.sort_by_key(|(order, _)| *order);
    if contours.is_empty() {
        return Err("CAM selection has no closed planar contour".to_string());
    }
    Ok(contours.into_iter().map(|(_, contour)| contour).collect())
}

fn cam_points_match(a: ocs_cam_core::Point2, b: ocs_cam_core::Point2) -> bool {
    let scale = a
        .x
        .abs()
        .max(a.y.abs())
        .max(b.x.abs())
        .max(b.y.abs())
        .max(1.0);
    let tolerance = scale * 1.0e-8;
    (a.x - b.x).hypot(a.y - b.y) <= tolerance
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
        EntityType::Polyline2D(polyline) => {
            let closed = polyline.flags.is_closed();
            if require_closed && !closed {
                return Err("the selected 2D polyline is open");
            }
            if polyline.normal.z < 0.999_999
                || polyline.normal.x.abs() > 1.0e-6
                || polyline.normal.y.abs() > 1.0e-6
                || polyline.elevation.abs() > 1.0e-6
            {
                return Err("only contours in the world XY plane are supported");
            }
            Ok(Contour {
                closed,
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
        EntityType::Arc(_) | EntityType::Ellipse(_) | EntityType::Spline(_) => {
            let curve = crate::entities::curve::entity_curve(entity)
                .ok_or("the selected curve is not planar")?;
            let closed = curve.curve.is_closed();
            if require_closed && !closed {
                return Err("the selected curve is open");
            }
            let mut points = crate::entities::curve::curve_points(&curve);
            if points.iter().any(|point| point[2].abs() > 1.0e-6) {
                return Err("only curves in the world XY plane are supported");
            }
            if closed
                && points.len() > 1
                && points[0]
                    .iter()
                    .zip(points.last().unwrap())
                    .all(|(a, b)| (a - b).abs() <= 1.0e-9)
            {
                points.pop();
            }
            Ok(Contour {
                closed,
                vertices: points
                    .into_iter()
                    .map(|point| ContourVertex::line(point[0], point[1]))
                    .collect(),
            })
        }
        _ => Err("supported geometry is a planar polyline, circle, arc, ellipse, spline, or engraving line"),
    }
}

fn cam_operation_index(cmd: &str, count: usize) -> Option<usize> {
    cmd.split_whitespace()
        .nth(1)?
        .parse::<usize>()
        .ok()
        .filter(|index| *index > 0 && *index <= count)
        .map(|index| index - 1)
}

fn apply_setup_arguments(setup: &mut ocs_cam_core::CamSetup, cmd: &str) -> Result<(), String> {
    for argument in cmd.split_whitespace().skip(1) {
        let Some((key, value)) = argument.split_once('=') else {
            return Err(format!("expected key=value, got '{argument}'"));
        };
        match key.to_ascii_lowercase().as_str() {
            "stockw" => setup.stock.width = parse_number(key, value)?,
            "stockh" => setup.stock.height = parse_number(key, value)?,
            "thickness" => setup.stock.thickness = parse_number(key, value)?,
            "originx" => setup.work_origin.x = parse_number(key, value)?,
            "originy" => setup.work_origin.y = parse_number(key, value)?,
            "clearance" => setup.clearance_z = parse_number(key, value)?,
            "travelx" => setup.machine.travel_x = parse_number(key, value)?,
            "travely" => setup.machine.travel_y = parse_number(key, value)?,
            "travelz" => setup.machine.travel_z = parse_number(key, value)?,
            "maxfeed" => setup.machine.maximum_feed = parse_number(key, value)?,
            "maxrpm" => {
                setup.machine.maximum_spindle_rpm = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid maxrpm '{value}'"))?
            }
            "material" => setup.material.name = value.replace('_', " "),
            _ => return Err(format!("unknown setup parameter '{key}'")),
        }
    }
    Ok(())
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

pub(crate) fn regenerate_cam_operation(
    operation: &mut ocs_cam_core::CamOperation,
) -> Result<(), String> {
    let geometry = operation
        .geometry
        .as_ref()
        .ok_or_else(|| "operation has no manufacturing snapshot".to_string())?;
    let parameters = operation.parameters;
    let first_region = || {
        geometry
            .regions
            .first()
            .map(|region| &region.outer)
            .ok_or_else(|| "operation has no closed region".to_string())
    };
    let first_path = || {
        geometry
            .engraving_paths
            .first()
            .map(|path| &path.contour)
            .ok_or_else(|| "operation has no open path".to_string())
    };
    let program = match operation.kind {
        ocs_cam_core::OperationKind::OutsideProfile => {
            ocs_cam_core::profile_with_options(
                first_region()?,
                parameters,
                true,
                operation.advanced,
            )
        }
        ocs_cam_core::OperationKind::InsideProfile => {
            ocs_cam_core::profile_with_options(
                first_region()?,
                parameters,
                false,
                operation.advanced,
            )
        }
        ocs_cam_core::OperationKind::Pocket => ocs_cam_core::pocket_region(
            geometry
                .regions
                .first()
                .ok_or_else(|| "operation has no closed region".to_string())?,
            parameters,
            if operation.advanced.step_over > 0.0 { operation.advanced.step_over } else { parameters.tool_diameter * 0.5 },
            operation.advanced.preserve_pocket_islands,
        ),
        ocs_cam_core::OperationKind::Facing => ocs_cam_core::facing(
            contour_bounds(first_region()?),
            parameters,
            if operation.advanced.step_over > 0.0 { operation.advanced.step_over } else { parameters.tool_diameter * 0.5 },
        ),
        ocs_cam_core::OperationKind::Engrave => {
            ocs_cam_core::engrave(first_path()?, parameters)
        }
        ocs_cam_core::OperationKind::Drill => ocs_cam_core::drill_with_cycle(
            &geometry
                .drill_locations
                .iter()
                .map(|location| location.point)
                .collect::<Vec<_>>(),
            parameters,
            operation.advanced.drill_cycle,
        ),
        ocs_cam_core::OperationKind::Bore => {
            let bounds = contour_bounds(first_region()?);
            let center = ocs_cam_core::Point2::new(
                (bounds.min.x + bounds.max.x) * 0.5,
                (bounds.min.y + bounds.max.y) * 0.5,
            );
            ocs_cam_core::bore(center, bounds.max.x - bounds.min.x, parameters)
        }
        ocs_cam_core::OperationKind::Slot => {
            let path = first_path()?;
            let start = path.vertices.first().ok_or("slot path is empty")?.point;
            let end = path.vertices.last().ok_or("slot path is empty")?.point;
            ocs_cam_core::slot(
                start,
                end,
                if operation.advanced.slot_width > 0.0 { operation.advanced.slot_width } else { parameters.tool_diameter },
                parameters,
                if operation.advanced.step_over > 0.0 { operation.advanced.step_over } else { parameters.tool_diameter * 0.6 },
            )
        }
    }
    .map_err(|error| error.to_string())?;
    operation.program = program;
    operation.tool.diameter = operation.parameters.tool_diameter;
    operation.tool.feed = operation.parameters.feed;
    operation.tool.plunge_feed = operation.parameters.plunge_feed;
    operation.tool.spindle_rpm = operation.parameters.spindle_rpm;
    Ok(())
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
    fn connected_line_group_becomes_one_closed_cam_contour() {
        let mut document = CadDocument::default();
        let lines = [
            acadrust::entities::Line::from_coords(10.0, 0.0, 0.0, 5.0, 8.0, 0.0),
            acadrust::entities::Line::from_coords(0.0, 0.0, 0.0, 10.0, 0.0, 0.0),
            acadrust::entities::Line::from_coords(5.0, 8.0, 0.0, 0.0, 0.0, 0.0),
        ];
        let handles = lines
            .into_iter()
            .map(|line| document.add_entity(EntityType::Line(line)).unwrap())
            .collect::<Vec<_>>();
        let contours = closed_contours_from_selection(&document, &handles).unwrap();
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].vertices.len(), 3);
        assert!(contours[0].validate_closed().is_ok());

        let source_ids = handles.iter().map(|handle| format!("{handle}")).collect::<Vec<_>>();
        let geometry = manufacturing_geometry(
            &document,
            ocs_cam_core::OperationKind::InsideProfile,
            &source_ids,
        )
        .unwrap();
        assert_eq!(geometry.regions.len(), 1);
        assert_eq!(geometry.regions[0].outer.vertices.len(), 3);
    }

    #[test]
    fn open_line_group_reports_a_specific_cam_error() {
        let mut document = CadDocument::default();
        let handles = [
            acadrust::entities::Line::from_coords(0.0, 0.0, 0.0, 10.0, 0.0, 0.0),
            acadrust::entities::Line::from_coords(10.0, 0.0, 0.0, 5.0, 8.0, 0.0),
        ]
        .into_iter()
        .map(|line| document.add_entity(EntityType::Line(line)).unwrap())
        .collect::<Vec<_>>();
        let error = closed_contours_from_selection(&document, &handles).unwrap_err();
        assert!(error.contains("closed"), "{error}");
    }

    #[test]
    fn drill_adapter_uses_point_locations() {
        let point = EntityType::Point(acadrust::entities::Point::from_coords(3.0, 7.0, 0.0));
        assert_eq!(
            drill_point_from_entity(&point).unwrap(),
            ocs_cam_core::Point2::new(3.0, 7.0)
        );
    }

    #[test]
    fn unrelated_document_edits_do_not_change_a_source_snapshot() {
        let mut document = CadDocument::default();
        let mut circle = acadrust::entities::Circle::default();
        circle.radius = 8.0;
        let handle = document.add_entity(EntityType::Circle(circle)).unwrap();
        let source_ids = vec![format!("{handle}")];
        let before = manufacturing_geometry(
            &document,
            ocs_cam_core::OperationKind::OutsideProfile,
            &source_ids,
        )
        .unwrap()
        .fingerprint()
        .unwrap();

        document
            .add_entity(EntityType::Point(acadrust::entities::Point::from_coords(
                100.0, 100.0, 0.0,
            )))
            .unwrap();
        let after = manufacturing_geometry(
            &document,
            ocs_cam_core::OperationKind::OutsideProfile,
            &source_ids,
        )
        .unwrap()
        .fingerprint()
        .unwrap();
        assert_eq!(before, after);

        let EntityType::Circle(circle) = document.get_entity_mut(handle).unwrap() else {
            unreachable!()
        };
        circle.radius = 9.0;
        let changed = manufacturing_geometry(
            &document,
            ocs_cam_core::OperationKind::OutsideProfile,
            &source_ids,
        )
        .unwrap()
        .fingerprint()
        .unwrap();
        assert_ne!(before, changed);
    }

    #[test]
    fn arc_is_available_as_an_open_engraving_curve() {
        let mut arc = acadrust::entities::Arc::default();
        arc.radius = 5.0;
        arc.start_angle = 0.0;
        arc.end_angle = std::f64::consts::FRAC_PI_2;
        let entity = EntityType::Arc(arc);
        assert!(contour_from_entity(&entity, false).is_ok());
        assert!(contour_from_entity(&entity, true).is_err());
    }

    #[test]
    fn setup_arguments_and_operation_numbers_are_validated() {
        let mut setup = ocs_cam_core::CamSetup::default_for(Units::Millimeters);
        apply_setup_arguments(
            &mut setup,
            "CAMSETUP stockw=240 stockh=120 thickness=18 originx=5 material=Birch_Plywood maxrpm=18000",
        )
        .unwrap();
        assert_eq!(setup.stock.width, 240.0);
        assert_eq!(setup.material.name, "Birch Plywood");
        assert_eq!(setup.machine.maximum_spindle_rpm, 18_000);
        assert_eq!(cam_operation_index("CAMDELETE 2", 3), Some(1));
        assert_eq!(cam_operation_index("CAMDELETE 4", 3), None);
    }
}
