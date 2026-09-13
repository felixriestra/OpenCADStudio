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
/// The current CAM operation's own tool-parameter overrides (diameter, feed,
/// plunge, RPM baked into `operation.parameters`) — distinct from
/// `LibraryToolField`, which edits a catalog `LibraryTool`'s geometry in the
/// Tool Database window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolField {
    Diameter,
    Feed,
    Plunge,
    Rpm,
}

/// Geometry fields editable on a catalog `LibraryTool` in the Tool Database
/// window. No feed/plunge/RPM here — that cutting data lives in presets,
/// resolved against a material and machine, not stored on the tool itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibraryToolField {
    Diameter,
    FluteCount,
    FluteLength,
    ShankDia,
    OverallLength,
    CornerRadius,
    /// Only meaningful for `ToolType::requires_included_angle()` types
    /// (V-bit, engraver, tapered ball).
    IncludedAngle,
    /// Flat diameter at the tip — engraver/chamfer/tapered-ball types.
    TipDiameter,
}

/// A fixed-vocabulary field backed by a `CHECK`-constrained TEXT column
/// (`schema.rs`'s `tool.substrate`/`tool.chip_direction`), rendered as a
/// `pick_list` instead of free text so it can never drift from the values
/// the database actually accepts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SubstrateOption {
    #[default]
    None,
    SolidCarbide,
    CarbideTipped,
    Hss,
    Diamond,
    Insert,
}

impl SubstrateOption {
    pub const ALL: [SubstrateOption; 6] = [
        Self::None,
        Self::SolidCarbide,
        Self::CarbideTipped,
        Self::Hss,
        Self::Diamond,
        Self::Insert,
    ];

    pub fn as_db_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::SolidCarbide => Some("solid_carbide"),
            Self::CarbideTipped => Some("carbide_tipped"),
            Self::Hss => Some("hss"),
            Self::Diamond => Some("diamond"),
            Self::Insert => Some("insert"),
        }
    }

    pub fn from_db_str(value: Option<&str>) -> Self {
        match value {
            Some("solid_carbide") => Self::SolidCarbide,
            Some("carbide_tipped") => Self::CarbideTipped,
            Some("hss") => Self::Hss,
            Some("diamond") => Self::Diamond,
            Some("insert") => Self::Insert,
            _ => Self::None,
        }
    }
}

impl std::fmt::Display for SubstrateOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "—",
            Self::SolidCarbide => "Solid carbide",
            Self::CarbideTipped => "Carbide tipped",
            Self::Hss => "HSS",
            Self::Diamond => "Diamond",
            Self::Insert => "Insert",
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ChipDirectionOption {
    #[default]
    None,
    Up,
    Down,
    Compression,
    Straight,
}

impl ChipDirectionOption {
    pub const ALL: [ChipDirectionOption; 5] =
        [Self::None, Self::Up, Self::Down, Self::Compression, Self::Straight];

    pub fn as_db_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Up => Some("up"),
            Self::Down => Some("down"),
            Self::Compression => Some("compression"),
            Self::Straight => Some("straight"),
        }
    }

    pub fn from_db_str(value: Option<&str>) -> Self {
        match value {
            Some("up") => Self::Up,
            Some("down") => Self::Down,
            Some("compression") => Self::Compression,
            Some("straight") => Self::Straight,
            _ => Self::None,
        }
    }
}

impl std::fmt::Display for ChipDirectionOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "—",
            Self::Up => "Upcut",
            Self::Down => "Downcut",
            Self::Compression => "Compression",
            Self::Straight => "Straight",
        })
    }
}

/// Sidebar vendor filter: "All vendors" plus whatever vendor names are
/// actually present in the catalog (`ToolLibraryStore::vendors()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorOption {
    All,
    Named(String),
}

impl std::fmt::Display for VendorOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => f.write_str("All vendors"),
            Self::Named(name) => f.write_str(name),
        }
    }
}

/// Sidebar type filter: "All types" plus whatever `ToolType`s are actually
/// present in the catalog (`ToolLibraryStore::tool_types_present()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeOption {
    All,
    Specific(crate::tool_library::model::ToolType),
}

