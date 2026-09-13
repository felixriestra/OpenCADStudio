//! Ported from 2DCam's `Sources/TwoDCamCore/ToolLibrary/PresetResolver.swift`.
//! Resolution order, highest priority first:
//!   1. an explicit user/vendor preset
//!   2. a vendor feed-vs-depth curve, evaluated at the operation's DOC
//!   3. a vendor chipload band
//!   4. a generic chipload band
//! Returns `None` only when the tool's geometry is too incomplete to use —
//! never invents a number.

use super::model::{ChiploadRule, CuttingPreset, FeedCurve, LibraryTool, MachineRow, MaterialClassInfo, PresetDerivation, PresetOrigin};

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Preset { origin: PresetOrigin, name: String },
    /// A vendor feed-vs-depth curve.
    FeedCurve { vendor: Option<String> },
    /// A chipload band. `vendor: None` means the generic fallback.
    Rule { vendor: Option<String> },
}

/// Everything CAM needs, with a paper trail for where each number came from.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedCuttingData {
    pub spindle_rpm: i64,
    pub feed_xy_mm_min: f64,
    pub feed_z_mm_min: f64,
    pub chipload_mm: f64,
    pub stepdown_mm: f64,
    pub stepover_mm: f64,
    pub source: Source,
    pub confidence: i64,
    /// Human-readable notes on what was limited or estimated.
    pub clamped: Vec<String>,
}

impl ResolvedCuttingData {
    /// True when nothing better than a generic band was available — the UI
    /// must show these as estimates, not as data.
    pub fn is_estimate(&self) -> bool {
        match &self.source {
            Source::Preset { origin, .. } => matches!(origin, PresetOrigin::Derived | PresetOrigin::Generic),
            Source::FeedCurve { .. } => false, // published vendor data
            Source::Rule { .. } => true,
        }
    }
}

