//! A staged CSV import pipeline: parse -> validate -> diff against the
//! existing catalog -> review -> commit (or cancel/rollback).
//!
//! Shaped after 2DCam's `CSVImporter.swift`/`ToolValidator.swift`, but with a
//! single generic header-name column mapping instead of 2DCam's per-vendor
//! `CatalogProfile` system (Sorotec/CMT/Amana/Bits&Bits-specific column
//! layouts) — Mac2CAM doesn't ship those vendor catalogs, so that whole
//! profile-matching layer isn't ported. The staging/validation/diff/rollback
//! *shape* that actually matters for undo and per-row review is preserved.

use super::model::{LibraryTool, ToolType};
use super::repository::ToolLibraryRepository;
use super::{new_id, ValidationIssue, ValidationSeverity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// No tool with this name exists yet.
    New,
    /// Exists and every field matches.
    Unchanged,
    /// Exists and some fields differ.
    Changed,
    /// At least one validation error.
    Rejected,
}

impl Disposition {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Unchanged => "unchanged",
            Self::Changed => "changed",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiffValue {
    pub old: Option<String>,
    pub new: Option<String>,
}

/// A candidate tool, normalised to canonical units, not yet in the library.
#[derive(Clone, Debug, Default)]
pub struct ToolDraft {
    pub row_index: usize,
    pub name: String,
    pub series: Option<String>,
    pub tool_type: Option<ToolType>,
    pub diameter_mm: Option<f64>,
    pub corner_radius_mm: Option<f64>,
    pub included_angle_deg: Option<f64>,
    pub flute_count: Option<i64>,
    pub flute_length_mm: Option<f64>,
    pub shank_dia_mm: Option<f64>,
    pub overall_length_mm: Option<f64>,
    pub substrate: Option<String>,
    pub coating: Option<String>,
    pub notes: Option<String>,
    pub spindle_rpm: Option<i64>,
    pub feed_xy: Option<f64>,
    pub feed_z: Option<f64>,
    pub raw_json: String,
}

#[derive(Clone, Debug)]
pub struct ImportRow {
    pub draft: ToolDraft,
    pub issues: Vec<ValidationIssue>,
    pub disposition: Disposition,
    pub existing_tool_id: Option<String>,
    pub diff: Vec<(String, DiffValue)>,
    /// Whether this row will actually be written on commit.
    pub include: bool,
}

impl ImportRow {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == ValidationSeverity::Error)
    }
}

pub struct ImportPlan {
    pub batch_id: String,
    pub source_file_name: String,
    pub rows: Vec<ImportRow>,
}

impl ImportPlan {
    pub fn new_count(&self) -> usize {
        self.rows.iter().filter(|r| r.include && r.disposition == Disposition::New).count()
    }
    pub fn changed_count(&self) -> usize {
        self.rows.iter().filter(|r| r.include && r.disposition == Disposition::Changed).count()
    }
    pub fn rejected_count(&self) -> usize {
        self.rows.iter().filter(|r| r.disposition == Disposition::Rejected).count()
    }
}

pub struct ImportResult {
    pub batch_id: String,
    pub inserted: usize,
    pub updated: usize,
    pub rejected: usize,
}

/// Sanity limits mirroring 2DCam's `ValidationLimits` (a pragmatic subset).
struct ValidationLimits;
impl ValidationLimits {
    const DIAMETER_MIN: f64 = 0.4;
    const DIAMETER_MAX: f64 = 120.0;
    const FLUTE_MIN: i64 = 1;
    const FLUTE_MAX: i64 = 12;
    const RPM_MIN: i64 = 3_000;
    const RPM_MAX: i64 = 60_000;
    const FEED_MIN: f64 = 50.0;
    const FEED_MAX: f64 = 30_000.0;
}

