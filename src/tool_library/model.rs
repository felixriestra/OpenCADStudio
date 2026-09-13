//! The tool database's own data types — distinct from `ocs_cam_core::ToolDefinition`,
//! which is a *project's* embedded copy of a tool baked into a `CamOperation`.
//! `LibraryTool` is the reusable catalog row; nothing here ever gets serialized
//! directly into a `.mac2cam` project file.
//!
//! Ported from 2DCam's `Sources/TwoDCamCore/ToolLibrary/LibraryTool.swift` and
//! `PresetResolver.swift`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolType {
    EndMill,
    BallNose,
    BullNose,
    VBit,
    Engraver,
    TaperedBall,
    Drill,
    Chamfer,
    Surfacing,
    Form,
    DragKnife,
}

impl ToolType {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::EndMill => "end_mill",
            Self::BallNose => "ball_nose",
            Self::BullNose => "bull_nose",
            Self::VBit => "v_bit",
            Self::Engraver => "engraver",
            Self::TaperedBall => "tapered_ball",
            Self::Drill => "drill",
            Self::Chamfer => "chamfer",
            Self::Surfacing => "surfacing",
            Self::Form => "form",
            Self::DragKnife => "drag_knife",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        Some(match value {
            "end_mill" => Self::EndMill,
            "ball_nose" => Self::BallNose,
            "bull_nose" => Self::BullNose,
            "v_bit" => Self::VBit,
            "engraver" => Self::Engraver,
            "tapered_ball" => Self::TaperedBall,
            "drill" => Self::Drill,
            "chamfer" => Self::Chamfer,
            "surfacing" => Self::Surfacing,
            "form" => Self::Form,
            "drag_knife" => Self::DragKnife,
            _ => return None,
        })
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::EndMill => "End Mill",
            Self::BallNose => "Ball Nose",
            Self::BullNose => "Bull Nose",
            Self::VBit => "V-Bit",
            Self::Engraver => "Engraving Cutter",
            Self::TaperedBall => "Tapered Ball Nose",
            Self::Drill => "Drill",
            Self::Chamfer => "Chamfer Mill",
            Self::Surfacing => "Surfacing Cutter",
            Self::Form => "Form Tool",
            Self::DragKnife => "Drag Knife",
        }
    }

    pub fn all() -> &'static [ToolType] {
        &[
            Self::EndMill,
            Self::BallNose,
            Self::BullNose,
            Self::VBit,
            Self::Engraver,
            Self::TaperedBall,
            Self::Drill,
            Self::Chamfer,
            Self::Surfacing,
            Self::Form,
            Self::DragKnife,
        ]
    }

    /// Types whose geometry is meaningless without an included angle.
    pub fn requires_included_angle(self) -> bool {
        matches!(self, Self::VBit | Self::Engraver | Self::TaperedBall)
    }
}

impl Default for ToolType {
    fn default() -> Self {
        Self::EndMill
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayUnits {
    Mm,
    Inch,
    FracInch,
}

impl DisplayUnits {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Inch => "inch",
            Self::FracInch => "frac_inch",
        }
    }

    pub fn from_db_str(value: &str) -> Self {
        match value {
            "inch" => Self::Inch,
            "frac_inch" => Self::FracInch,
            _ => Self::Mm,
        }
    }
}

impl Default for DisplayUnits {
    fn default() -> Self {
        Self::Mm
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormScaleMode {
    KeepAngle,
    FitEnvelope,
}

impl FormScaleMode {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::KeepAngle => "keepAngle",
            Self::FitEnvelope => "fitEnvelope",
        }
    }

    pub fn from_db_str(value: &str) -> Self {
        match value {
            "fitEnvelope" => Self::FitEnvelope,
            _ => Self::KeepAngle,
        }
    }
}

impl Default for FormScaleMode {
    fn default() -> Self {
        Self::KeepAngle
    }
}

/// A drawn moulding-silhouette point for a form tool: radius from centre at a
/// given rise above the tip, before scaling to a specific tool's size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormProfilePoint {
    pub radius_mm: f64,
    pub rise_mm: f64,
}

