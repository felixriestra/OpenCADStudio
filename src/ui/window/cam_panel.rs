use crate::app::Message;
use crate::ui::dock::{DockMsg, PanelId};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, Length};

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub enum ToolField {
    Diameter,
    Feed,
    Plunge,
    Rpm,
}

#[derive(Debug, Clone, Copy)]
pub enum AdvancedField {
    Tabs,
    TabHeight,
    LeadIn,
    LeadOut,
    RampLength,
    FinishAllowance,
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
    ClearPreview,
    PreviewFirst,
    PreviewPrevious,
    PreviewNext,
    PreviewLast,
    AdjustSetup(SetupField, f64),
    AdjustTool(ToolField, f64),
    ApplyLibraryTool(usize),
    AdjustAdvanced(AdvancedField, f64),
    ToggleFinishPass,
    TogglePocketIslands,
    ToggleDrillCycle,
    CycleMaterial,
}

pub fn view<'a>(
    job: &'a ocs_cam_core::CamJob,
    selected: Option<usize>,
    preview_len: usize,
    preview_step: Option<usize>,
    width: f32,
    auto_collapse: bool,
) -> Element<'a, Message> {
    let header = row![
        text("CAM Job").size(16).width(Fill),
        button(if auto_collapse { "Unpin" } else { "Pin" })
            .on_press(Message::Dock(DockMsg::AutoCollapseToggle(PanelId::Cam))),
        button("×").on_press(Message::Dock(DockMsg::Close(PanelId::Cam))),
    ]
    .spacing(6);

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
        let card = column![
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
        .spacing(3);
        operations = operations.push(container(card).padding(5).width(Fill));
    }

    let setup_section: Element<'_, Message> = if let Some(setup) = job.setups.first() {
        column![
            text("Setup & Stock").size(14),
            adjust_row("Width", setup.stock.width, SetupField::StockWidth, 5.0),
            adjust_row("Height", setup.stock.height, SetupField::StockHeight, 5.0),
            adjust_row(
                "Thickness",
                setup.stock.thickness,
                SetupField::StockThickness,
                1.0
            ),
            adjust_row("Clearance Z", setup.clearance_z, SetupField::Clearance, 1.0),
            adjust_row("Origin X", setup.work_origin.x, SetupField::OriginX, 1.0),
            adjust_row("Origin Y", setup.work_origin.y, SetupField::OriginY, 1.0),
            button(text(format!("Material: {}", setup.material.name)))
                .on_press(Message::CamPanel(CamPanelMsg::CycleMaterial)),
            text("Machine Limits").size(14),
            adjust_row(
                "Travel X",
                setup.machine.travel_x,
                SetupField::TravelX,
                10.0
            ),
            adjust_row(
                "Travel Y",
                setup.machine.travel_y,
                SetupField::TravelY,
                10.0
            ),
            adjust_row("Travel Z", setup.machine.travel_z, SetupField::TravelZ, 5.0),
            adjust_row(
                "Max feed",
                setup.machine.maximum_feed,
                SetupField::MaximumFeed,
                100.0
            ),
            adjust_row(
                "Max RPM",
                setup.machine.maximum_spindle_rpm as f64,
                SetupField::MaximumRpm,
                1000.0
            ),
        ]
        .spacing(4)
        .into()
    } else {
        text("No setup").into()
    };

    let tool_section: Element<'_, Message> = selected
        .and_then(|index| job.operations.get(index))
        .map(|operation| {
            column![
                text("Selected Tool").size(14),
                tool_row(
                    "Diameter",
                    operation.tool.diameter,
                    ToolField::Diameter,
                    0.5
                ),
                tool_row("Feed", operation.tool.feed, ToolField::Feed, 50.0),
                tool_row(
                    "Plunge",
                    operation.tool.plunge_feed,
                    ToolField::Plunge,
                    25.0
                ),
                tool_row(
                    "RPM",
                    operation.tool.spindle_rpm as f64,
                    ToolField::Rpm,
                    500.0
                ),
            ]
            .spacing(4)
            .into()
        })
        .unwrap_or_else(|| text("Select an operation to edit its tool.").into());

    let advanced_section: Element<'_, Message> = selected
        .and_then(|index| job.operations.get(index))
        .map(|operation| {
            let advanced = operation.advanced;
            column![
                text("Advanced Toolpath").size(14),
                advanced_row("Tabs", advanced.tab_count as f64, AdvancedField::Tabs, 1.0),
                advanced_row(
                    "Tab height",
                    advanced.tab_height,
                    AdvancedField::TabHeight,
                    0.5
                ),
                advanced_row("Lead in", advanced.lead_in, AdvancedField::LeadIn, 0.5),
                advanced_row("Lead out", advanced.lead_out, AdvancedField::LeadOut, 0.5),
                advanced_row(
                    "Ramp length",
                    advanced.ramp_length,
                    AdvancedField::RampLength,
                    1.0
                ),
                advanced_row(
                    "Finish allowance",
                    advanced.finish_allowance,
                    AdvancedField::FinishAllowance,
                    0.1,
                ),
                button(if advanced.finish_pass {
                    "Finish pass: On"
                } else {
                    "Finish pass: Off"
                })
                .on_press(Message::CamPanel(CamPanelMsg::ToggleFinishPass)),
                button(if advanced.preserve_pocket_islands {
                    "Pocket islands: Preserve"
                } else {
                    "Pocket islands: Ignore"
                })
                .on_press(Message::CamPanel(CamPanelMsg::TogglePocketIslands)),
                button(match advanced.drill_cycle {
                    ocs_cam_core::DrillCycle::Simple => "Drill cycle: Simple",
                    ocs_cam_core::DrillCycle::Peck => "Drill cycle: Peck",
                })
                .on_press(Message::CamPanel(CamPanelMsg::ToggleDrillCycle)),
            ]
            .spacing(4)
            .into()
        })
        .unwrap_or_else(|| text("Select an operation to edit its toolpath.").into());

    let mut library = column![text("Tool Library").size(14)].spacing(4);
    if job.tool_library.is_empty() {
        library = library.push(text("Tools used by operations will appear here."));
    }
    for (index, tool) in job.tool_library.iter().enumerate() {
        library = library.push(
            row![
                text(format!(
                    "{}  Ø{:.2}  {:.0} rpm",
                    tool.name, tool.diameter, tool.spindle_rpm
                ))
                .width(Fill),
                button("Use").on_press(Message::CamPanel(CamPanelMsg::ApplyLibraryTool(index))),
            ]
            .spacing(4),
        );
    }

    let body = column![
        header,
        row![
            button("Preview All").on_press(Message::CamPanel(CamPanelMsg::PreviewAll)),
            button("Selected").on_press(Message::CamPanel(CamPanelMsg::PreviewSelected)),
            button("Clear").on_press(Message::CamPanel(CamPanelMsg::ClearPreview)),
        ]
        .spacing(4),
        row![
            button("|◀").on_press(Message::CamPanel(CamPanelMsg::PreviewFirst)),
            button("◀").on_press(Message::CamPanel(CamPanelMsg::PreviewPrevious)),
            text(match preview_step {
                Some(step) => format!("{step}/{preview_len}"),
                None => format!("All {preview_len}"),
            })
            .width(Fill),
            button("▶").on_press(Message::CamPanel(CamPanelMsg::PreviewNext)),
            button("▶|").on_press(Message::CamPanel(CamPanelMsg::PreviewLast)),
        ]
        .spacing(4),
        operations,
        setup_section,
        tool_section,
        advanced_section,
        library,
    ]
    .spacing(10)
    .padding(8);
    container(scrollable(body).height(Fill))
        .width(Length::Fixed(width))
        .height(Fill)
        .into()
}

