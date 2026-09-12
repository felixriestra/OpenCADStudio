//! Integrated 2.5D CAM ribbon.

use crate::modules::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};

pub struct CamModule;

fn tool(id: &'static str, label: &'static str, glyph: &'static str) -> ToolDef {
    ToolDef {
        id,
        label,
        icon: IconKind::Glyph(glyph),
        event: ModuleEvent::Command(id.to_string()),
    }
}

fn svg_tool(id: &'static str, label: &'static str, bytes: &'static [u8]) -> ToolDef {
    ToolDef {
        id,
        label,
        icon: IconKind::Svg(bytes),
        event: ModuleEvent::Command(id.to_string()),
    }
}

impl CadModule for CamModule {
    fn id(&self) -> &'static str {
        "cam"
    }

    fn title(&self) -> &'static str {
        "CAM"
    }

    fn ribbon_groups(&self) -> &[RibbonGroup] {
        static GROUPS: std::sync::OnceLock<Vec<RibbonGroup>> = std::sync::OnceLock::new();
        GROUPS.get_or_init(|| {
            vec![
                RibbonGroup {
                    title: "Tools",
                    tools: vec![RibbonItem::LargeTool(svg_tool(
                        "CAMTOOLS",
                        "Tool Library",
                        include_bytes!("../../../assets/icons/cam_tools.svg"),
                    ))],
                },
                RibbonGroup {
                    title: "Setup",
                    tools: vec![
                        RibbonItem::LargeTool(tool("CAMSETUP", "CAM Setup", "⚙")),
                        RibbonItem::LargeTool(tool("CAMLIST", "Operations", "☷")),
                        RibbonItem::LargeTool(tool("CAMCLEAR", "Clear Job", "⌫")),
                    ],
                },
                RibbonGroup {
                    title: "Import",
                    tools: vec![
                        RibbonItem::LargeTool(tool("IMPORTSVG", "SVG to CAD", "◇")),
                        RibbonItem::LargeTool(tool("IMAGEATTACH", "Attach Bitmap", "▧")),
                    ],
                },
                RibbonGroup {
                    title: "2D Toolpaths",
                    tools: vec![
                        RibbonItem::Tool(svg_tool(
                            "CAMPROFILE",
                            "Outside Profile",
                            include_bytes!("../../../assets/icons/cam_profile.svg"),
                        )),
                        RibbonItem::Tool(svg_tool(
                            "CAMINSIDE",
                            "Inside Profile",
                            include_bytes!("../../../assets/icons/cam_inside.svg"),
                        )),
                        RibbonItem::Tool(svg_tool(
                            "CAMPOCKET",
                            "Pocket",
                            include_bytes!("../../../assets/icons/cam_pocket.svg"),
                        )),
                        RibbonItem::Tool(svg_tool(
                            "CAMFACE",
                            "Face",
                            include_bytes!("../../../assets/icons/cam_face.svg"),
                        )),
                        RibbonItem::Tool(svg_tool(
                            "CAMBORE",
                            "Bore",
                            include_bytes!("../../../assets/icons/cam_bore.svg"),
                        )),
                        RibbonItem::Tool(svg_tool(
                            "CAMSLOT",
                            "Slot",
                            include_bytes!("../../../assets/icons/cam_slot.svg"),
                        )),
                        RibbonItem::Tool(svg_tool(
                            "CAMENGRAVE",
                            "Engrave",
                            include_bytes!("../../../assets/icons/cam_engrave.svg"),
                        )),
                        RibbonItem::Tool(svg_tool(
                            "CAMDRILL",
                            "Drill",
                            include_bytes!("../../../assets/icons/cam_drill.svg"),
                        )),
                    ],
                },
                RibbonGroup {
                    title: "Output",
                    tools: vec![
                        RibbonItem::LargeTool(tool("CAMPREVIEW", "Preview", "▶")),
                        RibbonItem::LargeTool(tool("CAMEXPORT", "Export G-code", "⇧")),
                    ],
                },
            ]
        })
    }
}