/// Export the catalog back to the same generic column layout `plan()`
/// reads, so a round trip (export, edit in a spreadsheet, re-import) works.
pub fn to_csv(tools: &[&LibraryTool]) -> String {
    let mut out = String::from("name,type,diameter_mm,flute_count,flute_length,shank_diameter,overall_length,corner_radius,included_angle,substrate,coating,chip_direction,series\n");
    fn cell(s: &str) -> String {
        if s.contains(',') || s.contains('"') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
    fn opt(v: Option<impl std::fmt::Display>) -> String {
        v.map(|v| v.to_string()).unwrap_or_default()
    }
    for tool in tools {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            cell(&tool.name),
            tool.tool_type.as_db_str(),
            opt(tool.diameter_mm),
            opt(tool.flute_count),
            opt(tool.flute_length_mm),
            opt(tool.shank_dia_mm),
            opt(tool.overall_length_mm),
            opt(tool.corner_radius_mm),
            opt(tool.included_angle_deg),
            cell(tool.substrate.as_deref().unwrap_or_default()),
            cell(tool.coating.as_deref().unwrap_or_default()),
            cell(tool.chip_direction.as_deref().unwrap_or_default()),
            cell(tool.series.as_deref().unwrap_or_default()),
        ));
    }
    out
}

/// Parse a CSV file into a staged `ImportPlan`. Writes nothing.
/// Vendor CSV exports (this file included) commonly use ';' instead of ','
/// as the field separator — e.g. European/Excel locales that use ',' as the
/// decimal point. Sniff the header line rather than assuming a delimiter.
fn detect_delimiter(header_line: &str) -> char {
    if header_line.matches(';').count() > header_line.matches(',').count() {
        ';'
    } else {
        ','
    }
}

pub fn plan(repository: &ToolLibraryRepository, data: &str, file_name: &str) -> ImportPlan {
    let mut lines = data.lines().filter(|l| !l.trim().is_empty());
    let header_line = lines.next().unwrap_or_default();
    let delimiter = detect_delimiter(header_line);
    let headers: Vec<String> = header_line
        .split(delimiter)
        .map(|h| h.trim().to_lowercase())
        .collect();
    let column = |names: &[&str]| headers.iter().position(|h| names.contains(&h.as_str()));

    let name_col = column(&["name", "tool", "description", "código", "codigo"]);
    let type_col = column(&["type", "tool_type", "kind"]);
    let diameter_col = column(&["diameter", "diameter_mm", "cutting diameter", "cut diameter", "dia"]);
    let corner_radius_col = column(&["corner_radius", "corner_radius_mm", "tip radius"]);
    let angle_col = column(&["included_angle", "angle", "angle_deg"]);
    let flute_count_col = column(&["flutes", "flute_count", "number of flutes"]);
    let flute_length_col = column(&["flute_length", "flute_length_mm", "cutting length", "cut length"]);
    let shank_col = column(&["shank", "shank_dia", "shank_diameter", "shank_dia_mm"]);
    let overall_col = column(&["overall_length", "overall_length_mm", "oal", "total length"]);
    let substrate_col = column(&["substrate", "material"]);
    let coating_col = column(&["coating"]);
    let series_col = column(&["series"]);
    let notes_col = column(&["notes", "comment"]);
    let feed_col = column(&["feed", "feed_mm_min", "feed rate"]);
    let plunge_col = column(&["plunge", "plunge_feed", "plunge_mm_min"]);
    let rpm_col = column(&["rpm", "spindle_rpm", "spindle"]);

    let existing_by_name: std::collections::HashMap<String, LibraryTool> = repository
        .tools(false)
        .unwrap_or_default()
        .into_iter()
        .map(|t| (t.name.to_lowercase(), t))
        .collect();

    let mut rows = Vec::new();
    for (row_index, line) in lines.enumerate() {
        let cells: Vec<&str> = line.split(delimiter).map(str::trim).collect();
        if cells.iter().all(|c| c.is_empty()) {
            continue;
        }
        let get = |col: Option<usize>| col.and_then(|i| cells.get(i)).map(|s| s.trim()).filter(|s| !s.is_empty());
        let parse_f64 = |col: Option<usize>| get(col).and_then(|v| v.parse::<f64>().ok());
        let parse_i64 = |col: Option<usize>| get(col).and_then(|v| v.parse::<i64>().ok());

        let mut draft = ToolDraft {
            row_index,
            name: get(name_col).unwrap_or_default().to_string(),
            series: get(series_col).map(str::to_string),
            tool_type: get(type_col).and_then(|v| ToolType::from_db_str(&v.to_lowercase().replace(' ', "_"))),
            diameter_mm: parse_f64(diameter_col),
            corner_radius_mm: parse_f64(corner_radius_col),
            included_angle_deg: parse_f64(angle_col),
            flute_count: parse_i64(flute_count_col),
            flute_length_mm: parse_f64(flute_length_col),
            shank_dia_mm: parse_f64(shank_col),
            overall_length_mm: parse_f64(overall_col),
            substrate: get(substrate_col).map(str::to_string),
            coating: get(coating_col).map(str::to_string),
            notes: get(notes_col).map(str::to_string),
            spindle_rpm: parse_i64(rpm_col),
            feed_xy: parse_f64(feed_col),
            feed_z: parse_f64(plunge_col),
            raw_json: raw_json(&headers, &cells),
        };
        if draft.name.is_empty() {
            draft.name = format!("Imported tool {}", row_index + 1);
        }

        let issues = validate(&draft, row_index);
        let has_errors = issues.iter().any(|i| i.severity == ValidationSeverity::Error);

        let mut disposition = Disposition::New;
        let mut existing_tool_id = None;
        let mut diff = Vec::new();

        if has_errors {
            disposition = Disposition::Rejected;
        } else if let Some(existing) = existing_by_name.get(&draft.name.to_lowercase()) {
            existing_tool_id = Some(existing.id.clone());
            diff = diff_tool(existing, &draft);
            disposition = if diff.is_empty() { Disposition::Unchanged } else { Disposition::Changed };
        }

        let include = !matches!(disposition, Disposition::Rejected | Disposition::Unchanged);
        rows.push(ImportRow { draft, issues, disposition, existing_tool_id, diff, include });
    }

    ImportPlan {
        batch_id: new_id(),
        source_file_name: file_name.to_string(),
        rows,
    }
}

