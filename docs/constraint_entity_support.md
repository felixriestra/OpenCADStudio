# Constraint-to-Entity Support Matrix

Which geometric/dimensional constraints can actually be applied to which
entity types, and what happens when you try one that isn't supported.

## The short version

OCS's constraint solver (`ocs_gcs`, wired in via `src/scene/sketch_solve.rs`)
natively understands **four** geometry types: **Line, Circle, Arc, and
Ellipse** — not "everything simplified to lines and circles," but not
"everything" either:

- **Line, Circle, Arc** have full support across the constraint set (an Arc
  behaves like a Circle for whole-entity constraints via its `.circle`
  field, plus has its own two endpoints).
- **Ellipse** support is partial: position/size constraints (Fixed,
  Coincident/Concentric on its center, Equal Distance, Distance/DistanceX/
  DistanceY between points) work, but tangency and point-on-curve do not
  (`PointOnEllipse` isn't wired up yet).
- **Everything else — Polyline, Spline, Text, MText, Block/Insert, Hatch,
  Dimension, etc. — is invisible to every constraint.** `register_entity`
  (`src/scene/sketch_solve.rs`) only recognizes Line/Circle/Arc/Ellipse;
  anything else resolves to `None`.

A constraint referencing an unsupported entity or ref shape is **not an
error** — `build_constraint` returns an empty constraint list for that
`SketchConstraint`, so it's silently skipped on every solve. The constraint
record still gets added (its Properties-panel row and viewport glyph still
show up), it just never actually constrains anything. There is currently no
warning when this happens — see the "Common trap" section below.

## Support matrix

| Constraint | Command | What it selects | Works on | Notes |
|---|---|---|---|---|
| Coincident | `GCCOINCIDENT` | 2 points | Line, Circle, Arc, Ellipse | Any two addressable points (endpoint or center) held equal |
| Center Point | `GCCOINCIDENT_CENTER` | 2 points | Line, Circle, Arc, Ellipse | Same mechanism as Coincident — UI/label distinction only |
| Midpoint | `GCCOINCIDENT_MID` | 1 point + 1 whole line | point: any — line: **Line only** | |
| Point on Curve | `GCCOINCIDENT_CURVE` | 1 point + 1 whole curve | point: any — curve: **Line, Circle, Arc** (not Ellipse) | Arc case ignores its sweep (treated as the full circle) |
| Concentric | `GCCONCENTRIC` | 2 points | Line, Circle, Arc, Ellipse | Same point-pair mechanism; normally used with center markers |
| Horizontal | `GCHORIZONTAL` | 1 whole line | **Line only** | |
| Vertical | `GCVERTICAL` | 1 whole line | **Line only** | |
| Parallel | `GCPARALLEL` | 2 whole lines | **Line + Line only** | |
| Perpendicular | `GCPERPENDICULAR` | 2 whole lines | **Line + Line only** | |
| Colinear | `GCCOLLINEAR` | 2 whole lines | **Line + Line only** | |
| Equal | `GCEQUAL` | 2 whole entities | Line+Line (equal length), or (Circle\|Arc)+(Circle\|Arc) (equal radius) | Mixed Line+Circle is a no-op |
| Equal Distance | `GCEQUAL_DIST` | 4 points | Line, Circle, Arc, Ellipse | Distance between one point pair equals another's |
| Symmetric | `GCSYMMETRIC` | 2 points + 1 mirror line | points: any — mirror: **Line only** | |
| Fixed | `GCFIX` | 1 whole entity | **Line, Circle, Arc, Ellipse — all four** | The only constraint with full Ellipse support |
| Tangent | `GCTANGENT` | 2 whole entities | (Circle\|Arc)+(Circle\|Arc), or (Circle\|Arc)+Line | Line+Line and anything+Ellipse are no-ops |
| Normal | `NRCONSTRAINT` | 2 whole entities | **(Circle\|Arc)+Line only** | Line+Line, Circle+Circle, Ellipse all no-ops. No AutoCAD equivalent command |
| Distance | `DIMCONSTRAINT` | 2 points + value | Line, Circle, Arc, Ellipse | |
| Angle | `DCANGULAR` | 2 whole lines + value | **Line + Line only** | |
| Radius | `DIMCONSTRAINT` | 1 whole circle/arc + value | **Circle, Arc only** | |
| Diameter | `DIMCONSTRAINT` | 1 whole circle/arc + value | **Circle, Arc only** | |
| DistanceX / DistanceY | `DIMCONSTRAINT` | 2 points + value | Line, Circle, Arc, Ellipse | Signed X- or Y-only component |
| Arc Length | `DIMCONSTRAINT` | 1 whole arc + value | **Arc only** — not even Circle | |

Command names are current as of the 2026-09-11 AutoCAD-naming rename — see
[ocs_vs_autocad_commands.md](ocs_vs_autocad_commands.md).

## Addressable points

"Point" refs above resolve through `EntityGeom::point_for_marker`
(`src/scene/sketch_solve.rs`):

| Entity | Marker `0` | Marker `1` | Marker `-3` |
|---|---|---|---|
| Line | start | end | — |
| Circle | — | — | center |
| Arc | start | end | center |
| Ellipse | — | — | center |

## Common trap: exploded geometry, and reference geometry that can move

Two real gotchas found while debugging a "circle won't go tangent to a
rectangle" report:

1. **`RECTANG` draws a single Polyline**, not four Lines. Since Polylines
   aren't registered at all, any constraint against one (Tangent included)
   silently does nothing. `EXPLODE` it into four Lines first, then
   constrain the specific edge you need.
2. **An unconstrained reference entity can move too.** If you make a circle
   tangent to a line that has nothing else pinning it in place, the solver
   is free to move *either side* to reach zero error — it may relocate the
   line to meet the circle instead of (or as well as) moving the circle.
   This is correct, expected constraint-solver behavior (the same way
   FreeCAD/SolidWorks sketches work), not a bug. Add a `GCFIX` constraint
   to the reference edge first if you want only the circle to move.

## Methodology

Read directly from `build_constraint` in `src/scene/sketch_solve.rs`
(2026-09-11) — every `ConstraintKind` match arm, its ref-shape destructuring,
and which `EntityGeom` variants each arm's `whole_line`/`whole_circle`/
`point_ref`/inline match accepts. Cross-checked against `register_entity`
(same file) for which acadrust entity types make it into the solver's
parameter store at all, and `resolve_point`
(`src/scene/sketch_constraints.rs`) for the point-marker convention. Verified
end-to-end against a running debug build for the two "common trap" cases
above (Polyline no-op, reference-geometry drift), not just read from source.
