//! Ribbon button definitions for the Constraints group.

use crate::modules::{IconKind, ModuleEvent, ToolDef};

pub mod horizontal {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCHORIZONTAL",
            label: "Horizontal",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/horizontal.svg")),
            event: ModuleEvent::Command("GCHORIZONTAL".to_string()),
        }
    }
}

pub mod vertical {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCVERTICAL",
            label: "Vertical",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/vertical.svg")),
            event: ModuleEvent::Command("GCVERTICAL".to_string()),
        }
    }
}

pub mod parallel {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCPARALLEL",
            label: "Parallel",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/parallel.svg")),
            event: ModuleEvent::Command("GCPARALLEL".to_string()),
        }
    }
}

pub mod perpendicular {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCPERPENDICULAR",
            label: "Perpendicular",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/perpendicular.svg")),
            event: ModuleEvent::Command("GCPERPENDICULAR".to_string()),
        }
    }
}

pub mod equal {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCEQUAL",
            label: "Equal",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/equal.svg")),
            event: ModuleEvent::Command("GCEQUAL".to_string()),
        }
    }
}

pub mod tangent {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCTANGENT",
            label: "Tangent",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/tangent.svg")),
            event: ModuleEvent::Command("GCTANGENT".to_string()),
        }
    }
}

pub mod concentric {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCCONCENTRIC",
            label: "Concentric",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/concentric.svg")),
            event: ModuleEvent::Command("GCCONCENTRIC".to_string()),
        }
    }
}

/// A line perpendicular to a circle/arc's tangent at their point of
/// contact — for the Line/Circle-only entity model this is equivalent to
/// "the line passes through the circle's center" (a circle's radius is
/// always normal to its own tangent), so it's built from the same
/// `PointOnLine` primitive `PointOnCurve` already uses for a point-on-line
/// case (`sketch_solve.rs`). Distinct from `Perpendicular`, which only
/// covers line-to-line — AutoCAD's own `GeomConstraintType` enum lists
/// `kNormal` and `kPerpendicular` separately for exactly this reason.
pub mod normal {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "NRCONSTRAINT",
            label: "Normal",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/normal.svg")),
            event: ModuleEvent::Command("NRCONSTRAINT".to_string()),
        }
    }
}

pub mod colinear {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCCOLLINEAR",
            label: "Colinear",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/colinear.svg")),
            event: ModuleEvent::Command("GCCOLLINEAR".to_string()),
        }
    }
}

pub mod fixed {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCFIX",
            label: "Fixed",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/fixed.svg")),
            event: ModuleEvent::Command("GCFIX".to_string()),
        }
    }
}

pub mod symmetric {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "GCSYMMETRIC",
            label: "Symmetric",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/symmetric.svg")),
            event: ModuleEvent::Command("GCSYMMETRIC".to_string()),
        }
    }
}

/// Global constraint-glyph visibility toggle (Constraints ribbon group).
/// id "SHOWCONSTRAINTS" is special-cased in `ui/ribbon/widgets.rs`'s
/// `is_active_tool` for toggle-state highlighting off `App::show_constraints`.
pub mod show_constraints {
    use super::*;
    pub fn tool() -> ToolDef {
        ToolDef {
            id: "SHOWCONSTRAINTS",
            label: "Show Constraints",
            icon: IconKind::Svg(include_bytes!("../../../../assets/icons/constrain/show_constraints.svg")),
            event: ModuleEvent::Command("SHOWCONSTRAINTS".to_string()),
        }
    }
}

// ── Autocomplete registry ─────────────────────────────────
inventory::submit!(crate::command::CommandRegistration {
    names: &[
        "SHOWCONSTRAINTS",
        "GCHORIZONTAL",
        "GCVERTICAL",
        "GCPARALLEL",
        "GCPERPENDICULAR",
        "GCEQUAL",
        "GCTANGENT",
        "GCCONCENTRIC",
        "NRCONSTRAINT",
        "GCCOLLINEAR",
        "GCFIX",
        "GCSYMMETRIC",
    ]
});