/// `depth_of_cut_mm` is the DOC the operation will run. Feed curves need it;
/// everything else ignores it. `None` evaluates a curve at its shallow
/// (fastest) end.
pub fn resolve(
    tool: &LibraryTool,
    material_class: &MaterialClassInfo,
    machine: Option<&MachineRow>,
    presets: &[CuttingPreset],
    rules: &[ChiploadRule],
    feed_curves: &[FeedCurve],
    depth_of_cut_mm: Option<f64>,
) -> Option<ResolvedCuttingData> {
    // ---- Layer 1: an explicit preset ------------------------------------
    let candidates: Vec<&CuttingPreset> = presets
        .iter()
        .filter(|p| {
            p.tool_id == tool.id
                && (p.material_class_id.is_none() || p.material_class_id.as_deref() == Some(material_class.id.as_str()))
                && (p.machine_id.is_none() || p.machine_id.as_deref() == machine.map(|m| m.id.as_str()))
        })
        .collect();
    if let Some(best) = candidates
        .into_iter()
        .max_by_key(|p| p.score(machine.map(|m| m.id.as_str())))
    {
        if let Some(resolved) = materialise(best, tool, machine, material_class) {
            return Some(resolved);
        }
    }

    let dia = tool.diameter_mm.filter(|d| *d > 0.0)?;
    let flutes = tool.flute_count.filter(|f| *f > 0)?;

    // ---- Layer 2: a vendor feed-vs-depth curve --------------------------
    let matching_curve = feed_curves
        .iter()
        .filter(|c| c.matches(tool))
        .max_by_key(|c| (c.vendor_id.is_some() as i32) * 2 + (c.series.is_some() as i32));
    if let Some(curve) = matching_curve {
        // When the caller has not fixed a DOC yet (browsing the library, not
        // inside an operation), evaluate the curve at the tool's recommended
        // DOC so the displayed feed matches a depth this tool would run.
        let doc = depth_of_cut_mm.or_else(|| tool.recommended_doc_mm());
        return Some(resolve_feed_curve(
            curve,
            tool,
            dia,
            flutes,
            machine,
            material_class,
            doc,
            depth_of_cut_mm.is_some(),
        ));
    }

    // ---- Layers 3 & 4: derive from a chipload band ----------------------
    let matching: Vec<&ChiploadRule> = rules
        .iter()
        .filter(|r| {
            r.material_class_id == material_class.id
                && dia >= r.dia_min_mm
                && dia <= r.dia_max_mm
                && (r.tool_type.is_none() || r.tool_type == Some(tool.tool_type))
                && (r.vendor_id.is_none() || r.vendor_id.as_deref() == tool.vendor_id.as_deref())
        })
        .collect();
    // Vendor-specific beats generic; type-specific beats any-type.
    let rule = matching
        .into_iter()
        .max_by_key(|r| (r.vendor_id.is_some() as i32) * 2 + (r.tool_type.is_some() as i32))?;

    let mut clamped: Vec<String> = Vec::new();

    // RPM: start at the rule's ceiling, then apply reality in order.
    let mut rpm = rule.rpm_max.or(machine.map(|m| m.maximum_spindle_rpm)).unwrap_or(18_000);
    if let Some(melt) = material_class.max_rpm_hint {
        if rpm > melt {
            rpm = melt;
            clamped.push(format!("RPM capped at {melt} — {} melts at higher speeds", material_class.name));
        }
    }
    if let Some(hi) = machine.map(|m| m.maximum_spindle_rpm) {
        if rpm > hi {
            rpm = hi;
            clamped.push(format!("RPM limited to the spindle maximum ({hi})"));
        }
    }
    if let Some(lo) = machine.map(|m| m.minimum_spindle_rpm) {
        if rpm < lo {
            rpm = lo;
            clamped.push(format!("RPM raised to the spindle minimum ({lo}) — feed re-derived to match"));
        }
    }

    // Chipload, derated for the machine's rigidity.
    let rigidity = machine.map(|m| m.rigidity_factor).unwrap_or(1.0);
    let fz = rule.fz_typ_mm * rigidity;
    if rigidity < 1.0 {
        clamped.push(format!("Chipload derated to {}% for machine rigidity", (rigidity * 100.0) as i64));
    }

    let mut feed = fz * flutes as f64 * rpm as f64;
    if let Some(max_feed) = machine.map(|m| m.maximum_feed) {
        if feed > max_feed {
            feed = max_feed;
            clamped.push(format!("Feed limited to the machine maximum ({} mm/min)", max_feed as i64));
        }
    }

    let mut plunge = feed * 0.35;
    if let Some(max_z) = machine.and_then(|m| m.maximum_plunge_feed) {
        if plunge > max_z {
            plunge = max_z;
            clamped.push(format!("Plunge limited to the Z maximum ({} mm/min)", max_z as i64));
        }
    }

    let stepdown = (rule.max_stepdown_x_dia.unwrap_or(1.0) * dia).min(tool.flute_length_mm.unwrap_or(f64::MAX));

    Some(ResolvedCuttingData {
        spindle_rpm: rpm,
        feed_xy_mm_min: feed.round(),
        feed_z_mm_min: plunge.round(),
        chipload_mm: fz,
        stepdown_mm: stepdown,
        stepover_mm: dia * 0.4,
        source: Source::Rule { vendor: rule.vendor_id.clone() },
        confidence: if rule.vendor_id.is_some() { 3 } else { 1 },
        clamped,
    })
}

