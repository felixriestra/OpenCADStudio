use crate::app::Message;
use crate::ui::dock::{DockMsg, PanelId};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Fill, Length};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SetupField {
    StockWidth,
    StockHeight,
    StockThickness,
    Clearance,
    OriginX,
    OriginY,
    TravelX,
    TravelY,
    TravelZ,
    MaximumFeed,
    MaximumRpm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolField {
    Diameter,
    Feed,
    Plunge,
    Rpm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationField {
    Depth,
    StepDown,
    StepOver,
    SlotWidth,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdvancedField {
    Tabs,
    TabHeight,
    LeadIn,
    LeadOut,
    RampLength,
    FinishAllowance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumericField {
    Setup(SetupField),
    Tool(ToolField),
    Operation(OperationField),
    Advanced(AdvancedField),
}
#[derive(Debug, Clone, Copy)]
pub enum SetupTemplate {
    Small,
    Medium,
    DesktopRouter,
}

#[derive(Debug, Default)]
pub struct CamEditorState {
    pub(crate) drafts: BTreeMap<NumericField, String>,
    pub template_name: String,
    pub material_name: String,
    pub tool_name: String,
    pub tool_search: String,
    pub tool_trash: bool,
}
impl CamEditorState {
    pub fn edit(&mut self, field: NumericField, value: String) {
        self.drafts.insert(field, value);
    }
    pub fn clear_operation(&mut self) {
        self.drafts
            .retain(|field, _| matches!(field, NumericField::Setup(_)));
    }
    fn value(&self, field: NumericField, value: f64) -> String {
        self.drafts
            .get(&field)
            .cloned()
            .unwrap_or_else(|| format_number(value))
    }
}

#[derive(Debug, Clone)]
pub enum CamPanelMsg {
    Select(usize),
    Toggle(usize),
    MoveUp(usize),
    MoveDown(usize),
    Duplicate(usize),
    Delete(usize),
    Regenerate(usize),
    PreviewAll,
    PreviewSelected,
    SimulateSelected,
    ClearPreview,
    PreviewFirst,
    PreviewPrevious,
    PreviewNext,
    PreviewLast,
    TogglePlayback,
    ImportGcode,
    PasteGcode,
    Open3dPreview,
    Slower,
    Faster,
    TemplateName(String),
    TemplateSelect(usize),
    TemplateSave,
    TemplateUpdate,
    TemplateDelete,
    MaterialName(String),
    MaterialSelect(usize),
    MaterialCreate,
    MaterialUpdate,
    MaterialDelete,
    ToolName(String),
    ToolSelect(usize),
    ToolCreate,
    ToolUpdate,
    ToolDelete,
    ToolDuplicate,
    ToolSearch(String),
    ToolValue(ToolField, String),
    ToolToggleTrash,
    ToolRestore(usize),
    ToolPurge(usize),
    ToolImportCsv,
    ToolImportCommit,
    ToolImportCancel,
    ToolApplyResolved,
    ToolSavePreset,
    EditNumber(NumericField, String),
    ApplySetupTemplate(SetupTemplate),
    ApplyLibraryTool(usize),
    #[doc(hidden)]
    AdjustSetup(SetupField, f64),
    #[doc(hidden)]
    AdjustTool(ToolField, f64),
    #[doc(hidden)]
    AdjustAdvanced(AdvancedField, f64),
    ToggleFinishPass,
    TogglePocketIslands,
    ToggleDrillCycle,
    CycleMaterial,
}

pub fn operations_view<'a>(
    job: &'a ocs_cam_core::CamJob,
    selected: Option<usize>,
    editor: &'a CamEditorState,
    preview_len: usize,
    preview_step: Option<usize>,
    gcode_lines: &'a [String],
    active_gcode_line: Option<usize>,
    playing: bool,
    playback_speed: f64,
    saved_tools: &'a [ocs_cam_core::ToolDefinition],
    trashed_tools: &'a [ocs_cam_core::ToolDefinition],
    selected_tool: Option<usize>,
    width: f32,
    auto_collapse: bool,
) -> Element<'a, Message> {
    let mut operations = column![text("Operations").size(14)].spacing(5);
    if job.operations.is_empty() {
        operations = operations.push(text(
            "Select geometry and choose a toolpath in the CAM ribbon.",
        ));
    }
    for (index, operation) in job.operations.iter().enumerate() {
        let active = selected == Some(index);
        let title = format!(
            "{} {}  {}",
            if operation.enabled { "●" } else { "○" },
            index + 1,
            operation.name
        );
        operations = operations.push(
            container(
                column![
                    button(text(if active { format!("> {title}") } else { title }))
                        .width(Fill)
                        .on_press(Message::CamPanel(CamPanelMsg::Select(index))),
                    row![
                        button("↑").on_press(Message::CamPanel(CamPanelMsg::MoveUp(index))),
                        button("↓").on_press(Message::CamPanel(CamPanelMsg::MoveDown(index))),
                        button(if operation.enabled {
                            "Disable"
                        } else {
                            "Enable"
                        })
                        .on_press(Message::CamPanel(CamPanelMsg::Toggle(index))),
                        button("Copy").on_press(Message::CamPanel(CamPanelMsg::Duplicate(index))),
                        button("Regen").on_press(Message::CamPanel(CamPanelMsg::Regenerate(index))),
                        button("Delete").on_press(Message::CamPanel(CamPanelMsg::Delete(index))),
                    ]
                    .spacing(3),
                ]
                .spacing(3),
            )
            .padding(5)
            .width(Fill),
        );
    }

    let parameters: Element<'_, Message> = selected
        .and_then(|index| job.operations.get(index))
        .map(|operation| {
            let advanced = operation.advanced;
            let mut fields = column![
                text(format!("{:?} parameters", operation.kind)).size(14),
                number_row(
                    "Depth (total)",
                    NumericField::Operation(OperationField::Depth),
                    operation.parameters.depth,
                    editor
                ),
                number_row(
                    "Step-down (per pass)",
                    NumericField::Operation(OperationField::StepDown),
                    operation.parameters.step_down,
                    editor
                ),
            ]
            .spacing(4);
            if matches!(
                operation.kind,
                ocs_cam_core::OperationKind::Pocket
                    | ocs_cam_core::OperationKind::Facing
                    | ocs_cam_core::OperationKind::Slot
            ) {
                let default_percent = if operation.kind == ocs_cam_core::OperationKind::Slot {
                    60.0
                } else {
                    50.0
                };
                let step_over_percent = if advanced.step_over > 0.0 {
                    advanced.step_over / operation.parameters.tool_diameter * 100.0
                } else {
                    default_percent
                };
                fields = fields.push(number_row(
                    "Stepover (%)",
                    NumericField::Operation(OperationField::StepOver),
                    step_over_percent,
                    editor,
                ));
            }
            if operation.kind == ocs_cam_core::OperationKind::Slot {
                fields = fields.push(number_row(
                    "Slot width",
                    NumericField::Operation(OperationField::SlotWidth),
                    advanced.slot_width,
                    editor,
                ));
            }
            if matches!(
                operation.kind,
                ocs_cam_core::OperationKind::OutsideProfile
                    | ocs_cam_core::OperationKind::InsideProfile
            ) {
                fields = fields
                    .push(number_row(
                        "Tabs",
                        NumericField::Advanced(AdvancedField::Tabs),
                        advanced.tab_count as f64,
                        editor,
                    ))
                    .push(number_row(
                        "Tab height",
                        NumericField::Advanced(AdvancedField::TabHeight),
                        advanced.tab_height,
                        editor,
                    ));
            }
            fields = fields
                .push(number_row(
                    "Lead in",
                    NumericField::Advanced(AdvancedField::LeadIn),
                    advanced.lead_in,
                    editor,
                ))
                .push(number_row(
                    "Lead out",
                    NumericField::Advanced(AdvancedField::LeadOut),
                    advanced.lead_out,
                    editor,
                ))
                .push(number_row(
                    "Ramp length",
                    NumericField::Advanced(AdvancedField::RampLength),
                    advanced.ramp_length,
                    editor,
                ))
                .push(number_row(
                    "Finish allowance",
                    NumericField::Advanced(AdvancedField::FinishAllowance),
                    advanced.finish_allowance,
                    editor,
                ))
                .push(
                    button(if advanced.finish_pass {
                        "Finish pass: On"
                    } else {
                        "Finish pass: Off"
                    })
                    .on_press(Message::CamPanel(CamPanelMsg::ToggleFinishPass)),
                );
            if operation.kind == ocs_cam_core::OperationKind::Pocket {
                fields = fields.push(
                    button(if advanced.preserve_pocket_islands {
                        "Pocket islands: Preserve"
                    } else {
                        "Pocket islands: Ignore"
                    })
                    .on_press(Message::CamPanel(CamPanelMsg::TogglePocketIslands)),
                );
            }
            if operation.kind == ocs_cam_core::OperationKind::Drill {
                fields = fields.push(
                    button(match advanced.drill_cycle {
                        ocs_cam_core::DrillCycle::Simple => "Drill cycle: Simple",
                        ocs_cam_core::DrillCycle::Peck => "Drill cycle: Peck",
                    })
                    .on_press(Message::CamPanel(CamPanelMsg::ToggleDrillCycle)),
                );
            }
            fields.into()
        })
        .unwrap_or_else(|| text("Select an operation to edit its machining parameters.").into());

    let tool: Element<'_, Message> = selected
        .and_then(|index| job.operations.get(index))
        .map(|operation| {
            column![
                text("Tool for this operation").size(14),
                number_row(
                    "Diameter",
                    NumericField::Tool(ToolField::Diameter),
                    operation.tool.diameter,
                    editor
                ),
                number_row(
                    "Feed",
                    NumericField::Tool(ToolField::Feed),
                    operation.tool.feed,
                    editor
                ),
                number_row(
                    "Plunge",
                    NumericField::Tool(ToolField::Plunge),
                    operation.tool.plunge_feed,
                    editor
                ),
                number_row(
                    "RPM",
                    NumericField::Tool(ToolField::Rpm),
                    operation.tool.spindle_rpm as f64,
                    editor
                ),
            ]
            .spacing(4)
            .into()
        })
        .unwrap_or_else(|| text("Select an operation to edit its tool.").into());

    let mut library = column![text("Tool Library").size(14)].spacing(4);
    for (index, tool) in job.tool_library.iter().enumerate() {
        library = library.push(
            row![
                text(format!(
                    "{}  Ø{:.2}  {:.0} rpm",
                    tool.name, tool.diameter, tool.spindle_rpm
                ))
                .width(Fill),
                button("Use").on_press(Message::CamPanel(CamPanelMsg::ApplyLibraryTool(index)))
            ]
            .spacing(4),
        );
    }
    let body = column![
        panel_header("CAM Operations", PanelId::Cam, auto_collapse),
        row![
            button("Import G-code").on_press(Message::CamPanel(CamPanelMsg::ImportGcode)),
            button("Paste G-code").on_press(Message::CamPanel(CamPanelMsg::PasteGcode)),
        ]
        .spacing(4),
        row![
            button("Preview All").on_press(Message::CamPanel(CamPanelMsg::PreviewAll)),
            button("2D Selected").on_press(Message::CamPanel(CamPanelMsg::PreviewSelected)),
            button("3D Stock").on_press(Message::CamPanel(CamPanelMsg::SimulateSelected)),
            button("3D Window").on_press(Message::CamPanel(CamPanelMsg::Open3dPreview)),
            button("Clear").on_press(Message::CamPanel(CamPanelMsg::ClearPreview)),
        ]
        .spacing(4),
        row![
            button("|◀").on_press(Message::CamPanel(CamPanelMsg::PreviewFirst)),
            button("◀").on_press(Message::CamPanel(CamPanelMsg::PreviewPrevious)),
            button(if playing { "Pause" } else { "Play" })
                .on_press(Message::CamPanel(CamPanelMsg::TogglePlayback)),
            button("−").on_press(Message::CamPanel(CamPanelMsg::Slower)),
            text(format!("{playback_speed}×")),
            button("+").on_press(Message::CamPanel(CamPanelMsg::Faster)),
            text(preview_step.map_or_else(
                || format!("All {preview_len}"),
                |step| format!("{step}/{preview_len}")
            ))
            .width(Fill),
            button("▶").on_press(Message::CamPanel(CamPanelMsg::PreviewNext)),
            button("▶|").on_press(Message::CamPanel(CamPanelMsg::PreviewLast)),
        ]
        .spacing(4),
        gcode_view(gcode_lines, active_gcode_line),
        operations,
        parameters,
        tool,
        library,
        row![
            text("Tool Database").size(14).width(Fill),
            button(if editor.tool_trash { "Tools" } else { "Trash" })
                .on_press(Message::CamPanel(CamPanelMsg::ToolToggleTrash)),
        ],
        text_input("Search tools", &editor.tool_search)
            .on_input(|value| Message::CamPanel(CamPanelMsg::ToolSearch(value))),
        if editor.tool_trash {
            trashed_tools
                .iter()
                .enumerate()
                .fold(column![].spacing(2), |column, (index, tool)| {
                    column.push(
                        row![
                            text(format!("{}  Ø{:.2}", tool.name, tool.diameter)).width(Fill),
                            button("Restore")
                                .on_press(Message::CamPanel(CamPanelMsg::ToolRestore(index))),
                            button("Delete")
                                .on_press(Message::CamPanel(CamPanelMsg::ToolPurge(index))),
                        ]
                        .spacing(3),
                    )
                })
        } else {
            saved_tools
                .iter()
                .enumerate()
                .filter(|(_, tool)| {
                    editor.tool_search.trim().is_empty()
                        || tool
                            .name
                            .to_lowercase()
                            .contains(&editor.tool_search.trim().to_lowercase())
                })
                .fold(column![].spacing(2), |column, (index, tool)| {
                    column.push(
                        button(text(if selected_tool == Some(index) {
                            format!("> {}  Ø{:.2}", tool.name, tool.diameter)
                        } else {
                            format!("{}  Ø{:.2}", tool.name, tool.diameter)
                        }))
                        .width(Fill)
                        .on_press(Message::CamPanel(CamPanelMsg::ToolSelect(index))),
                    )
                })
        },
        text_input("Tool name", &editor.tool_name)
            .on_input(|value| Message::CamPanel(CamPanelMsg::ToolName(value))),
        row![
            button("Create").on_press(Message::CamPanel(CamPanelMsg::ToolCreate)),
            button("Save edits").on_press(Message::CamPanel(CamPanelMsg::ToolUpdate)),
            button("Duplicate").on_press(Message::CamPanel(CamPanelMsg::ToolDuplicate)),
            button("To Trash").on_press(Message::CamPanel(CamPanelMsg::ToolDelete)),
        ]
        .spacing(4),
    ]
    .spacing(10)
    .padding(iced::Padding {
        top: 8.0,
        right: 20.0,
        bottom: 8.0,
        left: 8.0,
    });
    container(scrollable(body).height(Fill))
        .width(Length::Fixed(width))
        .height(Fill)
        .into()
}

fn gcode_view<'a>(lines: &'a [String], active: Option<usize>) -> iced::widget::Column<'a, Message> {
    if lines.is_empty() {
        return column![text("No G-code loaded or generated.")];
    }
    let start = active.unwrap_or(1).saturating_sub(8);
    let end = (start + 18).min(lines.len());
    let mut code = column![text("G-code").size(14)].spacing(1);
    for (offset, line) in lines[start..end].iter().enumerate() {
        let number = start + offset + 1;
        code = code.push(
            text(format!(
                "{} {:04}  {}",
                if Some(number) == active { ">" } else { " " },
                number,
                line
            ))
            .size(11),
        );
    }
    code
}