fn raw_json(headers: &[String], cells: &[&str]) -> String {
    let pairs: Vec<String> = headers
        .iter()
        .zip(cells.iter())
        .map(|(h, c)| format!("{:?}:{:?}", h, c))
        .collect();
    format!("{{{}}}", pairs.join(","))
}

fn validate(draft: &ToolDraft, row_index: usize) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let error = |code: &str, field: &str, message: String| ValidationIssue {
        row_index,
        severity: ValidationSeverity::Error,
        code: code.to_string(),
        field: Some(field.to_string()),
        raw_value: None,
        message,
    };
    let warning = |code: &str, field: &str, message: String| ValidationIssue {
        row_index,
        severity: ValidationSeverity::Warning,
        code: code.to_string(),
        field: Some(field.to_string()),
        raw_value: None,
        message,
    };

    if draft.name.trim().is_empty() {
        issues.push(error("MISSING_NAME", "name", "A tool needs a name".into()));
    }
    if let Some(d) = draft.diameter_mm {
        if !(ValidationLimits::DIAMETER_MIN..=ValidationLimits::DIAMETER_MAX).contains(&d) {
            issues.push(error(
                "DIAMETER_RANGE",
                "diameter_mm",
                format!("Diameter {d} mm is outside the plausible range ({}-{} mm)", ValidationLimits::DIAMETER_MIN, ValidationLimits::DIAMETER_MAX),
            ));
        }
    }
    if let Some(f) = draft.flute_count {
        if !(ValidationLimits::FLUTE_MIN..=ValidationLimits::FLUTE_MAX).contains(&f) {
            issues.push(warning("FLUTE_RANGE", "flute_count", format!("Flute count {f} is unusual")));
        }
    }
    if let Some(rpm) = draft.spindle_rpm {
        if !(ValidationLimits::RPM_MIN..=ValidationLimits::RPM_MAX).contains(&rpm) {
            issues.push(warning("RPM_RANGE", "spindle_rpm", format!("RPM {rpm} is outside the plausible range")));
        }
    }
    if let Some(feed) = draft.feed_xy {
        if !(ValidationLimits::FEED_MIN..=ValidationLimits::FEED_MAX).contains(&feed) {
            issues.push(warning("FEED_RANGE", "feed_xy", format!("Feed {feed} mm/min is outside the plausible range")));
        }
    }
    if draft.tool_type == Some(ToolType::BullNose) && draft.corner_radius_mm.is_none() {
        issues.push(warning("MISSING_CORNER_RADIUS", "corner_radius_mm", "Bull-nose tools need a corner radius".into()));
    }
    issues
}