/// Evaluate a vendor feed curve at the chosen DOC, then apply the material
/// and cross-grain factors and clamp to the machine. Rigidity is
/// deliberately NOT applied here — this is published vendor data, not a
/// generic guess, so it is trusted and only clamped to machine limits.
#[allow(clippy::too_many_arguments)]
fn resolve_feed_curve(
    curve: &FeedCurve,
    tool: &LibraryTool,
    dia: f64,
    flutes: i64,
    machine: Option<&MachineRow>,
    material_class: &MaterialClassInfo,
    depth_of_cut_mm: Option<f64>,
    doc_is_from_operation: bool,
) -> ResolvedCuttingData {
    let mut clamped: Vec<String> = Vec::new();

    let doc = depth_of_cut_mm.unwrap_or(curve.doc_lo_mm);
    if !doc_is_from_operation {
        clamped.push(format!(
            "Feed shown for the recommended DOC of {} mm; the operation sets the final depth",
            doc.round() as i64
        ));
    }
    if doc < curve.doc_lo_mm.min(curve.doc_hi_mm) || doc > curve.doc_lo_mm.max(curve.doc_hi_mm) {
        clamped.push(format!(
            "DOC {} mm is outside the vendor range {}–{} mm; feed held at the nearest published value",
            doc.round() as i64,
            curve.doc_lo_mm as i64,
            curve.doc_hi_mm as i64
        ));
    }

    let base = curve.base_feed(doc);
    let mut feed = base * material_class.feed_factor * curve.cross_grain_factor;
    if (material_class.feed_factor - 1.0).abs() > f64::EPSILON {
        clamped.push(format!("Feed x{} for {}", factor_text(material_class.feed_factor), material_class.name));
    }
    clamped.push(format!("Feed x{} for cross-grain cutting", factor_text(curve.cross_grain_factor)));

    let mut rpm = curve.rpm;
    if let Some(melt) = material_class.max_rpm_hint {
        if rpm > melt {
            rpm = melt;
            clamped.push(format!("RPM capped at {melt} — {} melts at higher speeds", material_class.name));
        }
    }
    if let Some(hi) = machine.map(|m| m.maximum_spindle_rpm) {
        if rpm > hi {
            rpm = hi;
            clamped.push(format!("RPM limited to the spindle maximum ({hi})"));
        }
    }
    if let Some(lo) = machine.map(|m| m.minimum_spindle_rpm) {
        if rpm < lo {
            rpm = lo;
            clamped.push(format!("RPM raised to the spindle minimum ({lo})"));
        }
    }

    if let Some(max_feed) = machine.map(|m| m.maximum_feed) {
        if feed > max_feed {
            feed = max_feed;
            clamped.push(format!(
                "Feed limited to the machine maximum ({} mm/min) — raise it in the machine settings if your CNC is faster",
                max_feed as i64
            ));
        }
    }

    let mut plunge = feed * 0.35;
    if let Some(max_z) = machine.and_then(|m| m.maximum_plunge_feed) {
        if plunge > max_z {
            plunge = max_z;
            clamped.push(format!("Plunge limited to the Z maximum ({} mm/min)", max_z as i64));
        }
    }

    let fz = if flutes > 0 { feed / (rpm as f64 * flutes as f64) } else { 0.0 };
    // Report the geometry-recommended DOC as the stepdown, bounded by the
    // cutting length. The operation overrides this before G-code.
    let recommended = tool.recommended_doc_mm().unwrap_or(doc);
    let stepdown = depth_of_cut_mm.unwrap_or(recommended).min(tool.flute_length_mm.unwrap_or(f64::MAX));

    ResolvedCuttingData {
        spindle_rpm: rpm,
        feed_xy_mm_min: feed.round(),
        feed_z_mm_min: plunge.round(),
        chipload_mm: fz,
        stepdown_mm: stepdown,
        stepover_mm: dia * 0.4,
        source: Source::FeedCurve { vendor: curve.vendor_id.clone() },
        confidence: 4,
        clamped,
    }
}

fn factor_text(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    let mut s = format!("{rounded}");
    if s.ends_with(".0") {
        s.truncate(s.len() - 2);
    }
    s
}