pub fn setup_view<'a>(
    job: &'a ocs_cam_core::CamJob,
    editor: &'a CamEditorState,
    templates_saved: &'a [ocs_cam_core::SetupTemplate],
    selected_template: Option<usize>,
    materials: &'a [ocs_cam_core::MaterialPreset],
    selected_material: Option<usize>,
    width: f32,
    auto_collapse: bool,
) -> Element<'a, Message> {
    let templates = column![
        text("Stock templates").size(14),
        row![
            button("100 × 75 × 20").on_press(Message::CamPanel(CamPanelMsg::ApplySetupTemplate(
                SetupTemplate::Small
            ))),
            button("300 × 200 × 25").on_press(Message::CamPanel(CamPanelMsg::ApplySetupTemplate(
                SetupTemplate::Medium
            ))),
        ]
        .spacing(4),
        button("Desktop router 300 × 180 × 18").on_press(Message::CamPanel(
            CamPanelMsg::ApplySetupTemplate(SetupTemplate::DesktopRouter)
        )),
        text("My templates").size(14),
        template_rows(templates_saved, selected_template),
        text_input("Template name", &editor.template_name)
            .on_input(|value| Message::CamPanel(CamPanelMsg::TemplateName(value))),
        row![
            button("Save new").on_press(Message::CamPanel(CamPanelMsg::TemplateSave)),
            button("Update").on_press(Message::CamPanel(CamPanelMsg::TemplateUpdate)),
            button("Delete").on_press(Message::CamPanel(CamPanelMsg::TemplateDelete)),
        ]
        .spacing(4),
    ]
    .spacing(4);
    let setup: Element<'_, Message> = job
        .setups
        .first()
        .map(|setup| {
            column![
                text(format!("Job units: {:?}", job.units)),
                text("Stock").size(14),
                setup_number("Width", SetupField::StockWidth, setup.stock.width, editor),
                setup_number(
                    "Height",
                    SetupField::StockHeight,
                    setup.stock.height,
                    editor
                ),
                setup_number(
                    "Thickness",
                    SetupField::StockThickness,
                    setup.stock.thickness,
                    editor
                ),
                setup_number(
                    "Clearance Z",
                    SetupField::Clearance,
                    setup.clearance_z,
                    editor
                ),
                setup_number("Origin X", SetupField::OriginX, setup.work_origin.x, editor),
                setup_number("Origin Y", SetupField::OriginY, setup.work_origin.y, editor),
                button(text(format!("Material: {}", setup.material.name)))
                    .on_press(Message::CamPanel(CamPanelMsg::CycleMaterial)),
                text("Material library").size(14),
                material_rows(materials, selected_material),
                text_input("Material name", &editor.material_name)
                    .on_input(|value| Message::CamPanel(CamPanelMsg::MaterialName(value))),
                row![
                    button("Create").on_press(Message::CamPanel(CamPanelMsg::MaterialCreate)),
                    button("Update").on_press(Message::CamPanel(CamPanelMsg::MaterialUpdate)),
                    button("Delete").on_press(Message::CamPanel(CamPanelMsg::MaterialDelete)),
                ]
                .spacing(4),
                text("Machine limits").size(14),
                setup_number(
                    "Travel X",
                    SetupField::TravelX,
                    setup.machine.travel_x,
                    editor
                ),
                setup_number(
                    "Travel Y",
                    SetupField::TravelY,
                    setup.machine.travel_y,
                    editor
                ),
                setup_number(
                    "Travel Z",
                    SetupField::TravelZ,
                    setup.machine.travel_z,
                    editor
                ),
                setup_number(
                    "Maximum feed",
                    SetupField::MaximumFeed,
                    setup.machine.maximum_feed,
                    editor
                ),
                setup_number(
                    "Maximum RPM",
                    SetupField::MaximumRpm,
                    setup.machine.maximum_spindle_rpm as f64,
                    editor
                ),
                text("This setup is shared by every CAM operation in the job."),
            ]
            .spacing(4)
            .into()
        })
        .unwrap_or_else(|| text("No job setup is available.").into());
    let body = column![
        panel_header("Job Setup", PanelId::CamSetup, auto_collapse),
        templates,
        setup
    ]
    .spacing(10)
    .padding(iced::Padding {
        top: 8.0,
        right: 20.0,
        bottom: 8.0,
        left: 8.0,
    });
    container(scrollable(body).height(Fill))
        .width(Length::Fixed(width))
        .height(Fill)
        .into()
}

