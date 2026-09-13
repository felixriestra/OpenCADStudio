//! Ported from 2DCam's `Sources/TwoDCamCore/ToolLibrary/ToolLibraryRepository.swift`.
//! Uses `rusqlite` directly (no ORM), same shape of operations as the Swift
//! original. Mac2CAM's app state is single-threaded (iced's update loop), so
//! this needs no lock — the Swift version's `NSRecursiveLock` has no Rust
//! equivalent here.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use super::model::{
    ChiploadRule, CuttingPreset, DisplayUnits, FeedCurve, FormProfilePoint, FormScaleMode,
    LibraryTool, MachineRow, MaterialClassInfo, PresetDerivation, PresetOrigin, ToolType,
};
use super::resolver::{self, ResolvedCuttingData};
use super::{new_id, ValidationIssue};

#[derive(Debug)]
pub enum ToolLibraryError {
    Open(String),
    Sql(String),
    Corrupt(String),
}

impl fmt::Display for ToolLibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(m) => write!(f, "Unable to open the tool library: {m}"),
            Self::Sql(m) => write!(f, "Tool library operation failed: {m}"),
            Self::Corrupt(m) => write!(f, "The tool library is damaged: {m}"),
        }
    }
}

impl std::error::Error for ToolLibraryError {}

impl From<rusqlite::Error> for ToolLibraryError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value.to_string())
    }
}

type Result<T> = std::result::Result<T, ToolLibraryError>;

pub struct ToolLibraryRepository {
    conn: Connection,
    /// Transactions nest: an importer's whole-batch commit wraps per-tool
    /// calls (update, duplicate) that open transactions of their own. SQLite
    /// has no nested BEGIN, so only the outermost one is real.
    transaction_depth: i32,
    pub path: PathBuf,
}