fn diff_tool(existing: &LibraryTool, draft: &ToolDraft) -> Vec<(String, DiffValue)> {
    let mut out = Vec::new();
    let compare = |out: &mut Vec<(String, DiffValue)>, key: &str, a: Option<f64>, b: Option<f64>| {
        const TOLERANCE: f64 = 0.005;
        match (a, b) {
            (Some(x), Some(y)) if (x - y).abs() > TOLERANCE => {
                out.push((key.to_string(), DiffValue { old: Some(fmt(x)), new: Some(fmt(y)) }))
            }
            (None, Some(y)) => out.push((key.to_string(), DiffValue { old: None, new: Some(fmt(y)) })),
            (Some(x), None) => out.push((key.to_string(), DiffValue { old: Some(fmt(x)), new: None })),
            _ => {}
        }
    };
    fn fmt(v: f64) -> String {
        format!("{v:.3}").trim_end_matches('0').trim_end_matches('.').to_string()
    }

    compare(&mut out, "diameter_mm", existing.diameter_mm, draft.diameter_mm);
    compare(&mut out, "flute_length_mm", existing.flute_length_mm, draft.flute_length_mm);
    compare(&mut out, "shank_dia_mm", existing.shank_dia_mm, draft.shank_dia_mm);
    compare(&mut out, "overall_length_mm", existing.overall_length_mm, draft.overall_length_mm);
    compare(&mut out, "corner_radius_mm", existing.corner_radius_mm, draft.corner_radius_mm);
    compare(&mut out, "included_angle_deg", existing.included_angle_deg, draft.included_angle_deg);

    if existing.flute_count != draft.flute_count {
        out.push((
            "flute_count".into(),
            DiffValue {
                old: existing.flute_count.map(|v| v.to_string()),
                new: draft.flute_count.map(|v| v.to_string()),
            },
        ));
    }
    if let Some(t) = draft.tool_type {
        if existing.tool_type != t {
            out.push((
                "tool_type".into(),
                DiffValue { old: Some(existing.tool_type.as_db_str().to_string()), new: Some(t.as_db_str().to_string()) },
            ));
        }
    }
    out
}