fn adjust_row(
    label: &'static str,
    value: f64,
    field: SetupField,
    step: f64,
) -> Element<'static, Message> {
    row![
        text(format!("{label}: {value:.2}")).width(Fill),
        button("−").on_press(Message::CamPanel(CamPanelMsg::AdjustSetup(field, -step))),
        button("+").on_press(Message::CamPanel(CamPanelMsg::AdjustSetup(field, step))),
    ]
    .spacing(3)
    .into()
}

fn tool_row(
    label: &'static str,
    value: f64,
    field: ToolField,
    step: f64,
) -> Element<'static, Message> {
    row![
        text(format!("{label}: {value:.2}")).width(Fill),
        button("−").on_press(Message::CamPanel(CamPanelMsg::AdjustTool(field, -step))),
        button("+").on_press(Message::CamPanel(CamPanelMsg::AdjustTool(field, step))),
    ]
    .spacing(3)
    .into()
}

fn advanced_row(
    label: &'static str,
    value: f64,
    field: AdvancedField,
    step: f64,
) -> Element<'static, Message> {
    row![
        text(format!("{label}: {value:.2}")).width(Fill),
        button("−").on_press(Message::CamPanel(CamPanelMsg::AdjustAdvanced(field, -step))),
        button("+").on_press(Message::CamPanel(CamPanelMsg::AdjustAdvanced(field, step))),
    ]
    .spacing(3)
    .into()
}
