# 2DCam → Mac2CAM CAM workflow audit

Audit date: 2026-09-12

## Executive result

Mac2CAM already has the correct architectural starting point: CAD selections are converted into persistent manufacturing snapshots, operations are stored in an ordered job, canonical motions can be verified and posted, a 2D toolpath can be stepped, and a height-field stock result can be rendered. The missing work is primarily workflow integration and parity, not a replacement of the Rust CAM core.

## Capability comparison

| Workflow | 2DCam | Mac2CAM now | Required migration |
|---|---|---|---|
| Operation list | Ordered, enabled/disabled, editable, regenerated, per-operation estimates | Ordered, enabled/disabled, duplicate, regenerate, editable parameters | Add estimates and stronger stale/error status |
| Operations | Facing, drilling, bore, slot, outside/inside profile, pocket, open pocket, chamfer, engraving, STL relief, bitmap relief | Facing, drilling, bore, slot, outside/inside profile, pocket, engraving | Add open pocket and chamfer first; treat relief as a later dedicated workflow |
| Tool assignment | Tool database, per-operation tool, optional finish tool | Small job-local tool library and per-operation tool | Add persistent tool database and finish-tool support |
| Import G-code | File picker for `.gcode`, `.nc`, `.tap`; parses motion into canonical toolpath | Core GRBL parser exists, but no import workflow | Add file import, diagnostics, units handling and canonical-program handoff |
| Paste G-code | Monospaced editor sheet, then load for simulation | Missing | Add paste/editor overlay sharing the import pipeline |
| 2D preview | Generated or imported program, progressive playback, active G-code line, timing and play/pause/restart/step | Canvas wires with first/previous/next/last stepping | Add playback state/timer and synchronized G-code viewer |
| Stock simulation | Incremental height-field removal with fast/fine grids and full inspection | Static height-field calculation for a selected operation | Simulate the combined enabled job incrementally and expose resolution controls |
| Separate 3D view | Floating resizable AppKit window showing machined stock | Stock mesh is rendered only in the main scene | Add a second native iced window backed by shared CAM preview state |
| Verification/export | Machine/tool/fixture/safety checks, posted-code round trip and multiple postprocessors | Basic program verification and checked GRBL export | Expand verifier, machine envelope, fixtures, posts and posted-code round trip |

## 2DCam reference flow

1. `ProjectSidebarView` imports a file or opens `GCodePasteSheet`.
2. `GCodeParser` converts supported G0/G1/G2/G3 motion into a canonical toolpath.
3. `SimulationPlaybackModel` owns generated and imported programs through the same interface, tracks the active command/G-code line, timing, playback and stock state.
4. `ToolpathStockSimulator` advances a height field incrementally rather than recalculating the final result for every UI step.
5. The main operations view shows toolpath and G-code progress together.
6. `FloatingPreviewWindowController` opens a separate resizable 3D stock window using the same simulation state.

## Recommended Rust implementation sequence

### Phase 1 — One canonical simulation input

- Introduce `CamPlaybackState` in the application layer.
- Let both `CamJob::compile_enabled()` and imported/pasted G-code produce the same `Program` value.
- Preserve original imported lines plus a motion-to-line map for synchronized inspection.

### Phase 2 — Import and paste

- Extend the parser beyond the postprocessor's minimal GRBL subset with explicit modal units, absolute/incremental coordinates, planes, arcs, feed and spindle state.
- Add `CAMIMPORT` and `CAMPASTE` ribbon actions.
- Never add imported G-code as an editable CAD operation; load it as an external simulation program with its own filename/source identity.

### Phase 3 — Playback and G-code inspector

- Add play, pause, restart, single-step, previous/next, speed and progress actions.
- Render only the visible prefix of preview segments.
- Add a scrollable monospaced program panel that follows the active source line.

### Phase 4 — Incremental stock and separate 3D window

- Refactor `simulate_stock` into an initialized state plus `advance_to(command_index)`.
- Offer fast and fine cell-size presets with a hard memory budget.
- Open a second iced window for the stock mesh; both windows observe the same playback state.

### Phase 5 — Operation and safety parity

- Add open pocket and chamfer generators and operation-specific controls.
- Compile and simulate every enabled operation in job order, including tool changes.
- Add machine travel, flute-length, feed/RPM, fixture/collision and posted-round-trip checks before export.
- Add per-operation and total machining-time estimates.

## Important boundary

Imported G-code should bypass CAD geometry and operation generation, but it should not bypass parsing, verification or simulation. Generated and imported programs should converge at the canonical `Program` layer so the 2D viewer, 3D stock simulation and safety reporting cannot disagree about which motions are being inspected.