/// Turn a stored preset into usable numbers, filling any gap from chipload
/// and clamping to the machine.
fn materialise(
    p: &CuttingPreset,
    tool: &LibraryTool,
    machine: Option<&MachineRow>,
    material_class: &MaterialClassInfo,
) -> Option<ResolvedCuttingData> {
    let dia = tool.diameter_mm.filter(|d| *d > 0.0)?;
    let flutes = tool.flute_count.unwrap_or(0);
    let mut clamped: Vec<String> = Vec::new();

    let mut rpm = p.spindle_rpm.or(machine.map(|m| m.maximum_spindle_rpm)).unwrap_or(18_000);
    if let Some(melt) = material_class.max_rpm_hint {
        if rpm > melt {
            rpm = melt;
            clamped.push(format!("RPM capped at {melt} — {} melts at higher speeds", material_class.name));
        }
    }
    if let Some(hi) = machine.map(|m| m.maximum_spindle_rpm) {
        if rpm > hi {
            rpm = hi;
            clamped.push(format!("RPM limited to the spindle maximum ({hi})"));
        }
    }
    if let Some(lo) = machine.map(|m| m.minimum_spindle_rpm) {
        if rpm < lo {
            rpm = lo;
            clamped.push(format!("RPM raised to the spindle minimum ({lo})"));
        }
    }

    // Feed: recomputed from chipload when the preset says so, or when the
    // clamps above moved the RPM out from under a stored feed.
    let stored_feed = p.feed_xy_mm_min;
    let (mut feed, mut fz);
    if (p.derivation == PresetDerivation::FromChipload || stored_feed.is_none())
        && p.chipload_mm.is_some()
        && flutes > 0
    {
        fz = p.chipload_mm.unwrap();
        feed = fz * flutes as f64 * rpm as f64;
    } else if let Some(f) = stored_feed {
        feed = f;
        fz = if flutes > 0 { f / (rpm as f64 * flutes as f64) } else { p.chipload_mm.unwrap_or(0.0) };
    } else {
        return None;
    }

    if let Some(max_feed) = machine.map(|m| m.maximum_feed) {
        if feed > max_feed {
            feed = max_feed;
            clamped.push(format!("Feed limited to the machine maximum ({} mm/min)", max_feed as i64));
        }
    }

    let mut plunge = p.feed_z_mm_min.unwrap_or(feed * 0.35);
    if let Some(max_z) = machine.and_then(|m| m.maximum_plunge_feed) {
        if plunge > max_z {
            plunge = max_z;
            clamped.push(format!("Plunge limited to the Z maximum ({} mm/min)", max_z as i64));
        }
    }
    if plunge > feed {
        plunge = feed * 0.5;
        clamped.push("Plunge exceeded the cutting feed and was reduced".to_string());
    }
    let _ = &mut fz;

    Some(ResolvedCuttingData {
        spindle_rpm: rpm,
        feed_xy_mm_min: feed.round(),
        feed_z_mm_min: plunge.round(),
        chipload_mm: fz,
        stepdown_mm: p.stepdown_mm.unwrap_or_else(|| dia.min(tool.flute_length_mm.unwrap_or(dia))),
        stepover_mm: p.stepover_mm.unwrap_or(dia * 0.4),
        source: Source::Preset { origin: p.origin, name: p.name.clone() },
        confidence: p.confidence,
        clamped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool() -> LibraryTool {
        let mut t = LibraryTool::new("6mm Upcut", super::super::model::ToolType::EndMill);
        t.diameter_mm = Some(6.0);
        t.flute_count = Some(2);
        t.flute_length_mm = Some(20.0);
        t
    }

    fn material() -> MaterialClassInfo {
        MaterialClassInfo {
            id: "hardwood".into(),
            name: "Hardwood".into(),
            melts: false,
            prefers_downcut: false,
            max_rpm_hint: None,
            feed_factor: 0.9,
        }
    }

    #[test]
    fn explicit_preset_wins_over_everything_else() {
        let tool = tool();
        let preset = CuttingPreset {
            id: "p1".into(),
            tool_id: tool.id.clone(),
            material_id: None,
            material_class_id: Some("hardwood".into()),
            machine_id: None,
            name: "My trusted preset".into(),
            origin: PresetOrigin::User,
            derivation: PresetDerivation::Explicit,
            confidence: 5,
            chipload_mm: None,
            vc_m_per_min: None,
            spindle_rpm: Some(20_000),
            feed_xy_mm_min: Some(3_500.0),
            feed_z_mm_min: Some(1_000.0),
            ramp_feed_mm_min: None,
            stepdown_mm: None,
            stepover_mm: None,
            clearance_stepover_mm: None,
            cut_direction: None,
            air_blast: false,
            notes: None,
        };

        let resolved = resolve(&tool, &material(), None, &[preset], &[], &[], None).expect("resolved");
        assert!(!resolved.is_estimate());
        assert_eq!(resolved.spindle_rpm, 20_000);
        assert_eq!(resolved.feed_xy_mm_min, 3_500.0);
        assert!(matches!(resolved.source, Source::Preset { origin: PresetOrigin::User, .. }));
    }

    #[test]
    fn returns_none_when_geometry_is_incomplete() {
        let tool = LibraryTool::new("Unknown tool", super::super::model::ToolType::EndMill);
        let resolved = resolve(&tool, &material(), None, &[], &[], &[], None);
        assert!(resolved.is_none(), "must refuse rather than invent a number");
    }

    #[test]
    fn chipload_band_is_flagged_as_an_estimate() {
        let tool = tool();
        let rule = ChiploadRule {
            id: "r1".into(),
            material_class_id: "hardwood".into(),
            tool_type: None,
            vendor_id: None,
            dia_min_mm: 3.0,
            dia_max_mm: 6.5,
            fz_min_mm: 0.05,
            fz_typ_mm: 0.09,
            fz_max_mm: 0.15,
            rpm_min: Some(12_000),
            rpm_max: Some(20_000),
            max_stepdown_x_dia: Some(1.0),
        };
        let resolved = resolve(&tool, &material(), None, &[], &[rule], &[], None).expect("resolved");
        assert!(resolved.is_estimate());
        assert!(matches!(resolved.source, Source::Rule { vendor: None }));
    }
}