impl std::fmt::Display for TypeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => f.write_str("All types"),
            Self::Specific(t) => f.write_str(t.display_name()),
        }
    }
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
    LibraryTool(LibraryToolField),
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
    // Identity / Material & finish drafts — same "seeded on Select, diffed
    // and applied on Update" pattern as `tool_name` above.
    pub tool_vendor: String,
    pub tool_part_number: String,
    pub tool_series: String,
    pub tool_type: crate::tool_library::model::ToolType,
    pub tool_substrate: SubstrateOption,
    pub tool_coating: String,
    pub tool_chip_direction: ChipDirectionOption,
    pub tool_notes: String,
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
    ToolValue(LibraryToolField, String),
    ToolVendor(String),
    ToolPartNumber(String),
    ToolSeries(String),
    ToolTypeSelect(crate::tool_library::model::ToolType),
    ToolSubstrateSelect(SubstrateOption),
    ToolCoating(String),
    ToolChipDirectionSelect(ChipDirectionOption),
    ToolNotes(String),
    ToolToggleTrash,
    ToolRestore(usize),
    ToolPurge(usize),
    ToolImportCsv,
    ToolExportCsv,
    ToolImportCommit,
    ToolImportCancel,
    ToolApplyResolved,
    ToolSavePreset,
    MaterialClassSelect(String),
    DepthOfCutChanged(f64),
    VendorFilterSelect(VendorOption),
    TypeFilterSelect(TypeOption),
    UnitToggle(crate::tool_library::DisplayUnit),
    IncompleteOnlyToggle(bool),
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
        text("Manage the shared tool catalog — create, edit, import, trash — from Manage > CAMTOOLS, its own floating window.").size(11),
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
/// Backed by `crate::tool_library` — a real embedded SQLite catalog (ported
/// from 2DCam's tool database) rather than the earlier flat JSON list.
fn icon_button<'a>(bytes: &'static [u8], tooltip: &'a str, message: Message) -> Element<'a, Message> {
    let icon = iced::widget::svg(iced::widget::svg::Handle::from_memory(bytes))
        .width(18)
        .height(18);
    iced::widget::tooltip(
        button(icon).padding(6).on_press(message),
        text(tooltip).size(11),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

fn section_title<'a>(label: impl Into<String>) -> Element<'a, Message> {
    text(label.into()).size(14).into()
}

/// A read-only label/value row for the Cutting Data grid.
fn data_row<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    row![text(label).width(Length::Fixed(90.0)).size(12), text(value).size(12)]
        .spacing(8)
        .into()
}

pub fn tool_library_view<'a>(
    editor: &'a CamEditorState,
    library: &'a crate::tool_library::ToolLibraryStore,
    selected: Option<usize>,
    resolved: Option<crate::tool_library::resolver::ResolvedCuttingData>,
) -> Element<'a, Message> {
    use crate::tool_library::model::ToolType;
    use crate::tool_library::DisplayUnit;

    let tools = &library.tools;
    let trash = &library.trash;
    let query = editor.tool_search.trim().to_lowercase();
    let unit = library.display_unit;

    // ── Sidebar: search, filters, list ──────────────────────────────────
    let vendor_options: Vec<VendorOption> =
        std::iter::once(VendorOption::All).chain(library.vendors().into_iter().map(VendorOption::Named)).collect();
    let vendor_selected = match &library.vendor_filter {
        Some(name) => VendorOption::Named(name.clone()),
        None => VendorOption::All,
    };
    let type_options: Vec<TypeOption> =
        std::iter::once(TypeOption::All).chain(library.tool_types_present().into_iter().map(TypeOption::Specific)).collect();
    let type_selected = match library.type_filter {
        Some(t) => TypeOption::Specific(t),
        None => TypeOption::All,
    };

    let filters = column![
        iced::widget::pick_list(Some(vendor_selected), vendor_options, |choice: &VendorOption| choice.to_string())
            .on_select(|choice| Message::CamPanel(CamPanelMsg::VendorFilterSelect(choice)))
            .text_size(12)
            .width(Fill),
        iced::widget::pick_list(Some(type_selected), type_options, |choice: &TypeOption| choice.to_string())
            .on_select(|choice| Message::CamPanel(CamPanelMsg::TypeFilterSelect(choice)))
            .text_size(12)
            .width(Fill),
        row![
            text("Units").size(12).width(Fill),
            button(text("mm").size(11))
                .padding([3, 10])
                .style(if unit == DisplayUnit::Mm { button::primary } else { button::secondary })
                .on_press(Message::CamPanel(CamPanelMsg::UnitToggle(DisplayUnit::Mm))),
            button(text("inch").size(11))
                .padding([3, 10])
                .style(if unit == DisplayUnit::Inch { button::primary } else { button::secondary })
                .on_press(Message::CamPanel(CamPanelMsg::UnitToggle(DisplayUnit::Inch))),
        ]
        .spacing(4)
        .align_y(iced::Center),
        row![
            iced::widget::checkbox(library.incomplete_only)
                .size(14)
                .on_toggle(|value| Message::CamPanel(CamPanelMsg::IncompleteOnlyToggle(value))),
            text("Incomplete only").size(12),
        ]
        .spacing(6)
        .align_y(iced::Center),
    ]
    .spacing(8);

    let list: Element<'a, Message> = if editor.tool_trash {
        trash
            .iter()
            .enumerate()
            .fold(column![].spacing(5), |column, (index, tool)| {
                column.push(
                    row![
                        text(format!(
                            "{}   Ø{:.3} {}   {}",
                            tool.name,
                            tool.diameter_mm.map(|d| unit.from_mm(d)).unwrap_or(0.0),
                            unit.label(),
                            tool.tool_type.display_name(),
                        ))
                        .size(12)
                        .width(Fill),
                        button(text("Restore").size(11)).on_press(Message::CamPanel(CamPanelMsg::ToolRestore(index))),
                        button(text("Delete").size(11)).on_press(Message::CamPanel(CamPanelMsg::ToolPurge(index))),
                    ]
                    .spacing(6),
                )
            })
            .into()
    } else {
        library
            .filtered_tools(&query)
            .into_iter()
            .map(|tool| (tools.iter().position(|t| t.id == tool.id).unwrap_or(0), tool))
            .fold(column![].spacing(2), |column, (index, tool)| {
                let subtitle = format!(
                    "{} · Ø{:.3} {} · {}F{}",
                    tool.tool_type.display_name(),
                    tool.diameter_mm.map(|d| unit.from_mm(d)).unwrap_or(0.0),
                    unit.label(),
                    tool.flute_count.unwrap_or(0),
                    if tool.is_machinable() { "" } else { " · incomplete" },
                );
                let mut rows = column![
                    text(tool.name.clone()).size(13),
                    text(subtitle).size(11),
                ]
                .spacing(1);
                if tool.vendor_name.is_some() || tool.product_id.is_some() {
                    rows = rows.push(
                        text(format!(
                            "{} · {}",
                            tool.vendor_name.clone().unwrap_or_default(),
                            tool.product_id.clone().unwrap_or_default()
                        ))
                        .size(11),
                    );
                }
                column.push(
                    button(rows)
                        .width(Fill)
                        .padding(6)
                        .style(if selected == Some(index) { button::primary } else { button::secondary })
                        .on_press(Message::CamPanel(CamPanelMsg::ToolSelect(index))),
                )
            })
            .into()
    };

    let sidebar = column![
        text_input("Search name, vendor, or part number", &editor.tool_search)
            .size(12)
            .on_input(|value| Message::CamPanel(CamPanelMsg::ToolSearch(value))),
        filters,
        scrollable(list).height(Fill),
    ]
    .spacing(10)
    .width(Length::Fixed(240.0));

    // ── Detail panel ─────────────────────────────────────────────────────
    let selected_tool = selected.and_then(|i| tools.get(i));

    let toolbar = row![
        text("Tool Database").size(20).width(Fill),
        icon_button(include_bytes!("../../../assets/icons/tool_add.svg"), "New tool", Message::CamPanel(CamPanelMsg::ToolCreate)),
        icon_button(include_bytes!("../../../assets/icons/copy.svg"), "Duplicate", Message::CamPanel(CamPanelMsg::ToolDuplicate)),
        icon_button(include_bytes!("../../../assets/icons/ui/trash.svg"), "Move to Trash", Message::CamPanel(CamPanelMsg::ToolDelete)),
        icon_button(include_bytes!("../../../assets/icons/cui_import.svg"), "Import CSV", Message::CamPanel(CamPanelMsg::ToolImportCsv)),
        icon_button(include_bytes!("../../../assets/icons/cui_export.svg"), "Export CSV", Message::CamPanel(CamPanelMsg::ToolExportCsv)),
        button(text(if editor.tool_trash { "Back to Tools" } else { "Trash" }).size(12))
            .on_press(Message::CamPanel(CamPanelMsg::ToolToggleTrash)),
    ]
    .spacing(6)
    .align_y(iced::Center);

    let Some(tool) = selected_tool else {
        let body = column![toolbar, text("Select a tool from the catalog, or create a new one.").size(13)]
            .spacing(16)
            .padding(16);
        return row![sidebar, container(body).width(Fill).height(Fill)].spacing(16).padding(12).into();
    };

    let type_options: Vec<ToolType> = ToolType::all().to_vec();
    let identity = column![
        section_title("Identity"),
        library_text("Name", &editor.tool_name, |v| Message::CamPanel(CamPanelMsg::ToolName(v))),
        row![
            text("Type").width(Length::Fixed(110.0)).size(12),
            iced::widget::pick_list(Some(editor.tool_type), type_options, |choice: &ToolType| choice.display_name().to_string())
                .on_select(|choice| Message::CamPanel(CamPanelMsg::ToolTypeSelect(choice)))
                .text_size(12)
                .width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Center),
        library_text("Vendor", &editor.tool_vendor, |v| Message::CamPanel(CamPanelMsg::ToolVendor(v))),
        library_text("Part number", &editor.tool_part_number, |v| Message::CamPanel(CamPanelMsg::ToolPartNumber(v))),
        library_text("Series", &editor.tool_series, |v| Message::CamPanel(CamPanelMsg::ToolSeries(v))),
    ]
    .spacing(6);

    let mut geometry = column![
        section_title(format!("Geometry ({})", unit.label())),
        row![
            library_number("Diameter", LibraryToolField::Diameter, tool.diameter_mm.map(|d| unit.from_mm(d)).unwrap_or(6.0), editor),
            library_number("Flutes", LibraryToolField::FluteCount, tool.flute_count.unwrap_or(2) as f64, editor),
        ].spacing(8),
        row![
            library_number("Cutting length", LibraryToolField::FluteLength, tool.flute_length_mm.map(|d| unit.from_mm(d)).unwrap_or(20.0), editor),
            library_number("Shank diameter", LibraryToolField::ShankDia, tool.shank_dia_mm.map(|d| unit.from_mm(d)).unwrap_or(6.0), editor),
        ].spacing(8),
        row![
            library_number("Overall length", LibraryToolField::OverallLength, tool.overall_length_mm.map(|d| unit.from_mm(d)).unwrap_or(50.0), editor),
            library_number("Corner radius", LibraryToolField::CornerRadius, tool.corner_radius_mm.map(|d| unit.from_mm(d)).unwrap_or(0.0), editor),
        ].spacing(8),
    ]
    .spacing(6);
    if tool.tool_type.requires_included_angle() {
        geometry = geometry.push(library_number(
            "Included angle °",
            LibraryToolField::IncludedAngle,
            tool.included_angle_deg.unwrap_or(90.0),
            editor,
        ));
    }
    if matches!(tool.tool_type, ToolType::Engraver | ToolType::Chamfer | ToolType::TaperedBall) {
        geometry = geometry.push(library_number(
            "Tip diameter",
            LibraryToolField::TipDiameter,
            tool.tip_dia_mm.map(|d| unit.from_mm(d)).unwrap_or(0.0),
            editor,
        ));
    }

    let material_finish = column![
        section_title("Material and finish"),
        row![
            text("Substrate").width(Length::Fixed(110.0)).size(12),
            iced::widget::pick_list(Some(editor.tool_substrate), SubstrateOption::ALL.to_vec(), |choice: &SubstrateOption| choice.to_string())
                .on_select(|choice| Message::CamPanel(CamPanelMsg::ToolSubstrateSelect(choice)))
                .text_size(12)
                .width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Center),
        library_text("Coating", &editor.tool_coating, |v| Message::CamPanel(CamPanelMsg::ToolCoating(v))),
        row![
            text("Chip direction").width(Length::Fixed(110.0)).size(12),
            iced::widget::pick_list(Some(editor.tool_chip_direction), ChipDirectionOption::ALL.to_vec(), |choice: &ChipDirectionOption| choice.to_string())
                .on_select(|choice| Message::CamPanel(CamPanelMsg::ToolChipDirectionSelect(choice)))
                .text_size(12)
                .width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Center),
        library_text("Notes", &editor.tool_notes, |v| Message::CamPanel(CamPanelMsg::ToolNotes(v))),
    ]
    .spacing(6);

    let missing = {
        let m = tool.missing_fields();
        if m.is_empty() { String::new() } else { format!("Missing before this tool can cut: {}", m.join(", ")) }
    };

    let material_options: Vec<String> = library.material_classes.iter().map(|c| c.name.clone()).collect();
    let material_selected = library.material_classes.iter().find(|c| c.id == library.material_class_id).map(|c| c.name.clone());
    let doc_mm = library.depth_of_cut_mm.or_else(|| tool.recommended_doc_mm()).unwrap_or(6.0);

    let cutting_data: Element<'a, Message> = if let Some(data) = resolved {
        let source = match &data.source {
            crate::tool_library::resolver::Source::Preset { name, .. } => format!("Preset: {name}"),
            crate::tool_library::resolver::Source::FeedCurve { vendor } => {
                format!("Vendor feed curve{}", vendor.as_ref().map(|v| format!(" ({v})")).unwrap_or_default())
            }
            crate::tool_library::resolver::Source::Rule { vendor } => match vendor {
                Some(v) => format!("Chipload band ({v})"),
                None => "Generic chipload estimate".to_string(),
            },
        };
        let mut clamped_notes = column![].spacing(2);
        for note in &data.clamped {
            clamped_notes = clamped_notes.push(text(format!("↳ {note}")).size(11));
        }
        column![
            data_row("Spindle", format!("{} rpm", data.spindle_rpm)),
            data_row("Feed", format!("{:.0} mm/min", data.feed_xy_mm_min)),
            data_row("Plunge", format!("{:.0} mm/min", data.feed_z_mm_min)),
            data_row("Chipload", format!("{:.3} mm/tooth", data.chipload_mm)),
            data_row("Stepdown", format!("{:.2} mm", data.stepdown_mm)),
            text(format!(
                "{source}{}",
                if data.is_estimate() { " — estimate, verify before cutting" } else { " — trusted data" }
            ))
            .size(11),
            clamped_notes,
            row![
                button(text("Apply to operation").size(12)).on_press(Message::CamPanel(CamPanelMsg::ToolApplyResolved)),
                button(text("Save as my preset for this material").size(12)).on_press(Message::CamPanel(CamPanelMsg::ToolSavePreset)),
            ]
            .spacing(8),
        ]
        .spacing(4)
        .into()
    } else {
        text("No cutting data yet — pick a material, or add flute count/diameter to this tool.").size(12).into()
    };

    let cutting_section = column![
        section_title("Cutting Data"),
        row![
            text("Material").width(Length::Fixed(90.0)).size(12),
            iced::widget::pick_list(material_selected, material_options, |name: &String| name.clone())
                .on_select(|name| Message::CamPanel(CamPanelMsg::MaterialClassSelect(name)))
                .text_size(12)
                .width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Center),
        row![
            text("Depth of cut").width(Length::Fixed(90.0)).size(12),
            iced::widget::slider(0.5..=tool.diameter_mm.unwrap_or(20.0).max(1.0) * 3.0, doc_mm, |v| {
                Message::CamPanel(CamPanelMsg::DepthOfCutChanged(v))
            })
            .step(0.1),
            text(format!("{doc_mm:.1} mm")).size(12),
        ]
        .spacing(8)
        .align_y(iced::Center),
        cutting_data,
    ]
    .spacing(8);

    let import_review: Element<'a, Message> = if let Some(plan) = &library.pending_import {
        column![
            text(format!("Import review — {}", plan.source_file_name)).size(14),
            text(format!("{} new · {} changed · {} rejected", plan.new_count(), plan.changed_count(), plan.rejected_count())).size(12),
            row![
                button(text(format!("Import {} tools", plan.new_count() + plan.changed_count())).size(12))
                    .on_press(Message::CamPanel(CamPanelMsg::ToolImportCommit)),
                button(text("Cancel").size(12)).on_press(Message::CamPanel(CamPanelMsg::ToolImportCancel)),
            ]
            .spacing(8),
        ]
        .spacing(5)
        .into()
    } else {
        text("").into()
    };

    let detail = scrollable(
        column![toolbar, identity, geometry, material_finish, text(missing).size(11), cutting_section, import_review]
            .spacing(18)
            .padding(16),
    );

    row![sidebar, container(detail).width(Fill).height(Fill)].spacing(16).padding(12).into()
}

fn library_text<'a>(label: &'a str, value: &'a str, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    row![
        text(label).width(Length::Fixed(110.0)).size(12),
        text_input("", value).size(12).on_input(on_input).width(Fill),
    ]
    .spacing(8)
    .align_y(iced::Center)
    .into()
}

fn library_number<'a>(
    label: &'static str,
    field: LibraryToolField,
    value: f64,
    editor: &'a CamEditorState,
) -> Element<'a, Message> {
    row![
        text(label).width(Length::Fixed(110.0)),
        text_input("", &editor.value(NumericField::LibraryTool(field), value))
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