fn template_rows<'a>(
    items: &'a [ocs_cam_core::SetupTemplate],
    selected: Option<usize>,
) -> iced::widget::Column<'a, Message> {
    items
        .iter()
        .enumerate()
        .fold(column![].spacing(2), |column, (index, item)| {
            column.push(
                button(text(if selected == Some(index) {
                    format!("> {}", item.name)
                } else {
                    item.name.clone()
                }))
                .width(Fill)
                .on_press(Message::CamPanel(CamPanelMsg::TemplateSelect(index))),
            )
        })
}

fn material_rows<'a>(
    items: &'a [ocs_cam_core::MaterialPreset],
    selected: Option<usize>,
) -> iced::widget::Column<'a, Message> {
    items
        .iter()
        .enumerate()
        .fold(column![].spacing(2), |column, (index, item)| {
            column.push(
                button(text(if selected == Some(index) {
                    format!("> {}", item.name)
                } else {
                    item.name.clone()
                }))
                .width(Fill)
                .on_press(Message::CamPanel(CamPanelMsg::MaterialSelect(index))),
            )
        })
}

/// Dedicated application-level tool database window. CAM operations only keep
/// a copy of the selected cutter; management, search and Trash live here.
pub fn tool_library_view<'a>(
    editor: &'a CamEditorState,
    tools: &'a [ocs_cam_core::ToolDefinition],
    trash: &'a [ocs_cam_core::ToolDefinition],
    selected: Option<usize>,
    setup: Option<&'a ocs_cam_core::CamSetup>,
    resolved: Option<crate::app::cam_library::ResolvedCuttingData>,
    import_plan: Option<&'a crate::app::cam_library::ToolImportPlan>,
) -> Element<'a, Message> {
    let query = editor.tool_search.trim().to_lowercase();
    let list: Element<'a, Message> = if editor.tool_trash {
        trash
            .iter()
            .enumerate()
            .fold(column![].spacing(5), |column, (index, tool)| {
                column.push(
                    row![
                        text(format!(
                            "{}   Ø{:.3}   {} rpm",
                            tool.name, tool.diameter, tool.spindle_rpm
                        ))
                        .width(Fill),
                        button("Restore")
                            .on_press(Message::CamPanel(CamPanelMsg::ToolRestore(index))),
                        button("Delete permanently")
                            .on_press(Message::CamPanel(CamPanelMsg::ToolPurge(index))),
                    ]
                    .spacing(6),
                )
            })
            .into()
    } else {
        tools
            .iter()
            .enumerate()
            .filter(|(_, tool)| query.is_empty() || tool.name.to_lowercase().contains(&query))
            .fold(column![].spacing(5), |column, (index, tool)| {
                column.push(
                    button(text(format!(
                        "{}{}   Ø{:.3}   feed {:.0}   plunge {:.0}   {} rpm",
                        if selected == Some(index) { "> " } else { "" },
                        tool.name,
                        tool.diameter,
                        tool.feed,
                        tool.plunge_feed,
                        tool.spindle_rpm
                    )))
                    .width(Fill)
                    .on_press(Message::CamPanel(CamPanelMsg::ToolSelect(index))),
                )
            })
            .into()
    };
    let cutting_data: Element<'a, Message> = match (setup, resolved) {
        (Some(setup), Some(data)) => column![
            text(format!(
                "Cutting data for {} / current machine",
                setup.material.name
            ))
            .size(15),
            text(format!(
                "Feed {:.0} · Plunge {:.0} · {} rpm · Stepdown {:.3} · Stepover {:.3}",
                data.feed, data.plunge_feed, data.spindle_rpm, data.step_down, data.step_over
            )),
            text(format!(
                "{}{}",
                data.source,
                if data.estimated {
                    " — estimate; verify before cutting"
                } else {
                    " — saved preset"
                }
            ))
            .size(12),
            row![
                button("Apply to operation")
                    .on_press(Message::CamPanel(CamPanelMsg::ToolApplyResolved)),
                button("Save preset").on_press(Message::CamPanel(CamPanelMsg::ToolSavePreset)),
            ]
            .spacing(8),
        ]
        .spacing(5)
        .into(),
        _ => {
            text("Select a tool to resolve cutting data for the current job material and machine.")
                .into()
        }
    };
    let import_review: Element<'a, Message> = if let Some(plan) = import_plan {
        column![
            text(format!("Import review — {}", plan.source_name)).size(15),
            text(format!(
                "{} valid tools · {} rejected rows",
                plan.tools.len(),
                plan.rejected.len()
            )),
            text(
                plan.rejected
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            )
            .size(12),
            row![
                button(text(format!("Import {} tools", plan.tools.len())))
                    .on_press(Message::CamPanel(CamPanelMsg::ToolImportCommit)),
                button("Cancel").on_press(Message::CamPanel(CamPanelMsg::ToolImportCancel)),
            ]
            .spacing(8),
        ]
        .spacing(5)
        .into()
    } else {
        text("").into()
    };
    let body = column![
        row![
            text("Tool Database").size(22).width(Fill),
            button(if editor.tool_trash { "Back to Tools" } else { "Trash" })
                .on_press(Message::CamPanel(CamPanelMsg::ToolToggleTrash)),
        ].align_y(iced::Center),
        text("Reusable cutters are stored at application level. Selecting one copies it into the current CAM operation."),
        text_input("Search by tool name", &editor.tool_search)
            .on_input(|value| Message::CamPanel(CamPanelMsg::ToolSearch(value))),
        scrollable(list).height(Fill),
        cutting_data,
        import_review,
        text_input("Tool name", &editor.tool_name)
            .on_input(|value| Message::CamPanel(CamPanelMsg::ToolName(value))),
        row![
            library_number("Diameter", ToolField::Diameter, selected.and_then(|i| tools.get(i)).map(|t| t.diameter).unwrap_or(6.0), editor),
            library_number("Feed", ToolField::Feed, selected.and_then(|i| tools.get(i)).map(|t| t.feed).unwrap_or(800.0), editor),
        ].spacing(8),
        row![
            library_number("Plunge", ToolField::Plunge, selected.and_then(|i| tools.get(i)).map(|t| t.plunge_feed).unwrap_or(250.0), editor),
            library_number("RPM", ToolField::Rpm, selected.and_then(|i| tools.get(i)).map(|t| t.spindle_rpm as f64).unwrap_or(18_000.0), editor),
        ].spacing(8),
        row![
            button("New from operation").on_press(Message::CamPanel(CamPanelMsg::ToolCreate)),
            button("Save edits").on_press(Message::CamPanel(CamPanelMsg::ToolUpdate)),
            button("Duplicate").on_press(Message::CamPanel(CamPanelMsg::ToolDuplicate)),
            button("Move to Trash").on_press(Message::CamPanel(CamPanelMsg::ToolDelete)),
            button("Import CSV").on_press(Message::CamPanel(CamPanelMsg::ToolImportCsv)),
        ].spacing(8),
        text("Storage is updated atomically and the previous database snapshot is retained as a backup.").size(12),
    ].spacing(12).padding(16);
    container(body).width(Fill).height(Fill).into()
}