/// One transaction. On any failure, nothing lands. Changed rows update
/// geometry only, and never a field the user has edited by hand
/// (`field_provenance` origin='user').
pub fn commit(repository: &mut ToolLibraryRepository, plan: ImportPlan, group_id: Option<&str>) -> Result<ImportResult, super::repository::ToolLibraryError> {
    let group_id = group_id.map(|s| s.to_string());
    repository.transaction(move |repo| {
        let source_id = repo.upsert_data_source(None, "csv", &format!("CSV import — {}", plan.source_file_name), &plan.batch_id, 3)?;
        repo.create_batch(&plan.batch_id, &source_id, plan.rows.len() as i64)?;

        let mut inserted = 0usize;
        let mut updated = 0usize;
        let mut rejected = 0usize;

        for row in &plan.rows {
            let draft = &row.draft;
            let mut tool_id: Option<String> = None;

            if row.include && !row.has_errors() {
                match row.disposition {
                    Disposition::New => {
                        let mut tool = LibraryTool::new(draft.name.clone(), draft.tool_type.unwrap_or_default());
                        tool.series = draft.series.clone();
                        tool.diameter_mm = draft.diameter_mm;
                        tool.corner_radius_mm = draft.corner_radius_mm;
                        tool.included_angle_deg = draft.included_angle_deg;
                        tool.flute_count = draft.flute_count;
                        tool.flute_length_mm = draft.flute_length_mm;
                        tool.shank_dia_mm = draft.shank_dia_mm;
                        tool.overall_length_mm = draft.overall_length_mm;
                        tool.substrate = draft.substrate.clone();
                        tool.coating = draft.coating.clone();
                        tool.notes = draft.notes.clone();

                        repo.insert(&tool, Some(&source_id), Some(&plan.batch_id))?;
                        tool_id = Some(tool.id.clone());
                        if let Some(group_id) = &group_id {
                            repo.add_to_group(&tool.id, group_id)?;
                        }
                        if let (Some(feed), Some(rpm)) = (draft.feed_xy, draft.spindle_rpm) {
                            let fz = draft.flute_count.filter(|f| *f > 0).map(|f| feed / (rpm as f64 * f as f64));
                            let preset = super::model::CuttingPreset {
                                id: new_id(),
                                tool_id: tool.id.clone(),
                                material_id: None,
                                material_class_id: None,
                                machine_id: None,
                                name: format!("CSV import ({})", plan.source_file_name),
                                origin: super::model::PresetOrigin::Vendor,
                                derivation: super::model::PresetDerivation::FromChipload,
                                confidence: 4,
                                chipload_mm: fz,
                                vc_m_per_min: None,
                                spindle_rpm: Some(rpm),
                                feed_xy_mm_min: Some(feed),
                                feed_z_mm_min: draft.feed_z,
                                ramp_feed_mm_min: None,
                                stepdown_mm: None,
                                stepover_mm: None,
                                clearance_stepover_mm: None,
                                cut_direction: None,
                                air_blast: false,
                                notes: None,
                            };
                            repo.upsert(&preset, Some(&source_id), Some(&plan.batch_id))?;
                        }
                        inserted += 1;
                    }
                    Disposition::Changed => {
                        if let Some(id) = &row.existing_tool_id {
                            if let Some(mut existing) = repo.tools(false)?.into_iter().find(|t| &t.id == id) {
                                let protected = repo.user_edited_fields("tool", id)?;
                                for (field, _) in &row.diff {
                                    if protected.contains(field) {
                                        continue;
                                    }
                                    apply_field(field, draft, &mut existing);
                                }
                                repo.update(&existing, &[])?;
                                tool_id = Some(id.clone());
                                updated += 1;
                            }
                        }
                    }
                    _ => {}
                }
            } else if row.has_errors() {
                rejected += 1;
            }

            repo.stage(&plan.batch_id, draft.row_index as i64, &draft.raw_json, row.disposition.as_db_str(), tool_id.as_deref())?;
            for issue in &row.issues {
                repo.record_issue(issue, &plan.batch_id)?;
            }
        }

        repo.finish_batch(&plan.batch_id, "committed", (inserted + updated) as i64)?;
        Ok(ImportResult { batch_id: plan.batch_id.clone(), inserted, updated, rejected })
    })
}

