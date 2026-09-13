//! Save/load integration for a document's `CamJob` (the CAM operations,
//! setups, and tool library built up in the CAM ribbon) — embeds it directly
//! into the drawing itself via an XRecord, following the exact precedent
//! `crate::scene::sketch_persist`/`crate::scene::named_parameters_persist`
//! already established: the "lazy" model, where the job lives only in
//! `DocumentTab::cam_job` in-memory during editing and is materialized into
//! `document.objects` as one XRecord right before a save, then read back
//! right after a load.
//!
//! This is what makes a plain `Save` on an ordinary `.dwg` — not just an
//! explicit "Mac2CAM Project" (`.mac2cam`) save — keep the CAM operations:
//! before this, `CamJob` only round-tripped through the separate
//! `crate::io::mac2cam_project` sidecar format, so opening a `.dwg`, adding
//! operations, and hitting Save silently discarded them.
//!
//! Owner handle: `document.header.model_space_block_handle`, the same
//! already-proven anchor `named_parameters_persist` uses for its own
//! document-wide (not per-entity) data — a real `BlockRecord` guaranteed to
//! exist in every `CadDocument`, confirmed there to round-trip through both
//! DWG and DXF (a `Dictionary`-type owner like the root Named Objects
//! Dictionary does not, for the DXF path specifically — see that module's
//! doc comment for the full story).
//!
//! The free functions below take a plain `&mut CadDocument`/`&CamJob` rather
//! than `&mut DocumentTab` so the round-trip tests don't need to construct
//! one of `DocumentTab`'s ~30 unrelated fields — `impl DocumentTab` at the
//! bottom is a thin, untested-on-its-own wrapper.
//!
//! **One deliberate deviation from `named_parameters_persist`'s template**:
//! this module encodes the blob as JSON, not `bincode`. `CamOperation` has
//! several `#[serde(default, skip_serializing_if = "Option::is_none")]`
//! fields (`setup_id`, `geometry`, `geometry_fingerprint`) — correct for a
//! self-describing, field-tagged format like JSON, but fatal for `bincode`,
//! which serializes structs positionally with no field tags: skipping a
//! `None` field during serialize desyncs every field after it from what
//! Deserialize expects to read next, corrupting the whole struct (confirmed
//! by a round-trip test failing with a bincode `UnexpectedEof` the moment
//! any of those three fields was `None` — the common case). `CamJob`
//! already exposes `to_json_pretty`/`from_json` for exactly this reason;
//! this module calls `serde_json` directly instead, for the same result
//! without the pretty-printing overhead.

use acadrust::{CadDocument, Handle};
use ocs_cam_core::CamJob;

const XRECORD_KEY: &str = "MAC2CAM_CAM_JOB";

/// See `named_parameters_persist::MAX_CHUNK_BYTES`'s doc comment: the
/// vendored DWG writer truncates any single `XRecordValue::Chunk` entry to
/// `u8::MAX` bytes, so a job with more than a handful of operations (each
/// potentially carrying its own `ManufacturingGeometry`) must be split
/// across multiple same-code `310` entries on write and concatenated back
/// on read.
const MAX_CHUNK_BYTES: usize = u8::MAX as usize;

/// Prefixed onto the serialized blob so a future schema change can be
/// detected and gracefully skipped rather than silently misreading bytes.
/// Distinct from `CamJob::schema_version` (that one guards the job's own
/// internal shape via `CamJob::validate`; this one guards this module's
/// wire format).
const FORMAT_VERSION: u8 = 1;

fn encode(job: &CamJob) -> Option<Vec<u8>> {
    let mut bytes = vec![FORMAT_VERSION];
    bytes.extend(serde_json::to_vec(job).ok()?);
    Some(bytes)
}

fn decode(bytes: &[u8]) -> Option<CamJob> {
    let (&version, body) = bytes.split_first()?;
    if version != FORMAT_VERSION {
        return None;
    }
    serde_json::from_slice(body).ok()
}

/// Removes the XRecord from `owner`'s extension dictionary, if present.
/// The dictionary itself is left in place even if now empty — see
/// `sketch_persist`'s identical helper for why (harmless bookkeeping vs. a
/// stale XRecord that would resurrect deleted operations on the next load).
fn remove_xrecord(document: &mut CadDocument, owner: Handle, key: &str) {
    let Some(dictionary_handle) = document.extension_dictionary_handle(owner) else {
        return;
    };
    let Some(acadrust::objects::ObjectType::Dictionary(dictionary)) = document.objects.get_mut(&dictionary_handle) else {
        return;
    };
    let Some(index) = dictionary.entries.iter().position(|(name, _)| name.eq_ignore_ascii_case(key)) else {
        return;
    };
    let (_, record_handle) = dictionary.entries.remove(index);
    document.objects.remove(&record_handle);
}

