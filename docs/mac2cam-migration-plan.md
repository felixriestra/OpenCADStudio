# Mac2CAM full migration plan

## Product boundary

Mac2CAM is a 2.5D CAD/CAM application. The DWG/DXF document is the only
editable CAD model. Manufacturing algorithms consume immutable, normalized
geometry snapshots; they never edit or depend directly on DWG entities.

The application may read and display solid entities from existing drawings for
interoperability, but it does not author solids or expose extrusion, revolve,
loft, sweep, boolean-solid, shell, or slice workflows.

## Target architecture

```text
DWG/DXF entities ─┐
SVG import ───────┼─> native CAD entities ─> manufacturing snapshot
bitmap tracing ───┘                              │
                                                ▼
                                      ordered CAM operations
                                                │
                                                ▼
                                     canonical toolpath program
                                      │         │          │
                                      ▼         ▼          ▼
                                  preview   verification   postprocessor
                                                │
                                                ▼
                                         stock simulation
```

Generated toolpaths, previews, and simulation data are disposable caches.
Drawing geometry, manufacturing snapshots, tools, setups, and operation
parameters are authoritative project state.

## Phase 1 — Stable manufacturing geometry

- Add a versioned, UI-independent geometry snapshot to `ocs_cam_core`.
- Represent closed regions with islands, open engraving paths, and drill points.
- Track source entity identifiers and deterministic geometry fingerprints.
- Adapt DWG lines, arcs, circles, ellipses, splines, and polylines.
- Invalidate only operations whose source geometry changed.
- Preserve backward compatibility with existing CAM sidecars.

Exit criteria: geometry snapshots serialize deterministically, validate without
DWG dependencies, and round-trip through CAM operation persistence.

## Phase 2 — Mac2CAM project persistence

- Introduce the `.mac2cam` package format:
  - `drawing.dwg`
  - `cam.json`
  - `tools.json`
  - disposable `toolpaths/` and `simulation/` caches
- Add atomic save, recovery, migration, and missing-source diagnostics.
- Migrate OCS2Cam settings and recent-file records to Mac2CAM identifiers.

Exit criteria: a project reopens with drawing, setup, tools, operations, and
stale-state information intact.

## Phase 3 — SVG and bitmap import

- Port SVG validation, repair, units/viewBox handling, and transform flattening.
- Convert SVG primitives and paths into native CAD entities in one undoable
  transaction.
- Add bitmap modes: attach reference, trace to editable vectors, and later
  grayscale relief source.
- Use the Rust VTracer implementation directly.
- Port contour cleanup, simplification, curve fitting, hole detection, and
  import preview.

Exit criteria: imported SVG/traced geometry is editable, saveable to DWG/DXF,
and immediately usable by CAM.

## Phase 4 — Tool, material, and setup domain

- Port persistent tool libraries, cutter profiles, validation, CSV import, and
  presets.
- Add stock definition, work coordinate system, origin, fixtures, clearance,
  machine limits, and material presets.
- Replace command-line defaults with editable operation/setup panels.

Exit criteria: every operation references a validated tool and setup.

## Phase 5 — Complete 2.5D operations

- Port profile tabs, lead-in/out, ramps, finish passes, and direction control.
- Add pocket islands, rest machining, adaptive clearing, and entry strategies.
- Complete facing, drilling cycles, boring, slotting, and engraving.
- Add operation ordering, duplication, suppression, regeneration, and
  per-operation visualization.

Exit criteria: original 2DCam 2.5D workflows have Rust equivalents with golden
toolpath tests.

## Phase 6 — Verification and postprocessing

- Port machine-envelope, rapid-clearance, depth, feed, spindle, and gouge checks.
- Preserve canonical toolpaths independently of controller syntax.
- Add configurable posts, beginning with GRBL and then the original 2DCam set.
- Parse generated G-code and compare round-tripped motion with the canonical
  program.

Exit criteria: export is blocked on safety errors and every supported post has
round-trip fixtures.