fn apply_field(field: &str, draft: &ToolDraft, tool: &mut LibraryTool) {
    match field {
        "diameter_mm" => tool.diameter_mm = draft.diameter_mm,
        "flute_length_mm" => tool.flute_length_mm = draft.flute_length_mm,
        "shank_dia_mm" => tool.shank_dia_mm = draft.shank_dia_mm,
        "overall_length_mm" => tool.overall_length_mm = draft.overall_length_mm,
        "corner_radius_mm" => tool.corner_radius_mm = draft.corner_radius_mm,
        "included_angle_deg" => tool.included_angle_deg = draft.included_angle_deg,
        "flute_count" => tool.flute_count = draft.flute_count,
        "tool_type" => {
            if let Some(t) = draft.tool_type {
                tool.tool_type = t;
            }
        }
        "name" => tool.name = draft.name.clone(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_library::repository::ToolLibraryRepository;

    fn temp_repository() -> (ToolLibraryRepository, std::path::PathBuf) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mac2cam-csv-import-test-{}-{n}-{:?}.sqlite",
            new_id(),
            std::thread::current().id()
        ));
        let repo = ToolLibraryRepository::open(&path, false).expect("open repository");
        (repo, path)
    }

    #[test]
    fn plan_handles_semicolon_delimited_vendor_export() {
        let (repo, path) = temp_repository();
        let csv = "CÓDIGO;Cut Diameter;Cut length;Total length;Shank\n193.120.11;12;35;83;12\n193.121.11;12;42;90;12\n";
        let plan = plan(&repo, csv, "cmt_193_helical.csv");

        assert_eq!(plan.rows.len(), 2);
        assert_eq!(plan.new_count(), 2);
        assert_eq!(plan.rows[0].draft.name, "193.120.11");
        assert_eq!(plan.rows[0].draft.diameter_mm, Some(12.0));
        assert_eq!(plan.rows[0].draft.flute_length_mm, Some(35.0));
        assert_eq!(plan.rows[0].draft.overall_length_mm, Some(83.0));
        assert_eq!(plan.rows[0].draft.shank_dia_mm, Some(12.0));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn plan_stages_valid_rows_and_rejects_bad_ones() {
        let (repo, path) = temp_repository();
        let csv = "name,diameter_mm,flute_count,feed,rpm\nSix,6,2,900,18000\nBad,0,2,1,1\n";
        let plan = plan(&repo, csv, "tools.csv");

        assert_eq!(plan.rows.len(), 2);
        assert_eq!(plan.new_count(), 1);
        assert_eq!(plan.rejected_count(), 1);
        assert!(plan.rows[0].include);
        assert!(!plan.rows[1].include);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn commit_inserts_new_rows_and_rollback_undoes_them() {
        let (mut repo, path) = temp_repository();
        let csv = "name,diameter_mm,flute_count\nImported Tool,8,2\n";
        let plan_result = plan(&repo, csv, "tools.csv");
        let batch_id = plan_result.batch_id.clone();

        let result = commit(&mut repo, plan_result, Some("g-mine")).expect("commit");
        assert_eq!(result.inserted, 1);
        assert_eq!(repo.tools(false).expect("tools").len(), 1);

        repo.rollback(&batch_id).expect("rollback");
        assert_eq!(repo.tools(false).expect("tools").len(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reimport_marks_existing_tool_changed_and_protects_user_edits() {
        let (mut repo, path) = temp_repository();
        let first = plan(&repo, "name,diameter_mm,flute_count\nUpcut,6,2\n", "v1.csv");
        commit(&mut repo, first, None).expect("first commit");

        // The user hand-corrects the flute count after import.
        let mut tool = repo.tools(false).expect("tools").remove(0);
        tool.flute_count = Some(3);
        repo.update(&tool, &["flute_count"]).expect("user edit");

        // A re-import with a different diameter AND a different (vendor)
        // flute count must update the diameter but leave the user's flute
        // count alone.
        let second = plan(&repo, "name,diameter_mm,flute_count\nUpcut,6.35,4\n", "v2.csv");
        assert_eq!(second.rows[0].disposition, Disposition::Changed);
        commit(&mut repo, second, None).expect("second commit");

        let updated = repo.tools(false).expect("tools").remove(0);
        assert_eq!(updated.diameter_mm, Some(6.35));
        assert_eq!(updated.flute_count, Some(3), "user-edited flute count must survive a re-import");

        let _ = std::fs::remove_file(path);
    }
}