impl ToolLibraryRepository {
    pub fn open(path: &Path, seed_starter_tools: bool) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ToolLibraryError::Open(e.to_string()))?;
        }
        let is_new = !path.exists();

        let conn = Connection::open(path).map_err(|e| ToolLibraryError::Open(e.to_string()))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        let mut repo = Self {
            conn,
            transaction_depth: 0,
            path: path.to_path_buf(),
        };

        if is_new {
            repo.transaction(|repo| {
                repo.conn.execute_batch(super::schema::DDL)?;
                repo.conn.execute_batch(super::schema::SEED)?;
                Ok(())
            })?;
        } else {
            repo.migrate_if_needed()?;
        }

        if seed_starter_tools {
            repo.seed_starter_tools_if_needed()?;
        }

        Ok(repo)
    }

    // MARK: - Migration

    fn migrate_if_needed(&mut self) -> Result<()> {
        let current: i64 = self
            .conn
            .query_row("SELECT MAX(version) FROM schema_migration", [], |r| r.get(0))
            .unwrap_or(1);
        for (version, name, sql) in super::schema::MIGRATIONS {
            if *version > current {
                let sql = *sql;
                let name = *name;
                self.transaction(move |repo| {
                    repo.conn.execute_batch(sql)?;
                    repo.conn.execute(
                        "INSERT INTO schema_migration (version, name) VALUES (?1, ?2)",
                        params![version, name],
                    )?;
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    // MARK: - Integrity & backup

    /// Run on open. `false` means: restore the newest backup.
    pub fn quick_check(&self) -> Result<bool> {
        let ok: String = self.conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        Ok(ok == "ok")
    }

    /// Cheap, consistent snapshot. Call on launch and after every import.
    pub fn backup(&self, path: &Path) -> Result<()> {
        let escaped = path.to_string_lossy().replace('\'', "''");
        self.conn.execute_batch(&format!("VACUUM INTO '{escaped}'"))?;
        Ok(())
    }

    /// Keeps the newest `keep` snapshots in `Backups/`, drops the rest.
    pub fn rotating_backup(&self, keep: usize) -> Result<PathBuf> {
        let dir = self
            .path
            .parent()
            .map(|p| p.join("Backups"))
            .ok_or_else(|| ToolLibraryError::Open("no parent directory".into()))?;
        fs::create_dir_all(&dir).map_err(|e| ToolLibraryError::Open(e.to_string()))?;

        let stamp = chrono_stamp();
        let target = dir.join(format!("ToolLibrary-{stamp}.sqlite"));
        self.backup(&target)?;

        if let Ok(entries) = fs::read_dir(&dir) {
            let mut names: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("ToolLibrary-"))
                        .unwrap_or(false)
                })
                .collect();
            names.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            for stale in names.into_iter().skip(keep) {
                let _ = fs::remove_file(stale);
            }
        }
        Ok(target)
    }

    // MARK: - Plumbing

    pub fn transaction<T>(&mut self, body: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.transaction_depth > 0 {
            self.transaction_depth += 1;
            let result = body(self);
            self.transaction_depth -= 1;
            return result;
        }

        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        self.transaction_depth = 1;
        match body(self) {
            Ok(value) => {
                self.transaction_depth = 0;
                self.conn.execute_batch("COMMIT")?;
                Ok(value)
            }
            Err(error) => {
                self.transaction_depth = 0;
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    // MARK: - Tools

    const TOOL_COLUMNS: &'static str = "
        t.id, t.vendor_id, t.product_id, t.product_url, t.name, t.series, t.tool_type,
        t.diameter_mm, t.corner_radius_mm, t.included_angle_deg, t.tip_dia_mm,
        t.flute_count, t.flute_length_mm, t.shank_dia_mm, t.overall_length_mm, t.neck_length_mm,
        t.chip_direction, t.substrate, t.coating, t.display_units, t.nominal_label, t.notes,
        t.created_at, t.updated_at, t.deleted_at, t.form_scale_mode, v.name";

    fn read_tool(row: &rusqlite::Row) -> rusqlite::Result<LibraryTool> {
        Ok(LibraryTool {
            id: row.get(0)?,
            vendor_id: row.get(1)?,
            product_id: row.get(2)?,
            product_url: row.get(3)?,
            name: row.get(4)?,
            series: row.get(5)?,
            tool_type: row
                .get::<_, String>(6)
                .ok()
                .and_then(|s| ToolType::from_db_str(&s))
                .unwrap_or_default(),
            diameter_mm: row.get(7)?,
            corner_radius_mm: row.get(8)?,
            included_angle_deg: row.get(9)?,
            tip_dia_mm: row.get(10)?,
            flute_count: row.get(11)?,
            flute_length_mm: row.get(12)?,
            shank_dia_mm: row.get(13)?,
            overall_length_mm: row.get(14)?,
            neck_length_mm: row.get(15)?,
            chip_direction: row.get(16)?,
            substrate: row.get(17)?,
            coating: row.get(18)?,
            display_units: row
                .get::<_, Option<String>>(19)?
                .map(|s| DisplayUnits::from_db_str(&s))
                .unwrap_or_default(),
            nominal_label: row.get(20)?,
            notes: row.get(21)?,
            created_at: row.get(22)?,
            updated_at: row.get(23)?,
            deleted_at: row.get(24)?,
            form_scale_mode: row
                .get::<_, Option<String>>(25)?
                .map(|s| FormScaleMode::from_db_str(&s))
                .unwrap_or_default(),
            vendor_name: row.get(26)?,
            form_profile_points: None,
        })
    }

    /// The drawn moulding silhouette for a form tool, read from
    /// `tool_profile_point`. Loaded in a second pass to keep `read_tool` a
    /// pure row map. Empty for non-form tools.
    fn form_profile_points(&self, tool_id: &str) -> Result<Vec<FormProfilePoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT radius_mm, z_mm FROM tool_profile_point WHERE tool_id = ?1 ORDER BY idx",
        )?;
        let points = stmt
            .query_map(params![tool_id], |r| {
                Ok(FormProfilePoint {
                    radius_mm: r.get(0)?,
                    rise_mm: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(points)
    }

    fn attach_profiles(&self, mut tools: Vec<LibraryTool>) -> Result<Vec<LibraryTool>> {
        for tool in &mut tools {
            if tool.tool_type == ToolType::Form {
                let points = self.form_profile_points(&tool.id)?;
                tool.form_profile_points = if points.is_empty() { None } else { Some(points) };
            }
        }
        Ok(tools)
    }

    pub fn tools(&self, include_deleted: bool) -> Result<Vec<LibraryTool>> {
        let filter = if include_deleted { "" } else { "WHERE t.deleted_at IS NULL" };
        let sql = format!(
            "SELECT {} FROM tool t LEFT JOIN vendor v ON v.id = t.vendor_id {} ORDER BY v.name, t.name COLLATE NOCASE",
            Self::TOOL_COLUMNS,
            filter
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let tools = stmt
            .query_map([], Self::read_tool)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        self.attach_profiles(tools)
    }

    pub fn trashed_tools(&self) -> Result<Vec<LibraryTool>> {
        let sql = format!(
            "SELECT {} FROM tool t LEFT JOIN vendor v ON v.id = t.vendor_id WHERE t.deleted_at IS NOT NULL ORDER BY t.deleted_at DESC",
            Self::TOOL_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let tools = stmt
            .query_map([], Self::read_tool)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        self.attach_profiles(tools)
    }

    pub fn tool_by_vendor_product(&self, vendor_id: &str, product_id: &str) -> Result<Option<LibraryTool>> {
        let sql = format!(
            "SELECT {} FROM tool t LEFT JOIN vendor v ON v.id = t.vendor_id WHERE t.vendor_id = ?1 AND t.product_id = ?2 AND t.deleted_at IS NULL",
            Self::TOOL_COLUMNS
        );
        let found = self
            .conn
            .query_row(&sql, params![vendor_id, product_id], Self::read_tool)
            .optional()?;
        match found {
            Some(tool) => Ok(self.attach_profiles(vec![tool])?.into_iter().next()),
            None => Ok(None),
        }
    }

    pub fn insert(&mut self, tool: &LibraryTool, source_id: Option<&str>, batch_id: Option<&str>) -> Result<()> {
        let vendor_id = self.resolved_vendor_id(tool)?;
        self.conn.execute(
            "INSERT INTO tool (id, vendor_id, product_id, product_url, name, series, tool_type,
                diameter_mm, corner_radius_mm, included_angle_deg, tip_dia_mm, flute_count,
                flute_length_mm, shank_dia_mm, overall_length_mm, neck_length_mm,
                chip_direction, substrate, coating, display_units, nominal_label, notes,
                form_scale_mode, source_id, batch_id, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,datetime('now'),datetime('now'))",
            params![
                tool.id,
                vendor_id,
                tool.product_id,
                tool.product_url,
                tool.name,
                tool.series,
                tool.tool_type.as_db_str(),
                tool.diameter_mm,
                tool.corner_radius_mm,
                tool.included_angle_deg,
                tool.tip_dia_mm,
                tool.flute_count,
                tool.flute_length_mm,
                tool.shank_dia_mm,
                tool.overall_length_mm,
                tool.neck_length_mm,
                tool.chip_direction,
                tool.substrate,
                tool.coating,
                tool.display_units.as_db_str(),
                tool.nominal_label,
                tool.notes,
                tool.form_scale_mode.as_db_str(),
                source_id,
                batch_id,
            ],
        )?;
        self.write_profile_points(&tool.id, tool.form_profile_points.as_deref().unwrap_or(&[]))?;
        Ok(())
    }

    /// Replaces the stored silhouette for a tool so `tool_profile_point`
    /// always matches the tool's current profile.
    fn write_profile_points(&mut self, tool_id: &str, points: &[FormProfilePoint]) -> Result<()> {
        self.conn
            .execute("DELETE FROM tool_profile_point WHERE tool_id = ?1", params![tool_id])?;
        for (i, pt) in points.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO tool_profile_point (tool_id, idx, z_mm, radius_mm) VALUES (?1,?2,?3,?4)",
                params![tool_id, i as i64, pt.rise_mm.max(0.0), pt.radius_mm.max(0.0)],
            )?;
        }
        Ok(())
    }

    /// Vendor is a normalised table. A tool carries a display name; this maps
    /// it to a vendor row — matching an existing vendor by name, or creating
    /// one if it is genuinely new. An explicit `vendor_id` always wins.
    fn resolved_vendor_id(&mut self, tool: &LibraryTool) -> Result<Option<String>> {
        if let Some(id) = &tool.vendor_id {
            if !id.is_empty() {
                return Ok(Some(id.clone()));
            }
        }
        let Some(name) = tool.vendor_name.as_ref().map(|n| n.trim()).filter(|n| !n.is_empty())
        else {
            return Ok(None);
        };

        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM vendor WHERE name = ?1 COLLATE NOCASE",
                params![name],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return Ok(Some(existing));
        }

        let generated = new_id();
        let id = format!("v-{}", &generated[..8.min(generated.len())]);
        self.conn.execute(
            "INSERT INTO vendor (id, name, native_units) VALUES (?1, ?2, 'mm')",
            params![id, name],
        )?;
        Ok(Some(id))
    }

    /// Inserts a small default tool set the first time the library is
    /// opened, so a fresh install has usable tools straight away. Guarded by
    /// an `app_setting` flag so it runs exactly once and never resurrects
    /// tools the user deletes.
    pub fn seed_starter_tools_if_needed(&mut self) -> Result<()> {
        let already: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM app_setting WHERE key = 'starter_tools_seeded'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if already.is_some() {
            return Ok(());
        }
        self.transaction(|repo| {
            for tool in Self::starter_tools() {
                repo.insert(&tool, None, None)?;
                let _ = repo.add_to_group(&tool.id, "g-mine");
            }
            repo.conn.execute(
                "INSERT OR REPLACE INTO app_setting (key, value) VALUES ('starter_tools_seeded','1')",
                [],
            )?;
            Ok(())
        })
    }

    fn starter_tools() -> Vec<LibraryTool> {
        let mut tools = Vec::new();
        for (name, diameter, flute_length, shank, overall) in [
            ("16 mm End Mill", 16.0, 32.0, 16.0, 80.0),
            ("8 mm End Mill", 8.0, 25.0, 8.0, 63.0),
            ("6 mm End Mill", 6.0, 20.0, 6.0, 50.0),
        ] {
            let mut tool = LibraryTool::new(name, ToolType::EndMill);
            tool.diameter_mm = Some(diameter);
            tool.flute_count = Some(2);
            tool.flute_length_mm = Some(flute_length);
            tool.shank_dia_mm = Some(shank);
            tool.overall_length_mm = Some(overall);
            tool.substrate = Some("solid_carbide".into());
            tools.push(tool);
        }
        let mut ball_nose = LibraryTool::new("3 mm Ball Nose", ToolType::BallNose);
        ball_nose.diameter_mm = Some(3.0);
        ball_nose.corner_radius_mm = Some(1.5);
        ball_nose.flute_count = Some(2);
        ball_nose.flute_length_mm = Some(12.0);
        ball_nose.shank_dia_mm = Some(6.0);
        ball_nose.overall_length_mm = Some(38.0);
        ball_nose.substrate = Some("solid_carbide".into());
        tools.push(ball_nose);
        tools
    }

    /// Every field the user touches here is recorded as `origin='user'` in
    /// `field_provenance`, which is what stops a later catalogue re-import
    /// from overwriting a value dialled in by hand.
    pub fn update(&mut self, tool: &LibraryTool, mark_user_edited: &[&str]) -> Result<()> {
        let mark_user_edited: Vec<String> = mark_user_edited.iter().map(|s| s.to_string()).collect();
        let tool = tool.clone();
        self.transaction(move |repo| {
            let vendor_id = repo.resolved_vendor_id(&tool)?;
            repo.conn.execute(
                "UPDATE tool SET vendor_id=?1, product_id=?2, product_url=?3, name=?4, series=?5, tool_type=?6,
                    diameter_mm=?7, corner_radius_mm=?8, included_angle_deg=?9, tip_dia_mm=?10, flute_count=?11,
                    flute_length_mm=?12, shank_dia_mm=?13, overall_length_mm=?14, neck_length_mm=?15,
                    chip_direction=?16, substrate=?17, coating=?18, display_units=?19, nominal_label=?20, notes=?21,
                    form_scale_mode=?22, updated_at=datetime('now')
                 WHERE id=?23",
                params![
                    vendor_id,
                    tool.product_id,
                    tool.product_url,
                    tool.name,
                    tool.series,
                    tool.tool_type.as_db_str(),
                    tool.diameter_mm,
                    tool.corner_radius_mm,
                    tool.included_angle_deg,
                    tool.tip_dia_mm,
                    tool.flute_count,
                    tool.flute_length_mm,
                    tool.shank_dia_mm,
                    tool.overall_length_mm,
                    tool.neck_length_mm,
                    tool.chip_direction,
                    tool.substrate,
                    tool.coating,
                    tool.display_units.as_db_str(),
                    tool.nominal_label,
                    tool.notes,
                    tool.form_scale_mode.as_db_str(),
                    tool.id,
                ],
            )?;
            repo.write_profile_points(&tool.id, tool.form_profile_points.as_deref().unwrap_or(&[]))?;
            for field in &mark_user_edited {
                repo.record_provenance("tool", &tool.id, field, "user", None)?;
            }
            Ok(())
        })
    }

    /// Soft delete: recoverable from the Trash.
    pub fn delete(&mut self, tool_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tool SET deleted_at=datetime('now'), updated_at=datetime('now') WHERE id=?1",
            params![tool_id],
        )?;
        Ok(())
    }

    pub fn restore(&mut self, tool_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tool SET deleted_at=NULL, updated_at=datetime('now') WHERE id=?1",
            params![tool_id],
        )?;
        Ok(())
    }

    /// Hard delete, from the Trash only. The `change_log` keeps the record.
    pub fn purge(&mut self, tool_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM tool WHERE id=?1 AND deleted_at IS NOT NULL",
            params![tool_id],
        )?;
        Ok(())
    }

    pub fn duplicate(&mut self, tool_id: &str) -> Result<Option<LibraryTool>> {
        let Some(mut copy) = self.tools(false)?.into_iter().find(|t| t.id == tool_id) else {
            return Ok(None);
        };
        let presets = self.presets(Some(tool_id))?;
        self.transaction(move |repo| {
            let new_tool_id = new_id();
            copy.id = new_tool_id.clone();
            copy.name = format!("{} copy", copy.name);
            copy.product_id = None; // a copy is not the vendor's part any more
            repo.insert(&copy, None, None)?;
            for mut preset in presets {
                preset.id = new_id();
                preset.tool_id = new_tool_id.clone();
                preset.origin = PresetOrigin::User;
                repo.upsert(&preset, None, None)?;
            }
            Ok(Some(copy))
        })
    }

    // MARK: - Presets

    pub fn presets(&self, tool_id: Option<&str>) -> Result<Vec<CuttingPreset>> {
        let sql = "SELECT id, tool_id, material_id, material_class_id, machine_id, name, origin, derivation,
                   confidence, chipload_fz_mm, vc_m_min, spindle_rpm, feed_xy_mm_min, feed_z_mm_min,
                   ramp_feed_mm_min, stepdown_mm, stepover_mm, clearance_stepover_mm, cut_direction,
                   air_blast, notes
              FROM cutting_preset WHERE deleted_at IS NULL"
            .to_string();
        let sql = match tool_id {
            Some(_) => format!("{sql} AND tool_id = ?1"),
            None => sql,
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let read = |row: &rusqlite::Row| -> rusqlite::Result<CuttingPreset> {
            Ok(CuttingPreset {
                id: row.get(0)?,
                tool_id: row.get(1)?,
                material_id: row.get(2)?,
                material_class_id: row.get(3)?,
                machine_id: row.get(4)?,
                name: row.get(5)?,
                origin: row
                    .get::<_, String>(6)
                    .map(|s| PresetOrigin::from_db_str(&s))
                    .unwrap_or(PresetOrigin::User),
                derivation: row
                    .get::<_, String>(7)
                    .map(|s| PresetDerivation::from_db_str(&s))
                    .unwrap_or(PresetDerivation::Explicit),
                confidence: row.get(8)?,
                chipload_mm: row.get(9)?,
                vc_m_per_min: row.get(10)?,
                spindle_rpm: row.get(11)?,
                feed_xy_mm_min: row.get(12)?,
                feed_z_mm_min: row.get(13)?,
                ramp_feed_mm_min: row.get(14)?,
                stepdown_mm: row.get(15)?,
                stepover_mm: row.get(16)?,
                clearance_stepover_mm: row.get(17)?,
                cut_direction: row.get(18)?,
                air_blast: row.get::<_, i64>(19)? == 1,
                notes: row.get(20)?,
            })
        };
        let presets = match tool_id {
            Some(id) => stmt
                .query_map(params![id], read)?
                .collect::<rusqlite::Result<Vec<_>>>()?,
            None => stmt.query_map([], read)?.collect::<rusqlite::Result<Vec<_>>>()?,
        };
        Ok(presets)
    }

    pub fn upsert(&mut self, p: &CuttingPreset, source_id: Option<&str>, batch_id: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO cutting_preset (id, tool_id, material_id, material_class_id, machine_id, name,
                origin, derivation, confidence, chipload_fz_mm, vc_m_min, spindle_rpm, feed_xy_mm_min,
                feed_z_mm_min, ramp_feed_mm_min, stepdown_mm, stepover_mm, clearance_stepover_mm,
                cut_direction, air_blast, notes, source_id, batch_id, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,datetime('now'),datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                material_id=excluded.material_id, material_class_id=excluded.material_class_id,
                machine_id=excluded.machine_id, name=excluded.name, origin=excluded.origin,
                derivation=excluded.derivation, confidence=excluded.confidence,
                chipload_fz_mm=excluded.chipload_fz_mm, vc_m_min=excluded.vc_m_min,
                spindle_rpm=excluded.spindle_rpm, feed_xy_mm_min=excluded.feed_xy_mm_min,
                feed_z_mm_min=excluded.feed_z_mm_min, ramp_feed_mm_min=excluded.ramp_feed_mm_min,
                stepdown_mm=excluded.stepdown_mm, stepover_mm=excluded.stepover_mm,
                clearance_stepover_mm=excluded.clearance_stepover_mm,
                cut_direction=excluded.cut_direction, air_blast=excluded.air_blast,
                notes=excluded.notes, updated_at=datetime('now')",
            params![
                p.id,
                p.tool_id,
                p.material_id,
                p.material_class_id,
                p.machine_id,
                p.name,
                p.origin.as_db_str(),
                p.derivation.as_db_str(),
                p.confidence,
                p.chipload_mm,
                p.vc_m_per_min,
                p.spindle_rpm,
                p.feed_xy_mm_min,
                p.feed_z_mm_min,
                p.ramp_feed_mm_min,
                p.stepdown_mm,
                p.stepover_mm,
                p.clearance_stepover_mm,
                p.cut_direction,
                p.air_blast as i64,
                p.notes,
                source_id,
                batch_id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_preset(&mut self, id: &str) -> Result<()> {
        self.conn
            .execute("UPDATE cutting_preset SET deleted_at=datetime('now') WHERE id=?1", params![id])?;
        Ok(())
    }

    // MARK: - Reference data

    pub fn material_classes(&self) -> Result<Vec<MaterialClassInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, melts, prefers_downcut, max_rpm_hint, feed_factor
               FROM material_class ORDER BY sort_order",
        )?;
        let classes = stmt
            .query_map([], |r| {
                Ok(MaterialClassInfo {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    melts: r.get::<_, i64>(2)? == 1,
                    prefers_downcut: r.get::<_, i64>(3)? == 1,
                    max_rpm_hint: r.get(4)?,
                    feed_factor: r.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(classes)
    }

    pub fn feed_curves(&self) -> Result<Vec<FeedCurve>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, vendor_id, series, tool_type, dia_min_mm, dia_max_mm, rpm,
                    doc_lo_mm, feed_lo_mm_min, doc_hi_mm, feed_hi_mm_min, cross_grain_factor,
                    chip_min_mm, chip_max_mm
               FROM feed_curve",
        )?;
        let curves = stmt
            .query_map([], |r| {
                Ok(FeedCurve {
                    id: r.get(0)?,
                    vendor_id: r.get(1)?,
                    series: r.get(2)?,
                    tool_type: r
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| ToolType::from_db_str(&s)),
                    dia_min_mm: r.get(4)?,
                    dia_max_mm: r.get(5)?,
                    rpm: r.get(6)?,
                    doc_lo_mm: r.get(7)?,
                    feed_lo_mm_min: r.get(8)?,
                    doc_hi_mm: r.get(9)?,
                    feed_hi_mm_min: r.get(10)?,
                    cross_grain_factor: r.get(11)?,
                    chip_min_mm: r.get(12)?,
                    chip_max_mm: r.get(13)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(curves)
    }

    pub fn chipload_rules(&self) -> Result<Vec<ChiploadRule>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, material_class_id, tool_type, vendor_id, dia_min_mm, dia_max_mm,
                    fz_min_mm, fz_typ_mm, fz_max_mm, rpm_min, rpm_max, max_stepdown_x_dia
               FROM chipload_rule",
        )?;
        let rules = stmt
            .query_map([], |r| {
                Ok(ChiploadRule {
                    id: r.get(0)?,
                    material_class_id: r.get(1)?,
                    tool_type: r
                        .get::<_, Option<String>>(2)?
                        .and_then(|s| ToolType::from_db_str(&s)),
                    vendor_id: r.get(3)?,
                    dia_min_mm: r.get(4)?,
                    dia_max_mm: r.get(5)?,
                    fz_min_mm: r.get(6)?,
                    fz_typ_mm: r.get(7)?,
                    fz_max_mm: r.get(8)?,
                    rpm_min: r.get(9)?,
                    rpm_max: r.get(10)?,
                    max_stepdown_x_dia: r.get(11)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rules)
    }

    /// The library keeps its own row per machine so presets can be scoped to
    /// one, separate from `ocs_cam_core::MachineEnvelope` (the project's own
    /// machine setup).
    pub fn machines(&self) -> Result<Vec<MachineRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, spindle_min_rpm, spindle_max_rpm, max_feed_xy_mm_min,
                    max_feed_z_mm_min, rigidity_factor
               FROM machine WHERE deleted_at IS NULL ORDER BY name",
        )?;
        let machines = stmt
            .query_map([], |r| {
                Ok(MachineRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    minimum_spindle_rpm: r.get::<_, Option<i64>>(2)?.unwrap_or(6_000),
                    maximum_spindle_rpm: r.get::<_, Option<i64>>(3)?.unwrap_or(24_000),
                    maximum_feed: r.get::<_, Option<f64>>(4)?.unwrap_or(5_000.0),
                    maximum_plunge_feed: r.get(5)?,
                    rigidity_factor: r.get::<_, Option<f64>>(6)?.unwrap_or(1.0),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(machines)
    }

    pub fn upsert_machine(&mut self, m: &MachineRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO machine (id, name, spindle_min_rpm, spindle_max_rpm,
                                 max_feed_xy_mm_min, max_feed_z_mm_min, rigidity_factor)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,
                spindle_min_rpm=excluded.spindle_min_rpm, spindle_max_rpm=excluded.spindle_max_rpm,
                max_feed_xy_mm_min=excluded.max_feed_xy_mm_min,
                max_feed_z_mm_min=excluded.max_feed_z_mm_min,
                rigidity_factor=excluded.rigidity_factor",
            params![
                m.id,
                m.name,
                m.minimum_spindle_rpm,
                m.maximum_spindle_rpm,
                m.maximum_feed,
                m.maximum_plunge_feed,
                m.rigidity_factor,
            ],
        )?;
        Ok(())
    }

    // MARK: - Resolution

    /// The whole point of the library: what should this tool cut this
    /// material at? `depth_of_cut_mm` lets a vendor feed curve pick the
    /// right feed for the DOC the operation will run; `None` evaluates the
    /// curve at its fastest (shallow) end.
    pub fn resolve(
        &self,
        tool: &LibraryTool,
        material_class_id: &str,
        machine: Option<&MachineRow>,
        depth_of_cut_mm: Option<f64>,
    ) -> Result<Option<ResolvedCuttingData>> {
        let Some(material_class) = self
            .material_classes()?
            .into_iter()
            .find(|c| c.id == material_class_id)
        else {
            return Ok(None);
        };
        Ok(resolver::resolve(
            tool,
            &material_class,
            machine,
            &self.presets(Some(&tool.id))?,
            &self.chipload_rules()?,
            &self.feed_curves()?,
            depth_of_cut_mm,
        ))
    }

    // MARK: - Import support

    pub fn upsert_data_source(
        &mut self,
        vendor_id: Option<&str>,
        kind: &str,
        title: &str,
        file_hash: &str,
        reliability: i64,
    ) -> Result<String> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM data_source WHERE file_hash=?1 AND kind=?2",
                params![file_hash, kind],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return Ok(existing);
        }
        let id = new_id();
        self.conn.execute(
            "INSERT INTO data_source (id, vendor_id, kind, title, file_hash, retrieved_at, reliability)
             VALUES (?1,?2,?3,?4,?5,datetime('now'),?6)",
            params![id, vendor_id, kind, title, file_hash, reliability],
        )?;
        Ok(id)
    }

    pub fn create_batch(&mut self, id: &str, source_id: &str, row_count: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO import_batch (id, source_id, status, row_count) VALUES (?1,?2,'staged',?3)",
            params![id, source_id, row_count],
        )?;
        Ok(())
    }

    pub fn finish_batch(&mut self, id: &str, status: &str, committed: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE import_batch SET status=?1, committed_count=?2, finished_at=datetime('now') WHERE id=?3",
            params![status, committed, id],
        )?;
        Ok(())
    }

    pub fn stage(&mut self, batch_id: &str, row_index: i64, raw_json: &str, status: &str, tool_id: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO staging_record (id, batch_id, row_index, raw_json, status, tool_id)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![new_id(), batch_id, row_index, raw_json, status, tool_id],
        )?;
        Ok(())
    }

    pub fn record_issue(&mut self, issue: &ValidationIssue, batch_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO validation_issue (id, batch_id, entity_type, entity_id, severity, code,
                                          field, raw_value, message)
             VALUES (?1,?2,'staging_record',?3,?4,?5,?6,?7,?8)",
            params![
                new_id(),
                batch_id,
                format!("{batch_id}#{}", issue.row_index),
                issue.severity.as_db_str(),
                issue.code,
                issue.field,
                issue.raw_value,
                issue.message,
            ],
        )?;
        Ok(())
    }

    pub fn record_provenance(&mut self, entity: &str, id: &str, field: &str, origin: &str, source_id: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO field_provenance (entity_type, entity_id, field, origin, source_id, updated_at)
             VALUES (?1,?2,?3,?4,?5,datetime('now'))
             ON CONFLICT(entity_type, entity_id, field) DO UPDATE SET
                origin=excluded.origin, source_id=excluded.source_id, updated_at=excluded.updated_at",
            params![entity, id, field, origin, source_id],
        )?;
        Ok(())
    }

    /// Fields the user hand-edited. A catalogue re-import must never clobber
    /// these.
    pub fn user_edited_fields(&self, entity: &str, id: &str) -> Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT field FROM field_provenance WHERE entity_type=?1 AND entity_id=?2 AND origin='user'",
        )?;
        let fields = stmt
            .query_map(params![entity, id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;
        Ok(fields)
    }

    /// Undo an entire import. Tools created by the batch are removed; tools
    /// it merely updated are left alone — their prior values are in
    /// `change_log`.
    pub fn rollback(&mut self, batch_id: &str) -> Result<()> {
        let batch_id = batch_id.to_string();
        self.transaction(move |repo| {
            repo.conn.execute("DELETE FROM tool WHERE batch_id = ?1", params![batch_id])?;
            repo.finish_batch(&batch_id, "rolled_back", 0)?;
            Ok(())
        })
    }

    pub fn add_to_group(&mut self, tool_id: &str, group_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tool_group_member (group_id, tool_id) VALUES (?1,?2)",
            params![group_id, tool_id],
        )?;
        Ok(())
    }
}

/// Fixed-width so filename sort order matches chronological order regardless
/// of digit count (seconds since epoch is 10 digits until the year 2286).
fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:010}-{:09}", now.as_secs(), now.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_library::model::ToolType;

    fn temp_repository(seed_starter_tools: bool) -> (ToolLibraryRepository, PathBuf) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mac2cam-tool-library-test-{}-{n}-{:?}.sqlite",
            new_id(),
            std::thread::current().id()
        ));
        let repo = ToolLibraryRepository::open(&path, seed_starter_tools).expect("open repository");
        (repo, path)
    }

    #[test]
    fn insert_and_read_round_trips_geometry() {
        let (mut repo, path) = temp_repository(false);
        let mut tool = LibraryTool::new("Test End Mill", ToolType::EndMill);
        tool.diameter_mm = Some(6.35);
        tool.flute_count = Some(3);
        repo.insert(&tool, None, None).expect("insert");

        let tools = repo.tools(false).expect("read tools");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "Test End Mill");
        assert_eq!(tools[0].diameter_mm, Some(6.35));
        assert_eq!(tools[0].flute_count, Some(3));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn soft_delete_moves_to_trash_and_restore_brings_it_back() {
        let (mut repo, path) = temp_repository(false);
        let tool = LibraryTool::new("Deletable", ToolType::EndMill);
        repo.insert(&tool, None, None).expect("insert");

        repo.delete(&tool.id).expect("delete");
        assert_eq!(repo.tools(false).expect("active").len(), 0);
        assert_eq!(repo.trashed_tools().expect("trash").len(), 1);

        repo.restore(&tool.id).expect("restore");
        assert_eq!(repo.tools(false).expect("active").len(), 1);
        assert_eq!(repo.trashed_tools().expect("trash").len(), 0);

        repo.delete(&tool.id).expect("delete again");
        repo.purge(&tool.id).expect("purge");
        assert_eq!(repo.tools(true).expect("all").len(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn duplicate_copies_presets_onto_the_new_tool() {
        let (mut repo, path) = temp_repository(false);
        let tool = LibraryTool::new("Original", ToolType::EndMill);
        repo.insert(&tool, None, None).expect("insert");
        let preset = super::super::model::CuttingPreset {
            id: new_id(),
            tool_id: tool.id.clone(),
            material_id: None,
            material_class_id: Some("hardwood".into()),
            machine_id: None,
            name: "Test preset".into(),
            origin: super::super::model::PresetOrigin::User,
            derivation: super::super::model::PresetDerivation::Explicit,
            confidence: 5,
            chipload_mm: Some(0.1),
            vc_m_per_min: None,
            spindle_rpm: Some(18_000),
            feed_xy_mm_min: Some(3000.0),
            feed_z_mm_min: Some(900.0),
            ramp_feed_mm_min: None,
            stepdown_mm: None,
            stepover_mm: None,
            clearance_stepover_mm: None,
            cut_direction: None,
            air_blast: false,
            notes: None,
        };
        repo.upsert(&preset, None, None).expect("upsert preset");

        let copy = repo.duplicate(&tool.id).expect("duplicate").expect("some");
        assert_ne!(copy.id, tool.id);
        assert_eq!(copy.name, "Original copy");
        let copied_presets = repo.presets(Some(&copy.id)).expect("presets");
        assert_eq!(copied_presets.len(), 1);
        assert_eq!(copied_presets[0].tool_id, copy.id);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn starter_tools_seed_exactly_once() {
        let (mut repo, path) = temp_repository(true);
        let first_count = repo.tools(false).expect("tools").len();
        assert!(first_count > 0);

        repo.delete(&repo.tools(false).unwrap()[0].id.clone()).expect("delete one");
        repo.seed_starter_tools_if_needed().expect("seed again");
        // The flag prevents re-seeding, so the deleted tool stays deleted
        // rather than being resurrected.
        assert_eq!(repo.tools(false).expect("tools").len(), first_count - 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn resolve_falls_through_to_generic_chipload_band() {
        let (mut repo, path) = temp_repository(false);
        let mut tool = LibraryTool::new("6mm Upcut", ToolType::EndMill);
        tool.diameter_mm = Some(6.0);
        tool.flute_count = Some(2);
        repo.insert(&tool, None, None).expect("insert");

        let resolved = repo
            .resolve(&tool, "hardwood", None, None)
            .expect("resolve ok")
            .expect("resolve some");
        assert!(resolved.is_estimate(), "generic chipload band must be flagged as an estimate");
        assert!(resolved.spindle_rpm > 0);
        assert!(resolved.feed_xy_mm_min > 0.0);

        let _ = std::fs::remove_file(path);
    }
}