/// Writes `cam_job` into `document`'s `MAC2CAM_CAM_JOB` XRecord, ready for
/// whatever save call happens next to serialize the document as-is.
///
/// A job with no operations writes no XRecord (and actively removes one
/// left over from an earlier, non-empty save) — a plain drawing with no
/// machining setup yet should not gain persisted-but-empty bookkeeping, and
/// clearing every operation should not resurrect them on reload.
fn materialize_cam_job(document: &mut CadDocument, cam_job: &CamJob) {
    let owner = document.header.model_space_block_handle;
    if owner.is_null() {
        return;
    }
    if cam_job.operations.is_empty() {
        remove_xrecord(document, owner, XRECORD_KEY);
        return;
    }
    let Some(bytes) = encode(cam_job) else { return };
    document.ensure_xrecord(owner, XRECORD_KEY);
    if let Some(record) = document.xrecord_mut(owner, XRECORD_KEY) {
        // Overwrite, not append: a resave must replace the prior blob, not
        // accumulate more Chunk entries every time.
        record.entries.clear();
        for chunk in bytes.chunks(MAX_CHUNK_BYTES) {
            record
                .entries
                .push(acadrust::objects::XRecordEntry::new(310, acadrust::objects::XRecordValue::Chunk(chunk.to_vec())));
        }
    }
}

/// Reads back `document`'s `MAC2CAM_CAM_JOB` XRecord, if any. Returns
/// `None` when there's nothing to load (no XRecord, or an undecodable/
/// future-format blob) so the caller can decide what "nothing found" means
/// — for `DocumentTab::load_cam_job_from_document` that means "leave
/// `cam_job` untouched", not "reset it".
fn load_cam_job(document: &CadDocument) -> Option<CamJob> {
    let owner = document.header.model_space_block_handle;
    if owner.is_null() {
        return None;
    }
    let record = document.xrecord(owner, XRECORD_KEY)?;
    // Concatenate every Chunk entry in order, not just the first — see
    // `MAX_CHUNK_BYTES`'s doc comment.
    let mut bytes = Vec::new();
    for entry in &record.entries {
        if let acadrust::objects::XRecordValue::Chunk(chunk) = &entry.value {
            bytes.extend_from_slice(chunk);
        }
    }
    if bytes.is_empty() {
        return None;
    }
    decode(&bytes)
}

impl super::document::DocumentTab {
    /// Called from the same save entry points as
    /// `materialize_sketch_constraints_for_save`/
    /// `materialize_named_parameters_for_save`
    /// (`Mac2CAM::prepare_native_save`, the wasm save path).
    pub(super) fn materialize_cam_job_for_save(&mut self) {
        materialize_cam_job(&mut self.scene.document, &self.cam_job);
    }