## Phase 7 — Preview and stock simulation

- Draw source geometry, rapid moves, cutting moves, direction, tabs, and tool.
- Add playback controls and operation-range isolation.
- Port height-field stock removal and cache invalidation.
- Keep 3D strictly as manufacturing visualization, not editable solid geometry.

Exit criteria: users can visually verify the complete ordered job and simulated
remaining stock before export.

## Phase 8 — Relief and migration completion

- Port grayscale relief and optional STL-derived height-field workflows.
- Run parity fixtures against representative original 2DCam projects.
- Remove superseded compatibility adapters and obsolete OCS branding.
- Produce signed/notarized Mac2CAM bundles and migration documentation.

Exit criteria: agreed 2DCam workflows have parity evidence and Mac2CAM is the
sole maintained application.

## Engineering rules

- CAM and import geometry live in headless Rust crates with no UI dependency.
- The UI may orchestrate operations but may not implement machining algorithms.
- Every migration slice includes deterministic tests and backward-compatible
  persistence where practical.
- Native CAD geometry is authoritative; manufacturing snapshots are immutable
  and regenerable.
- No G-code is produced directly from SVG or bitmap input.

## Execution status

- 2026-09-11: Phase 1 complete.
  - Added the first versioned manufacturing-geometry schema.
  - Added closed regions with islands, open engraving paths, drill locations,
    source references, validation, deterministic fingerprints, and serde tests.
  - Added CAD adapters for lines, arcs, circles, ellipses, splines, lightweight
    polylines, and legacy 2D polylines.
  - Replaced whole-document invalidation with source-specific snapshot checks.
  - Preserved loading of legacy operation records that have no snapshot fields.
- 2026-09-11: Phase 2 persistence foundation complete.
  - Added the versioned `.mac2cam` ZIP package with `drawing.dwg`, `cam.json`,
    `tools.json`, and reserved toolpath/simulation cache directories.
  - Added validation, entry size limits, deterministic authoritative contents,
    atomic replacement, and round-trip tests.
  - Wired native Open, Save, Save As, recent-file handling, and CAM restoration
    to the package format; new drawings now default to Mac2CAM projects while
    DWG and DXF remain explicit interchange formats.
- 2026-09-11: Phase 3 import foundation complete.
  - Added a headless Rust SVG importer with input limits, XML entity rejection,
    physical-unit conversion, nested transform flattening, primitive/path
    normalization, curve sampling, multi-subpath handling, and tests.
  - Added `IMPORTSVG` to the CAM ribbon; imports are native editable CAD
    polylines committed as one undoable drawing edit.
  - Exposed native bitmap attachment in the CAM ribbon and added normalized
    grayscale height-field decoding for the relief workflow.
- 2026-09-11: Phase 4 setup foundation complete.
  - Added persistent stock, work origin, material, clearance, machine travel,
    maximum feed/spindle, and per-operation setup references.
  - Tool definitions are deduplicated into `tools.json` in project packages.
- 2026-09-11: Phase 5 baseline is operational.
  - Outside/inside profile, pocket, facing, boring, slotting, engraving, and
    drilling all generate canonical Rust toolpaths and ordered job operations.
  - Added operation listing, ordering, duplication, suppression, re-enabling,
    and deletion commands; the CAM ribbon exposes the ordered operation list.
- 2026-09-11: Phase 6 safety foundation complete.
  - Canonical motion validation and GRBL round-trip parsing are active.
  - Setup-aware validation blocks jobs exceeding feed, spindle, XYZ travel, or
    malformed motion constraints before export.
- 2026-09-11: Phase 7 simulation foundation complete.
  - Existing canonical preview segments now feed a headless stock height-field
    removal simulator with deterministic tests.
- 2026-09-11: Phase 8 relief foundation complete.
  - Bitmap grayscale data is decoded into bounded, normalized relief fields;
    it remains manufacturing data and never becomes editable solid geometry.