/// A tool as the library knows it: identity, geometry, provenance. No cutting
/// data — that lives in `CuttingPreset`, keyed by material and machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryTool {
    pub id: String,
    pub vendor_id: Option<String>,
    pub vendor_name: Option<String>,
    pub product_id: Option<String>,
    pub product_url: Option<String>,
    pub name: String,
    pub series: Option<String>,
    pub tool_type: ToolType,

    // Geometry. `None` means UNKNOWN — never substitute a zero.
    pub diameter_mm: Option<f64>,
    pub corner_radius_mm: Option<f64>,
    pub included_angle_deg: Option<f64>,
    pub tip_dia_mm: Option<f64>,
    pub flute_count: Option<i64>,
    pub flute_length_mm: Option<f64>,
    pub shank_dia_mm: Option<f64>,
    pub overall_length_mm: Option<f64>,
    pub neck_length_mm: Option<f64>,

    pub chip_direction: Option<String>,
    pub substrate: Option<String>,
    pub coating: Option<String>,
    /// The drawn moulding silhouette (right half: radius from centre, rise
    /// above tip) before scaling to this tool's diameter. `None` for
    /// non-form tools.
    pub form_profile_points: Option<Vec<FormProfilePoint>>,
    pub form_scale_mode: FormScaleMode,

    pub display_units: DisplayUnits,
    /// The vendor's own wording, e.g. `1/4"`.
    pub nominal_label: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl LibraryTool {
    pub fn new(name: impl Into<String>, tool_type: ToolType) -> Self {
        Self {
            id: crate::tool_library::new_id(),
            vendor_id: None,
            vendor_name: None,
            product_id: None,
            product_url: None,
            name: name.into(),
            series: None,
            tool_type,
            diameter_mm: None,
            corner_radius_mm: None,
            included_angle_deg: None,
            tip_dia_mm: None,
            flute_count: None,
            flute_length_mm: None,
            shank_dia_mm: None,
            overall_length_mm: None,
            neck_length_mm: None,
            chip_direction: None,
            substrate: None,
            coating: None,
            form_profile_points: None,
            form_scale_mode: FormScaleMode::default(),
            display_units: DisplayUnits::default(),
            nominal_label: None,
            notes: None,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    /// Geometry-derived recommended depth of cut per pass, in mm: the lesser
    /// of 60% of the cutting diameter and 50% of the cutting length. Advisory
    /// only — the operation picks the final DOC.
    pub fn recommended_doc_mm(&self) -> Option<f64> {
        let dia = self.diameter_mm.filter(|d| *d > 0.0)?;
        let by_diameter = 0.60 * dia;
        match self.flute_length_mm.filter(|f| *f > 0.0) {
            Some(flute) => Some(by_diameter.min(0.50 * flute)),
            None => Some(by_diameter),
        }
    }

    /// The minimum needed to compute a toolpath at all.
    pub fn is_machinable(&self) -> bool {
        let Some(d) = self.diameter_mm.filter(|d| *d > 0.0) else {
            return false;
        };
        let Some(z) = self.flute_count.filter(|z| *z > 0) else {
            return false;
        };
        let _ = (d, z);
        if self.tool_type.requires_included_angle() && self.included_angle_deg.is_none() {
            return false;
        }
        if self.tool_type == ToolType::Form
            && self.form_profile_points.as_ref().map(|p| p.len()).unwrap_or(0) < 2
        {
            return false;
        }
        true
    }

    /// Fields a user must fill before this tool can be used. Drives the
    /// "incomplete" badge in the library and the disabled state in CAM.
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.diameter_mm.is_none() {
            missing.push("diameter");
        }
        if self.flute_count.is_none() {
            missing.push("flute count");
        }
        if self.flute_length_mm.is_none() {
            missing.push("cutting length");
        }
        if self.overall_length_mm.is_none() {
            missing.push("overall length");
        }
        if self.tool_type.requires_included_angle() && self.included_angle_deg.is_none() {
            missing.push("included angle");
        }
        if self.tool_type == ToolType::BullNose && self.corner_radius_mm.is_none() {
            missing.push("corner radius");
        }
        if self.tool_type == ToolType::Form
            && self.form_profile_points.as_ref().map(|p| p.len()).unwrap_or(0) < 2
        {
            missing.push("profile shape");
        }
        missing
    }

    /// Build the CAM-side `ToolDefinition`. Mac2CAM's CAM pipeline only
    /// understands diameter/feed/plunge/RPM today (no cutter-kind-aware
    /// toolpaths yet), so richer geometry (corner radius, included angle,
    /// form profile) stays in the library rather than being invented here.
    /// Returns `None` rather than guessing when the geometry is incomplete.
    pub fn to_tool_definition(&self, resolved: &super::resolver::ResolvedCuttingData) -> Option<ocs_cam_core::ToolDefinition> {
        let diameter = self.diameter_mm?;
        Some(ocs_cam_core::ToolDefinition {
            id: self.id.clone(),
            name: self.name.clone(),
            diameter,
            feed: resolved.feed_xy_mm_min,
            plunge_feed: resolved.feed_z_mm_min,
            spindle_rpm: resolved.spindle_rpm.clamp(0, u32::MAX as i64) as u32,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialClassInfo {
    pub id: String,
    pub name: String,
    pub melts: bool,
    pub prefers_downcut: bool,
    pub max_rpm_hint: Option<i64>,
    /// Feed multiplier relative to the base (softwood) feed curve.
    pub feed_factor: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetOrigin {
    User,
    Vendor,
    Derived,
    Generic,
}

impl PresetOrigin {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Vendor => "vendor",
            Self::Derived => "derived",
            Self::Generic => "generic",
        }
    }

    pub fn from_db_str(value: &str) -> Self {
        match value {
            "vendor" => Self::Vendor,
            "derived" => Self::Derived,
            "generic" => Self::Generic,
            _ => Self::User,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetDerivation {
    Explicit,
    FromChipload,
}

impl PresetDerivation {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::FromChipload => "from_chipload",
        }
    }

    pub fn from_db_str(value: &str) -> Self {
        match value {
            "from_chipload" => Self::FromChipload,
            _ => Self::Explicit,
        }
    }
}

/// A user- or vendor-supplied cutting recommendation for one tool, optionally
/// scoped to a material and/or machine. Distinct from `ocs_cam_core`'s
/// `CuttingPreset`-shaped operation defaults — this is catalog data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CuttingPreset {
    pub id: String,
    pub tool_id: String,
    pub material_id: Option<String>,
    pub material_class_id: Option<String>,
    pub machine_id: Option<String>,
    pub name: String,
    pub origin: PresetOrigin,
    pub derivation: PresetDerivation,
    pub confidence: i64,

    pub chipload_mm: Option<f64>,
    pub vc_m_per_min: Option<f64>,
    pub spindle_rpm: Option<i64>,
    pub feed_xy_mm_min: Option<f64>,
    pub feed_z_mm_min: Option<f64>,
    pub ramp_feed_mm_min: Option<f64>,
    pub stepdown_mm: Option<f64>,
    pub stepover_mm: Option<f64>,
    pub clearance_stepover_mm: Option<f64>,
    pub cut_direction: Option<String>,
    pub air_blast: bool,
    pub notes: Option<String>,
}

impl CuttingPreset {
    /// Mirrors `v_preset_ranked`. Higher wins.
    pub fn score(&self, machine_id: Option<&str>) -> i64 {
        let origin_score = match self.origin {
            PresetOrigin::User => 400,
            PresetOrigin::Vendor => 300,
            PresetOrigin::Derived => 200,
            PresetOrigin::Generic => 100,
        };
        let scope_score = if self.material_id.is_some() {
            40
        } else if self.material_class_id.is_some() {
            20
        } else {
            0
        };
        let machine_score = if self.machine_id.as_deref() == machine_id && machine_id.is_some() {
            5
        } else {
            0
        };
        origin_score + scope_score + machine_score + self.confidence
    }
}

/// The shape in which Sorotec, CMT and Leuco actually publish cutting data: a
/// chipload band per material class and diameter range.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChiploadRule {
    pub id: String,
    pub material_class_id: String,
    pub tool_type: Option<ToolType>,
    /// `None` = generic (applies to every vendor).
    pub vendor_id: Option<String>,
    pub dia_min_mm: f64,
    pub dia_max_mm: f64,
    pub fz_min_mm: f64,
    pub fz_typ_mm: f64,
    pub fz_max_mm: f64,
    pub rpm_min: Option<i64>,
    pub rpm_max: Option<i64>,
    pub max_stepdown_x_dia: Option<f64>,
}

/// A vendor feed-vs-depth curve (e.g. CMT's Vf/H chart): at a fixed RPM the
/// safe feed falls as depth of cut rises. Stored as a line between two
/// published points, the BASE (softwood, along-grain) curve; material
/// feed-factor and cross-grain factor scale it at resolve time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedCurve {
    pub id: String,
    pub vendor_id: Option<String>,
    pub series: Option<String>,
    pub tool_type: Option<ToolType>,
    pub dia_min_mm: f64,
    pub dia_max_mm: f64,
    pub rpm: i64,
    pub doc_lo_mm: f64,
    pub feed_lo_mm_min: f64,
    pub doc_hi_mm: f64,
    pub feed_hi_mm_min: f64,
    pub cross_grain_factor: f64,
    pub chip_min_mm: Option<f64>,
    pub chip_max_mm: Option<f64>,
}

impl FeedCurve {
    /// Base feed at a depth of cut, clamped to the curve's measured range so
    /// we never extrapolate past what the vendor actually published.
    pub fn base_feed(&self, doc: f64) -> f64 {
        let lo = self.doc_lo_mm.min(self.doc_hi_mm);
        let hi = self.doc_lo_mm.max(self.doc_hi_mm);
        let d = doc.max(lo).min(hi);
        if (self.doc_hi_mm - self.doc_lo_mm).abs() < f64::EPSILON {
            return self.feed_lo_mm_min;
        }
        let t = (d - self.doc_lo_mm) / (self.doc_hi_mm - self.doc_lo_mm);
        self.feed_lo_mm_min + (self.feed_hi_mm_min - self.feed_lo_mm_min) * t
    }

    pub fn matches(&self, tool: &LibraryTool) -> bool {
        let Some(dia) = tool.diameter_mm else {
            return false;
        };
        if dia < self.dia_min_mm || dia > self.dia_max_mm {
            return false;
        }
        if let Some(vendor_id) = &self.vendor_id {
            if Some(vendor_id.as_str()) != tool.vendor_id.as_deref() {
                return false;
            }
        }
        if let Some(tool_type) = self.tool_type {
            if tool_type != tool.tool_type {
                return false;
            }
        }
        if let Some(series) = &self.series {
            if Some(series.as_str()) != tool.series.as_deref() {
                return false;
            }
        }
        true
    }
}

/// A machine row in the tool database — kept separate from
/// `ocs_cam_core::MachineEnvelope` (the project's own machine setup) so
/// presets can be scoped to a named machine profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineRow {
    pub id: String,
    pub name: String,
    pub minimum_spindle_rpm: i64,
    pub maximum_spindle_rpm: i64,
    pub maximum_feed: f64,
    pub maximum_plunge_feed: Option<f64>,
    pub rigidity_factor: f64,
}

impl Default for MachineRow {
    fn default() -> Self {
        Self {
            id: "m-default".into(),
            name: "Default Machine".into(),
            minimum_spindle_rpm: 6_000,
            maximum_spindle_rpm: 24_000,
            maximum_feed: 5_000.0,
            maximum_plunge_feed: None,
            rigidity_factor: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolGroup {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub sort_order: i64,
}