    /// Called right after a document open installs its `CadDocument`, the
    /// same call sites `load_sketch_constraints_from_document`/
    /// `load_named_parameters_from_document` already have
    /// (`Mac2CAM::on_file_opened`, the automation `"open"` op).
    ///
    /// Deliberately does *not* reset `cam_job` when no XRecord is found —
    /// unlike the constraint/parameter loaders, which reset to empty first:
    /// a `.mac2cam` project's own sidecar load (`mac2cam_project::
    /// read_project`) runs right after this at the same call sites and is
    /// the authoritative source for that format, so this must not clobber
    /// it back to a blank job when the embedded drawing itself has no
    /// XRecord (e.g. a project created before this feature existed).
    pub(super) fn load_cam_job_from_document(&mut self) {
        if let Some(job) = load_cam_job(&self.scene.document) {
            self.cam_job = job;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acadrust::CadDocument;
    use ocs_cam_core::{outside_profile, AdvancedParameters, CamOperation, Contour, ContourVertex, OperationKind, ProfileParameters, ToolDefinition, Units};

    /// Mirrors `ocs_cam_core::job::tests::operation` — the smallest
    /// `CamOperation` that survives `CamJob::add_operation`'s validation
    /// (a real, compiled `Program` via `outside_profile`, not a stub).
    fn sample_operation(id: &str) -> CamOperation {
        let parameters = ProfileParameters {
            depth: 1.0,
            step_down: 1.0,
            ..ProfileParameters::default()
        };
        let contour = Contour {
            closed: true,
            vertices: vec![
                ContourVertex::line(0.0, 0.0),
                ContourVertex::line(20.0, 0.0),
                ContourVertex::line(20.0, 10.0),
                ContourVertex::line(0.0, 10.0),
            ],
        };
        CamOperation {
            id: id.to_string(),
            name: "Profile".to_string(),
            kind: OperationKind::OutsideProfile,
            enabled: true,
            setup_id: None,
            source_ids: vec!["AB".to_string()],
            geometry: None,
            geometry_fingerprint: None,
            tool: ToolDefinition::from_parameters("tool-1", parameters),
            parameters,
            advanced: AdvancedParameters::default(),
            program: outside_profile(&contour, parameters).unwrap(),
        }
    }

    fn job_with_one_operation() -> CamJob {
        let mut job = CamJob::new("Test job", Units::Millimeters);
        job.add_operation(sample_operation("op-1")).unwrap();
        job
    }

    #[test]
    fn a_cam_job_survives_materialize_and_reload() {
        let mut document = CadDocument::new();
        let job = job_with_one_operation();
        materialize_cam_job(&mut document, &job);

        let restored = load_cam_job(&document).expect("materialized job should load back");
        assert_eq!(restored, job);
    }

    #[test]
    fn an_empty_job_is_not_materialized() {
        let mut document = CadDocument::new();
        materialize_cam_job(&mut document, &CamJob::new("Empty", Units::Millimeters));
        let owner = document.header.model_space_block_handle;
        assert!(document.xrecord(owner, XRECORD_KEY).is_none(), "a job with no operations should not create an XRecord");
    }

    #[test]
    fn clearing_all_operations_removes_the_stale_xrecord() {
        let mut document = CadDocument::new();
        let mut job = job_with_one_operation();
        materialize_cam_job(&mut document, &job);
        let owner = document.header.model_space_block_handle;
        assert!(document.xrecord(owner, XRECORD_KEY).is_some(), "sanity: the first save should have materialized an XRecord");

        job.operations.clear();
        materialize_cam_job(&mut document, &job);
        assert!(
            document.xrecord(owner, XRECORD_KEY).is_none(),
            "the stale XRecord from the earlier non-empty save must be removed, not left behind"
        );
    }

    #[test]
    fn loading_with_no_xrecord_returns_none() {
        let document = CadDocument::new();
        assert!(load_cam_job(&document).is_none());
    }

    #[test]
    fn decode_rejects_a_mismatched_format_version() {
        let mut bytes = vec![FORMAT_VERSION.wrapping_add(1)];
        bytes.extend(serde_json::to_vec(&CamJob::new("x", Units::Millimeters)).unwrap());
        assert!(decode(&bytes).is_none(), "a future/unknown format version must be rejected, not misread");
    }

    /// The above tests only prove `materialize`/`load` agree with each other
    /// on the same in-memory `CadDocument` — this drives a real
    /// `save_to_bytes`/`load_bytes` round trip (the same primitives the
    /// native save/open path uses), through both supported formats,
    /// mirroring `named_parameters_persist`'s own real-bytes tests.
    fn full_bytes_roundtrip(ext: &str) -> CamJob {
        let mut document = CadDocument::new();
        let job = job_with_one_operation();
        materialize_cam_job(&mut document, &job);
        let bytes = crate::io::save_to_bytes(&document, ext, document.version).unwrap_or_else(|e| panic!("save to {ext} bytes: {e}"));
        let reloaded_document =
            crate::io::load_bytes(&format!("cam_job_roundtrip.{ext}"), bytes).unwrap_or_else(|e| panic!("reload {ext} bytes: {e}"));
        load_cam_job(&reloaded_document).expect("materialized job should survive a real save/load round trip")
    }

    #[test]
    fn a_cam_job_survives_a_real_dxf_save_and_load_round_trip() {
        let restored = full_bytes_roundtrip("dxf");
        assert_eq!(restored.operations.len(), 1);
        assert_eq!(restored.name, "Test job");
    }

    #[test]
    fn a_cam_job_survives_a_real_dwg_save_and_load_round_trip() {
        let restored = full_bytes_roundtrip("dwg");
        assert_eq!(restored.operations.len(), 1);
        assert_eq!(restored.name, "Test job");
    }
}
