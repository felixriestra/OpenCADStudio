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
                    title: "Setup",
                    tools: vec![RibbonItem::LargeTool(tool("CAMINFO", "CAM Setup", "⚙"))],
                },
                RibbonGroup {
                    title: "2D Toolpaths",
                    tools: vec![
                        RibbonItem::LargeTool(tool("CAMPROFILE", "Outside Profile", "◎")),
                        RibbonItem::LargeTool(tool("CAMINSIDE", "Inside Profile", "◉")),
                        RibbonItem::LargeTool(tool("CAMPOCKET", "Pocket", "▣")),
                        RibbonItem::LargeTool(tool("CAMFACE", "Face", "▤")),
                        RibbonItem::LargeTool(tool("CAMBORE", "Bore", "◌")),
                        RibbonItem::LargeTool(tool("CAMSLOT", "Slot", "▭")),
                        RibbonItem::LargeTool(tool("CAMENGRAVE", "Engrave", "⌁")),
                        RibbonItem::LargeTool(tool("CAMDRILL", "Drill", "⊙")),
                    ],
                },
                RibbonGroup {
                    title: "Output",
                    tools: vec![RibbonItem::LargeTool(tool(
                        "CAMEXPORT",
                        "Export G-code",
                        "⇧",
                    ))],
                },
            ]
        })
    }
}
