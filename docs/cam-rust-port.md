# CAM Rust port

This branch makes Mac2CAM the CAD authoring surface and adds CAM as a
built-in workflow. It does not copy 2DCam's SwiftUI editor: Mac2CAM's DWG
entities remain the only editable geometry.

## Data flow

```text
DWG/DXF entity selection
        ↓ host adapter (acadrust → Contour / Point2)
ocs_cam_core (headless, serializable, no UI dependency)
        ↓ canonical Program / Motion stream
safety verification
        ↓
GRBL postprocessor → .nc / .gcode / .tap
```

The normalized contour is an in-memory boundary between CAD and CAM, not a
second user document. DXF bulges are retained so circular motion is posted as
G2/G3 instead of being unnecessarily flattened.

## Implemented workflow

- A dedicated `CAM` ribbon tab with Setup, 2D Toolpaths, and Output groups.
- Outside profile of a selected closed LWPOLYLINE or CIRCLE.
- Inside profile of a selected closed LWPOLYLINE or CIRCLE.
- Concentric pocket clearing of a selected closed LWPOLYLINE or CIRCLE.
- Facing over the bounds of a selected closed LWPOLYLINE or CIRCLE.
- Circular-interpolation boring from a selected CIRCLE.
- Multi-row slot clearing from a selected LINE.
- Engraving of a selected LINE, open/closed LWPOLYLINE, or CIRCLE.
- Peck drilling at selected POINT and CIRCLE centers.
- Millimeter and inch output based on drawing insertion units.
- Multiple depth passes, tool-radius compensation, safe-Z moves, feed/plunge
  rates, spindle control, preserved arcs, program verification, and GRBL
  output.
- Ordered operations, source entity handles, parameters, and tool definitions
  live in a versioned `CamJob`. Jobs compile all enabled operations into one
  verified program.
- Generated output is document-scoped and tied to the drawing revision. A
  drawing edit invalidates export until the path is regenerated. Export writes
  both controller G-code and a same-name `.cam.json` job sidecar.
- The GRBL parser accepts the postprocessor's supported motion subset and
  rejects unknown or incomplete commands for round-trip validation.
- Canonical rapid, line, and arc motions expand into typed preview segments;
  full-circle bores are tessellated correctly for canvas playback.

The command form is useful for repeatable testing and automation:

```text
CAMPROFILE tool=6 depth=3 stepdown=1 safe=5 feed=500 plunge=200 rpm=12000
CAMINSIDE tool=6 depth=3 stepdown=1 safe=5 feed=500 plunge=200 rpm=12000
CAMPOCKET tool=6 stepover=3 depth=3 stepdown=1 safe=5 feed=500 plunge=200 rpm=12000
CAMFACE tool=10 stepover=7 depth=0.5 stepdown=0.5 safe=5 feed=600 plunge=200 rpm=12000
CAMBORE tool=6 depth=8 stepdown=2 safe=5 feed=300 plunge=120 rpm=9000
CAMSLOT tool=6 width=12 stepover=3 depth=5 stepdown=2 safe=5 feed=400 plunge=150 rpm=12000
CAMENGRAVE depth=0.5 stepdown=0.5 safe=5 feed=300 plunge=100 rpm=12000
CAMDRILL depth=8 stepdown=2 safe=5 plunge=150 rpm=9000
CAMINFO
CAMCLEAR
CAMPREVIEW
CAMEXPORT
```

Parameters use drawing units. `tool` and `diameter` are aliases; `stepdown`,
`doc` are aliases; `top` sets stock-top Z.

## Port boundary

The first production slice intentionally establishes a safe architecture and
the most common 2.5D operations. Remaining 2DCam capabilities should be ported
into `ocs_cam_core` in this order:

1. Persistent operation records and a tool/material library.
2. Canvas toolpath overlay plus operation editing panel.
3. Tabs/ramp/lead controls; extend pocketing with islands, entry ramps, and
   adaptive clearing, and add stock-boundary checks to face/bore/slot.
4. G-code parser, round-trip validation, and configurable machine posts.
5. Playback and stock-removal simulation.
6. Relief/STL and bitmap-derived machining, after the 2D pipeline is stable.

Each generator belongs in the headless crate and must produce the same
canonical `Program`. UI code may adapt entity selection and display results,
but must not contain machining geometry or G-code logic.

## Development

Run tests and compile the app:

```text
cargo test -p ocs_cam_core
cargo check --bin Mac2CAM
```

On macOS, `./script/build_and_run.sh --verify` builds a development app bundle,
launches it, and checks that its GUI process stays alive. The Codex project Run
action invokes the same script.