fn library_number<'a>(
    label: &'static str,
    field: ToolField,
    value: f64,
    editor: &'a CamEditorState,
) -> Element<'a, Message> {
    row![
        text(label).width(Length::Fixed(70.0)),
        text_input("", &editor.value(NumericField::Tool(field), value))
            .on_input(move |input| Message::CamPanel(CamPanelMsg::ToolValue(field, input)))
            .width(Fill),
    ]
    .align_y(iced::Center)
    .into()
}

fn panel_header<'a>(
    title: &'static str,
    panel: PanelId,
    auto_collapse: bool,
) -> Element<'a, Message> {
    row![
        text(title).size(16).width(Fill),
        button(if auto_collapse { "Unpin" } else { "Pin" })
            .on_press(Message::Dock(DockMsg::AutoCollapseToggle(panel))),
        button("×").on_press(Message::Dock(DockMsg::Close(panel)))
    ]
    .spacing(6)
    .into()
}
fn setup_number<'a>(
    label: &'static str,
    field: SetupField,
    value: f64,
    editor: &'a CamEditorState,
) -> Element<'a, Message> {
    number_row(label, NumericField::Setup(field), value, editor)
}
fn number_row<'a>(
    label: &'static str,
    field: NumericField,
    value: f64,
    editor: &'a CamEditorState,
) -> Element<'a, Message> {
    row![
        text(label).width(Fill),
        text_input("0", &editor.value(field, value))
            .on_input(move |value| Message::CamPanel(CamPanelMsg::EditNumber(field, value)))
            .width(Length::Fixed(105.0))
    ]
    .spacing(6)
    .into()
}
fn format_number(value: f64) -> String {
    if value.fract().abs() < 1.0e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.3}").trim_end_matches('0').to_string()
    }
}
