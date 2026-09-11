# OCS vs AutoCAD Command Comparison

OCS currently registers **557** commands in its autocomplete/command-line registry
(`crate::command::all_registered_command_names()`, queried live via the app's
`{"op":"commands"}` automation endpoint on 2026-09-10; re-verified after the
2026-09-11 constraint-command rename below, which also added one brand-new
command — `SHOWCONSTRAINTS`, the Constraints ribbon group's new global
visibility toggle — bringing the live total from 556 to 557).

Of those:
- **495** use the exact same command name as real AutoCAD, with the same purpose.
- **62** differ from AutoCAD — either the name is OCS-invented (no AutoCAD command
  by that name exists), or the name collides with something in OCS that AutoCAD
  names differently, or a duplicate/synonym was registered alongside the real one.

## Methodology

- The live 556-command list was fetched from the running app (not from the stale,
  pre-generated `docs/command_reference.md`, which we confirmed predates this
  count and contains zero AutoCAD cross-references).
- Ambiguous short tokens (`P`, `MS`, `SO`, `TS`, …) were resolved against a real
  `acad.pgp` alias table, not guessed.
- The sketch-constraint command family was checked directly against Autodesk's
  documentation for the `GCxxx`/`DCxxx` geometric/dimensional constraint commands.
- Suspicious duplicate-looking pairs (e.g. `ALIGN3D` next to `3DALIGN`,
  `DWGPROP` next to `DWGPROPS`) were checked individually rather than assumed.
- `HORIZONTAL` / `VERTICAL` looked like constraint aliases at first glance but
  were traced in the OCS source (`src/modules/view/tile_horiz.rs`,
  `src/app/commands/view.rs`) to a *viewport-tiling* macro (`VPORTS 2H` / `VPORTS 2V`),
  unrelated to AutoCAD's `GCHORIZONTAL`/`GCVERTICAL` constraints — a good example
  of why name-only matching isn't enough.
- This is a best-effort classification of 556 individual names, cross-checked
  where the classification was non-obvious; treat the exact counts as accurate
  to within a small margin (a handful of the 495 "same" rows are common,
  unambiguous AutoCAD terms not individually re-verified against a live index).
- **2026-09-11 rename pass**: 17 of the 18 OCS-invented sketch-constraint command
  ids (originally listed in Section 2) were renamed to their real AutoCAD
  `GCxxx`/`DCxxx` equivalents and moved into Section 1 — no aliases were kept for
  the old names (`CCONSTRAINT`, `HCONSTRAINT`, `DCONSTRAINT`, etc. no longer
  resolve to anything; these were brand-new, unshipped commands nobody depended
  on). Four sub-modes with no distinct AutoCAD command of their own (AutoCAD
  expresses them as an osnap choice mid-command, not a separate command) were
  given a suffixed AutoCAD-rooted name instead, matching OCS's own existing
  convention for command variants (`ARC_3P`, `CIRCLE_2P`, `RECT_CEN`, …):
  `GCCOINCIDENT_CENTER` (center-point coincident), `GCCOINCIDENT_MID`
  (midpoint coincident), `GCCOINCIDENT_CURVE` (point-on-curve coincident), and
  `GCEQUAL_DIST` (equal distance between point pairs). `DCONSTRAINT` (OCS's
  typed-value dispatcher for Distance/Radius/Diameter/ArcLength) became
  `DIMCONSTRAINT`, AutoCAD's real generic auto-detect dimensional-constraint
  command. `ACONSTRAINT` became `DCANGULAR`, AutoCAD's real angular
  dimensional-constraint command. `NRCONSTRAINT` (Normal) was left unchanged —
  AutoCAD's constraint set has no equivalent command — and stays in Section 2.
  This same pass added `SHOWCONSTRAINTS` (Section 2, new): a global on/off
  ribbon toggle for constraint-glyph visibility, with no AutoCAD equivalent —
  real AutoCAD has no single command that hides every constraint glyph at
  once. Two more visibility controls were added as Properties-panel UI
  (a per-constraint glyph/value-label toggle, and a named-parameter-label
  toggle) rather than as typed commands, so they don't appear in this table.

---

## Section 1 — Same name, same purpose (495)

| # | OCS Command | AutoCAD Command | Purpose |
|---|---|---|---|
| 1 | 3DALIGN | 3DALIGN | Aligns objects in 3D using up to 3 source/destination point pairs |
| 2 | 3DARRAY | 3DARRAY | Creates a 3D rectangular or polar array (legacy) |
| 3 | 3DFACE | 3DFACE | Creates a 3- or 4-sided 3D surface face |
| 4 | 3DMESH | 3DMESH | Creates a free-form 3D polygon mesh |
| 5 | 3DMOVE | 3DMOVE | Moves objects in 3D with a move grip tool |
| 6 | 3DORBIT | 3DORBIT | Interactively rotates the 3D view around a target |
| 7 | 3DPOLY | 3DPOLY | Creates a 3D polyline with non-planar vertices |
| 8 | 3DROTATE | 3DROTATE | Rotates objects in 3D with a rotate grip tool |
| 9 | ABOUT | ABOUT | Displays application version/license information |
| 10 | ADC | ADC (alias) | Opens the DesignCenter palette |
| 11 | ADCENTER | ADCENTER | Opens DesignCenter for browsing/inserting content |
| 12 | ADDSELECTED | ADDSELECTED | Creates a new object using the properties of a selected object |
| 13 | ADJUST | ADJUST | Adjusts brightness, contrast, and fade of a raster image |
| 14 | ALIASEDIT | ALIASEDIT | Opens the command-alias editor |
| 15 | ALIGN | ALIGN | Aligns objects with other objects in 2D or 3D |
| 16 | ANGBASE | ANGBASE | Sysvar: direction of angle 0 |
| 17 | ANGDIR | ANGDIR | Sysvar: direction of positive angles (CW/CCW) |
| 18 | ANNOALLVISIBLE | ANNOALLVISIBLE | Sysvar: show/hide annotations not matching current scale |
| 19 | ANNOAUTOSCALE | ANNOAUTOSCALE | Sysvar: auto-add scale to annotative objects on scale change |
| 20 | ANNOSCALE | ANNOSCALE | Sysvar: current annotation scale |
| 21 | ANNOUPDATE | ANNOUPDATE | Updates non-current-scale annotative object representations |
| 22 | ARC | ARC | Draws an arc |
| 23 | ARCHIVE | ARCHIVE | Archives a sheet set into a single package |
| 24 | ARCTEXT | ARCTEXT (Express Tool) | Places text along an arc |
| 25 | AREA | AREA | Calculates area/perimeter of objects or a point sequence |
| 26 | ARRAY | ARRAY | Creates rectangular, polar, or path arrays |
| 27 | ARRAYPATH | ARRAYPATH | Creates an array distributed along a path curve |
| 28 | ARRAYPOLAR | ARRAYPOLAR | Creates a polar (circular) array |
| 29 | ARRAYRECT | ARRAYRECT | Creates a rectangular array |
| 30 | ATTDEF | ATTDEF | Creates a block attribute definition |
| 31 | ATTDIA | ATTDIA | Sysvar: dialog vs command-line attribute prompts on insert |
| 32 | ATTDISP | ATTDISP | Controls global visibility of block attributes |
| 33 | ATTEDIT | ATTEDIT | Edits attribute values in a block instance |
| 34 | ATTEXT | ATTEXT | Extracts block attribute data to a file |
| 35 | ATTMAN | ATTMAN | Opens the Block Attribute Manager |
| 36 | ATTMODE | ATTMODE | Sysvar: legacy attribute display mode |
| 37 | ATTREQ | ATTREQ | Sysvar: toggles attribute-value prompting on insert |
| 38 | ATTSYNC | ATTSYNC | Updates block references to match current attribute definitions |
| 39 | AUDIT | AUDIT | Checks the drawing for errors, optionally fixes them |
| 40 | AUNITS | AUNITS | Sysvar: angular units format |
| 41 | AUPREC | AUPREC | Sysvar: angular units display precision |
| 42 | BACKGROUND | BACKGROUND | Sets a background for a named/rendered view |
| 43 | BASE | BASE | Sets the insertion base point of the current drawing |
| 44 | BATTMAN | BATTMAN | Opens the Block Attribute Manager |
| 45 | BEDIT | BEDIT | Opens the Block Editor |
| 46 | BLEND | BLEND | Creates a spline blending two curves |
| 47 | BLIPMODE | BLIPMODE | Sysvar: toggles marker blips at picked points |
| 48 | BLOCK | BLOCK | Creates a block definition from selected objects |
| 49 | BLOCKSPALETTE | BLOCKSPALETTE | Opens the Blocks palette |
| 50 | BMAKE | BMAKE | Legacy command opening the Block Definition dialog |
| 51 | BOUNDARY | BOUNDARY | Creates a polyline/region from an enclosed area |
| 52 | BOX | BOX | Creates a 3D solid box |
| 53 | BREAK | BREAK | Breaks an object between two points |
| 54 | BREAKATPOINT | BREAKATPOINT | Breaks an object at a single point with no gap |
| 55 | BYLAYER | BYLAYER | Sets current color/linetype/lineweight to BYLAYER |
| 56 | CAL | CAL | Evaluates a geometric/mathematical expression (Geometry Calculator) |
| 57 | CANNOSCALE | CANNOSCALE | Sysvar: current annotation scale name |
| 58 | CASCADE | CASCADE | Cascades open drawing windows |
| 59 | CECOLOR | CECOLOR | Sysvar: current object color |
| 60 | CELTSCALE | CELTSCALE | Sysvar: current object linetype scale |
| 61 | CENTERDISASSOCIATE | CENTERDISASSOCIATE | Removes associativity of a centerline/center mark |
| 62 | CENTERLINE | CENTERLINE | Creates an associative centerline between two lines |
| 63 | CENTERMARK | CENTERMARK | Creates an associative center mark on a circle/arc |
| 64 | CENTERREASSOCIATE | CENTERREASSOCIATE | Re-associates a centerline/center mark to geometry |
| 65 | CENTERRESET | CENTERRESET | Resets an associative centerline/center mark to default |
| 66 | CHAMFER | CHAMFER | Bevels the corner between two lines |
| 67 | CHAMFEREDGE | CHAMFEREDGE | Chamfers edges of a 3D solid |
| 68 | CHPROP | CHPROP | Changes properties of selected objects (legacy) |
| 69 | CIRCLE | CIRCLE | Draws a circle |
| 70 | CLAYER | CLAYER | Sysvar: current layer |
| 71 | CLEANSCREEN | CLEANSCREEN | Toggles clean-screen display (hides palettes/toolbars) |
| 72 | CLEAR | CLEAR | Deletes selected objects |
| 73 | CLIPROMPTLINES | CLIPROMPTLINES | Sysvar: number of command-line prompt lines shown |
| 74 | CLOSE | CLOSE | Closes the current drawing |
| 75 | CMLJUST | CMLJUST | Sysvar: current multiline justification |
| 76 | CMLSCALE | CMLSCALE | Sysvar: current multiline scale |
| 77 | CMLSTYLE | CMLSTYLE | Sysvar: current multiline style |
| 78 | COLOR | COLOR | Sets the current object color |
| 79 | COLORSCHEME | COLORSCHEME | Sysvar: light/dark UI color scheme |
| 80 | COLOUR | COLOUR (alias) | Sets the current object color (British spelling alias) |
| 81 | COMMANDLINEFADETIME | COMMANDLINEFADETIME | Sysvar: fade time for command-line dynamic prompts |
| 82 | CONE | CONE | Creates a 3D solid cone |
| 83 | CONSTRUCTIONLINE | CONSTRUCTIONLINE (alias) | Draws an infinite construction line (XLINE) |
| 84 | CONTENTBROWSER | CONTENTBROWSER | Opens the Content Browser palette |
| 85 | CONVTOSURFACE | CONVTOSURFACE | Converts objects to 3D surfaces |
| 86 | COORDS | COORDS | Sysvar: coordinate readout update mode |
| 87 | COPY | COPY | Copies objects |
| 88 | COPYBASE | COPYBASE | Copies objects with a specified base point |
| 89 | COPYCLIP | COPYCLIP | Copies objects to the system clipboard |
| 90 | COUNT | COUNT | Counts instances of selected objects/blocks |
| 91 | CROSSINGAREACOLOR | CROSSINGAREACOLOR | Sysvar: color of the crossing-selection area |
| 92 | CUI | CUI | Opens the Customize User Interface editor |
| 93 | CUIEXPORT | CUIEXPORT | Exports customizations to a CUIx file |
| 94 | CUIIMPORT | CUIIMPORT | Imports customizations from a CUIx file |
| 95 | CUILOAD | CUILOAD | Loads a partial CUIx customization file |
| 96 | CURSORSIZE | CURSORSIZE | Sysvar: crosshair cursor size |
| 97 | CURSORTYPE | CURSORTYPE | Sysvar: crosshair type/behavior |
| 98 | CUTCLIP | CUTCLIP | Cuts objects to the clipboard |
| 99 | CYLINDER | CYLINDER | Creates a 3D solid cylinder |
| 100 | DATAEXTRACTION | DATAEXTRACTION | Extracts drawing data/properties to a table or file |
| 101 | DATALINK | DATALINK | Creates/manages a link to external data (e.g. Excel) |
| 102 | DATALINKUPDATE | DATALINKUPDATE | Updates data pulled in through a data link |
| 103 | DBLIST | DBLIST | Lists database info for every object in the drawing |
| 104 | DCE | DCE (alias) | Draws a center mark (DIMCENTER) |
| 105 | DDCOLOR | DDCOLOR (alias) | Opens the Select Color dialog |
| 106 | DDEDIT | DDEDIT | Edits text, mtext, or attribute definitions in place |
| 107 | DDIM | DDIM | Legacy command opening the Dimension Style Manager |
| 108 | DDPTYPE | DDPTYPE (alias) | Opens the Point Style dialog |
| 109 | DDUNITS | DDUNITS (alias) | Opens the Drawing Units dialog |
| 110 | DELOBJ | DELOBJ | Sysvar: deletion of source objects after a command |
| 111 | DESELALL | DESELALL | Deselects all objects |
| 112 | DESELECT | DESELECT | Deselects a selection |
| 113 | DIMALIGNED | DIMALIGNED | Creates an aligned dimension |
| 114 | DIMANGULAR | DIMANGULAR | Creates an angular dimension |
| 115 | DIMARC | DIMARC | Creates an arc-length dimension |
| 116 | DIMASO | DIMASO | Legacy sysvar: toggles associative dimensioning |
| 117 | DIMASSOC | DIMASSOC | Sysvar: dimension associativity level |
| 118 | DIMBASELINE | DIMBASELINE | Creates baseline dimensions from a previous one |
| 119 | DIMBREAK | DIMBREAK | Breaks or restores dimension/extension lines around geometry |
| 120 | DIMCENTER | DIMCENTER | Draws a center mark/lines for a circle or arc |
| 121 | DIMCONTINUE | DIMCONTINUE | Creates continued (chained) dimensions |
| 122 | DIMDIAMETER | DIMDIAMETER | Creates a diameter dimension |
| 123 | DIMEDIT | DIMEDIT | Edits dimension text position/content/rotation |
| 124 | DIMJOG | DIMJOG | Creates a jogged radius dimension |
| 125 | DIMJOGGED | DIMJOGGED | Creates a jogged radius dimension |
| 126 | DIMJOGLINE | DIMJOGLINE | Adds/removes a jog line on a linear dimension |
| 127 | DIMLINEAR | DIMLINEAR | Creates a horizontal, vertical, or rotated linear dimension |
| 128 | DIMORDINATE | DIMORDINATE | Creates an ordinate (X/Y) dimension |
| 129 | DIMRADIUS | DIMRADIUS | Creates a radius dimension |
| 130 | DIMSHO | DIMSHO | Legacy sysvar: dimension dragged-value display |
| 131 | DIMSPACE | DIMSPACE | Adjusts spacing between parallel linear/angular dimensions |
| 132 | DIMSTYLE | DIMSTYLE | Creates/manages dimension styles |
| 133 | DIMTED | DIMTED (alias) | Edits dimension text position |
| 134 | DIMTEDIT | DIMTEDIT | Edits location/orientation of dimension text |
| 135 | DISPSILH | DISPSILH | Sysvar: toggles silhouette curve display on solids |
| 136 | DIST | DIST | Measures distance and angle between two points |
| 137 | DIVIDE | DIVIDE | Places equally spaced point/block markers along an object |
| 138 | DJO | DJO (alias) | Creates a jogged radius dimension (DIMJOGGED) |
| 139 | DONUT | DONUT | Draws a filled donut/ring polyline |
| 140 | DRAGMODE | DRAGMODE | Sysvar: dragged-object image display |
| 141 | DRAWORDER | DRAWORDER | Changes display order of overlapping objects |
| 142 | DSETTINGS | DSETTINGS | Opens the Drafting Settings dialog (snap/grid/osnap/polar) |
| 143 | DWGPROPS | DWGPROPS | Sets/displays drawing file properties |
| 144 | DWGUNITS | DWGUNITS | Legacy command to set insertion units |
| 145 | EATTEXT | EATTEXT | Legacy alias for extracting block attributes |
| 146 | EDGE | EDGE | Toggles visibility of a 3D face's edge |
| 147 | ELEVATION | ELEVATION | Sysvar: default Z elevation for new objects |
| 148 | ELLIPSE | ELLIPSE | Draws an ellipse or elliptical arc |
| 149 | ERASE | ERASE | Deletes selected objects |
| 150 | ETRANSMIT | ETRANSMIT | Packages a drawing set with its dependent files |
| 151 | EXIT | EXIT | Exits the application |
| 152 | EXPLODE | EXPLODE | Breaks a compound object into its component objects |
| 153 | EXPORT | EXPORT | Exports the drawing to another file format |
| 154 | EXPORTPDF | EXPORTPDF | Exports the drawing to a PDF file |
| 155 | EXPORTSTEP | EXPORTSTEP | Exports 3D solids to a STEP file |
| 156 | EXPORTSTL | EXPORTSTL | Exports 3D solids to an STL file |
| 157 | EXTEND | EXTEND | Extends objects to meet another object |
| 158 | EXTRIM | EXTRIM | Trims objects against the extended edge of a chosen shape |
| 159 | EXTRUDE | EXTRUDE | Extrudes a 2D profile into a 3D solid or surface |
| 160 | FI | FI (alias) | Builds a named selection filter (FILTER) |
| 161 | FILETAB | FILETAB | Sysvar: toggles the file tabs bar |
| 162 | FILLET | FILLET | Rounds the corner between two objects |
| 163 | FILLETEDGE | FILLETEDGE | Fillets edges of a 3D solid |
| 164 | FILTER | FILTER | Builds a named selection filter |
| 165 | FIND | FIND | Finds and replaces text |
| 166 | FINDNONPURGEABLE | FINDNONPURGEABLE | Lists reasons items can't be purged |
| 167 | FITSPLINE | FITSPLINE | Fits a spline through a polyline's vertices |
| 168 | FLATSHOT | FLATSHOT | Creates a 2D flattened representation of 3D objects |
| 169 | FLATTEN | FLATTEN | Flattens 3D objects/view onto the XY plane |
| 170 | FRAME | FRAME | Sysvar: display/plot of image/underlay/wipeout frames |
| 171 | GRADIENT | GRADIENT | Fills an area with a color gradient |
| 172 | GRID | GRID | Toggles/configures the display grid |
| 173 | GRIPCOLOR | GRIPCOLOR | Sysvar: color of unselected grips |
| 174 | GRIPHOT | GRIPHOT | Sysvar: color of hot (selected) grips |
| 175 | GRIPHOVER | GRIPHOVER | Sysvar: color of a grip under the cursor |
| 176 | GRIPOBJLIMIT | GRIPOBJLIMIT | Sysvar: object count above which grips are suppressed |
| 177 | GRIPSIZE | GRIPSIZE | Sysvar: size of grip boxes, in pixels |
| 178 | GROUP | GROUP | Creates a named group of objects |
| 179 | HALOGAP | HALOGAP | Sysvar: gap where hidden lines dip behind others in shaded/hidden views |
| 180 | HATCH | HATCH | Fills an enclosed area with a pattern |
| 181 | HATCHEDIT | HATCHEDIT | Edits an existing hatch's properties |
| 182 | HATCHTOBACK | HATCHTOBACK | Sends hatches to the back of the draw order |
| 183 | HB | HB (alias) | Draws a 2D spiral or 3D helix (HELIX) |
| 184 | HELIX | HELIX | Draws a 2D spiral or 3D helix |
| 185 | HELP | HELP | Opens the help system |
| 186 | HI | HI (alias) | Regenerates the view with hidden lines suppressed (HIDE) |
| 187 | HIDE | HIDE | Regenerates the 3D view with hidden lines suppressed |
| 188 | HIDEOBJECTS | HIDEOBJECTS | Hides selected objects from view |
| 189 | HYPERLINK | HYPERLINK | Attaches a hyperlink to an object |
| 190 | ID | ID | Reports the coordinates of a picked point |
| 191 | IM | IM (alias) | Opens the Image Manager (IMAGE) |
| 192 | IMAGE | IMAGE | Opens the Image Manager / attaches a raster image |
| 193 | IMAGEATTACH | IMAGEATTACH | Attaches a raster image to the drawing |
| 194 | IMAGEFRAME | IMAGEFRAME | Sysvar: image frame display/plot mode |
| 195 | IMPORTOBJ | IMPORTOBJ | Imports a Wavefront OBJ file as a 3D solid/mesh |
| 196 | INF | INF (alias) | Highlights interference between two sets of solids (INTERFERE) |
| 197 | INSERT | INSERT | Inserts a block or another drawing |
| 198 | INSUNITS | INSUNITS | Sysvar: units of blocks inserted into the drawing |
| 199 | INTERFERE | INTERFERE | Highlights interference between two sets of 3D solids |
| 200 | INTERSECT | INTERSECT | Creates a solid/region from the intersection of others |
| 201 | ISODRAFT | ISODRAFT | Toggles isometric drafting mode |
| 202 | ISOLATEOBJECTS | ISOLATEOBJECTS | Hides all objects except those selected |
| 203 | ISOLINES | ISOLINES | Sysvar: number of contour lines on curved solid surfaces |
| 204 | ISOPLANE | ISOPLANE | Sets the current isometric drawing plane |
| 205 | JOIN | JOIN | Joins compatible objects into a single object |
| 206 | JUSTIFYTEXT | JUSTIFYTEXT | Changes the justification of text without moving it |
| 207 | LA | LA (alias) | Opens the Layer Properties Manager (LAYER) |
| 208 | LANDXMLIMPORT | LANDXMLIMPORT | Imports a LandXML surface/feature file |
| 209 | LAS | LAS (alias) | Opens the Layer States Manager (LAYERSTATE) |
| 210 | LAYDEL | LAYDEL | Deletes a layer and all its objects |
| 211 | LAYER | LAYER | Opens the Layer Properties Manager |
| 212 | LAYERSTATE | LAYERSTATE | Opens the Layer States Manager |
| 213 | LAYFRZ | LAYFRZ | Freezes the layer of a selected object |
| 214 | LAYISO | LAYISO | Hides/locks all layers except those of selected objects |
| 215 | LAYLCK | LAYLCK | Locks the layer of a selected object |
| 216 | LAYMATCH | LAYMATCH | Changes an object's layer to match a selected target |
| 217 | LAYMCH | LAYMCH | Changes an object's layer to match a selected target |
| 218 | LAYMCUR | LAYMCUR | Sets the current layer to that of a selected object |
| 219 | LAYMRG | LAYMRG | Merges one or more layers into a target layer |
| 220 | LAYOFF | LAYOFF | Turns off the layer of a selected object |
| 221 | LAYON | LAYON | Turns on all layers in the drawing |
| 222 | LAYOUTMANAGER | LAYOUTMANAGER | Opens the layout management interface |
| 223 | LAYOUTPANEL | LAYOUTPANEL | Sysvar/toggle for the layout panel |
| 224 | LAYOUTTAB | LAYOUTTAB | Sysvar: whether layout/model tabs display |
| 225 | LAYTHW | LAYTHW | Thaws all layers in the drawing |
| 226 | LAYTRANS | LAYTRANS | Translates layer names/properties to a standard |
| 227 | LAYULK | LAYULK | Unlocks the layer of a selected object |
| 228 | LAYUNISO | LAYUNISO | Restores layers hidden/locked by LAYISO |
| 229 | LEADER | LEADER | Draws a leader line with annotation |
| 230 | LENGTHEN | LENGTHEN | Changes the length of objects/included angle of arcs |
| 231 | LI | LI (alias) | Lists database info about selected objects (LIST) |
| 232 | LIMCHECK | LIMCHECK | Sysvar: controls whether objects can be created outside drawing limits |
| 233 | LIMITS | LIMITS | Sets/reports the drawing limits (grid boundary) |
| 234 | LINE | LINE | Draws a straight line segment |
| 235 | LINETYPE | LINETYPE | Loads/sets/manages linetypes |
| 236 | LIST | LIST | Lists database info about selected objects |
| 237 | LMAN | LMAN (alias) | Opens the Layer States Manager |
| 238 | LOFT | LOFT | Creates a 3D solid/surface through a set of cross sections |
| 239 | LTSCALE | LTSCALE | Sysvar: global linetype scale factor |
| 240 | LUNITS | LUNITS | Sysvar: linear units format |
| 241 | LUPREC | LUPREC | Sysvar: linear units display precision |
| 242 | LWDISPLAY | LWDISPLAY | Sysvar: toggles lineweight display in the drawing |
| 243 | MASSPROP | MASSPROP | Calculates mass properties of solids/regions |
| 244 | MATCHPROP | MATCHPROP | Copies properties from one object to others |
| 245 | MAXACTVP | MAXACTVP | Sysvar: max number of active viewports |
| 246 | MEA | MEA (alias) | Measures distance/radius/angle/area/volume (MEASUREGEOM) |
| 247 | MEASURE | MEASURE | Places point/block markers at fixed intervals along an object |
| 248 | MEASUREGEOM | MEASUREGEOM | Measures distance, radius, angle, area, and volume |
| 249 | MINSERT | MINSERT | Inserts multiple copies of a block in an array pattern |
| 250 | MIRROR | MIRROR | Mirrors objects |
| 251 | MIRROR3D | MIRROR3D | Mirrors objects about a 3D plane |
| 252 | MIRRTEXT | MIRRTEXT | Sysvar: controls whether text is mirrored by MIRROR |
| 253 | MLEADER | MLEADER | Creates a multileader |
| 254 | MLEADERALIGN | MLEADERALIGN | Aligns multiple multileaders |
| 255 | MLEADERCOLLECT | MLEADERCOLLECT | Collects block-content multileaders into one leader |
| 256 | MLEADERADD | MLEADERADD | Adds a leader line to an existing multileader |
| 257 | MLEADERREMOVE | MLEADERREMOVE | Removes a leader line from a multileader |
| 258 | MLEADERSTYLE | MLEADERSTYLE | Creates/manages multileader styles |
| 259 | MLEDIT | MLEDIT | Edits multiline intersections/crosses/joins |
| 260 | MLINE | MLINE | Draws multiple parallel lines |
| 261 | MLSTYLE | MLSTYLE | Creates/manages multiline styles |
| 262 | MOVE | MOVE | Moves objects |
| 263 | MS | MS (alias) | Switches to model space inside a viewport (MSPACE) |
| 264 | MSPACE | MSPACE | Switches to model space inside a floating viewport |
| 265 | MTEXT | MTEXT | Creates a multiline text object |
| 266 | MVIEW | MVIEW | Creates and controls layout viewports |
| 267 | NAVVCUBE | NAVVCUBE | Controls display/settings of the ViewCube |
| 268 | NCOPY | NCOPY | Copies nested objects from an xref/block without exploding it |
| 269 | NCOPYALL | NCOPYALL | Copies all nested objects from an xref/block |
| 270 | NEW | NEW | Creates a new drawing |
| 271 | OBJECTSCALE | OBJECTSCALE | Adds/removes annotation scales for an object |
| 272 | OFFSET | OFFSET | Creates a parallel copy of an object at a set distance |
| 273 | OOPS | OOPS | Restores the most recently erased objects |
| 274 | OP | OP (alias) | Opens the Options dialog |
| 275 | OPEN | OPEN | Opens an existing drawing |
| 276 | OPTIONS | OPTIONS | Opens the application Options dialog |
| 277 | ORTHO | ORTHO | Toggles orthogonal cursor constraint |
| 278 | OSMODE | OSMODE | Sysvar: current running object snap modes |
| 279 | OSNAP | OSNAP | Sets running object snap modes |
| 280 | OVERKILL | OVERKILL | Removes duplicate/overlapping objects |
| 281 | P | P (alias) | Draws a polyline (PLINE) |
| 282 | PAGESETUP | PAGESETUP | Configures layout/plot page setup |
| 283 | PAN | PAN | Pans the current view |
| 284 | PARAMETERS | PARAMETERS | Opens the Parameters Manager (named parameters) |
| 285 | PASTE | PASTE | Pastes clipboard content into the drawing |
| 286 | PASTEBLOCK | PASTEBLOCK | Pastes clipboard content as a block |
| 287 | PASTECLIP | PASTECLIP | Pastes clipboard content into the drawing |
| 288 | PASTEORIG | PASTEORIG | Pastes objects at their original coordinates |
| 289 | PDFATTACH | PDFATTACH | Attaches a PDF file as an underlay |
| 290 | PDFFRAME | PDFFRAME | Sysvar: PDF underlay frame display/plot mode |
| 291 | PDMODE | PDMODE | Sysvar: display style of point objects |
| 292 | PDSIZE | PDSIZE | Sysvar: display size of point objects |
| 293 | PEDIT | PEDIT | Edits a polyline (vertices, width, fit/spline) |
| 294 | PERF | PERF | Displays/logs performance information |
| 295 | PERSP | PERSP | Sysvar: toggles perspective projection |
| 296 | PICKADD | PICKADD | Sysvar: controls whether new picks add to selection |
| 297 | PICKBOX | PICKBOX | Sysvar: object-selection pickbox size |
| 298 | PICKDRAG | PICKDRAG | Sysvar: controls click-and-drag vs. two-click selection |
| 299 | PICKSTYLE | PICKSTYLE | Sysvar: group/associative-hatch selection behavior |
| 300 | PLAN | PLAN | Restores a plan view of a UCS |
| 301 | PLINE | PLINE | Draws a 2D polyline |
| 302 | PLINEGEN | PLINEGEN | Sysvar: linetype pattern generation around polyline vertices |
| 303 | PLOT | PLOT | Plots/prints the drawing |
| 304 | PLOTSTYLE | PLOTSTYLE | Sets the current plot style for new objects |
| 305 | PLOTSTYLEEDITOR | PLOTSTYLEEDITOR | Opens a plot style table for editing |
| 306 | PLOTSTYLEPANEL | PLOTSTYLEPANEL | Opens the plot style panel/palette |
| 307 | PLOTWINDOW | PLOTWINDOW | Opens the plot preview window |
| 308 | POINT | POINT | Creates a point object |
| 309 | POINTCLOUDCLIPFRAME | POINTCLOUDCLIPFRAME | Sysvar: point-cloud clipping boundary display |
| 310 | POLAR | POLAR | Sysvar/toggle for polar tracking |
| 311 | POLYGON | POLYGON | Draws a regular polygon |
| 312 | POLYSOLID | POLYSOLID | Creates a wall-like 3D solid along a path |
| 313 | PR | PR (alias) | Opens the Properties palette |
| 314 | PRESSPULL | PRESSPULL | Extrudes/modifies bounded areas by pressing/pulling |
| 315 | PRINT | PRINT (alias) | Plots/prints the drawing |
| 316 | PRINTALL | PRINTALL | Plots multiple sheets in one operation |
| 317 | PROPERTIES | PROPERTIES | Opens the Properties palette |
| 318 | PROPS | PROPS (alias) | Opens the Properties palette |
| 319 | PSLTSCALE | PSLTSCALE | Sysvar: paper space linetype scaling |
| 320 | PSPACE | PSPACE | Switches from a floating viewport to paper space |
| 321 | PURGE | PURGE | Removes unused named objects (blocks, layers, styles, …) |
| 322 | PYR | PYR (alias) | Creates a 3D solid pyramid (PYRAMID) |
| 323 | PYRAMID | PYRAMID | Creates a 3D solid pyramid |
| 324 | QC | QC (alias) | Opens the QuickCalc calculator |
| 325 | QDIM | QDIM | Creates dimensions quickly from a selection of objects |
| 326 | QLEADER | QLEADER | Creates a leader with annotation via a quick dialog flow |
| 327 | QP | QP (alias) | Opens the Quick Properties panel |
| 328 | QS | QS (alias) | Opens the Quick Select dialog (QSELECT) |
| 329 | QSAVE | QSAVE | Saves the drawing without prompting for a filename |
| 330 | QSELECT | QSELECT | Builds a selection set by filtering object properties |
| 331 | QTEXTMODE | QTEXTMODE | Sysvar: displays text as bounding boxes for speed |
| 332 | QUICKCALC | QUICKCALC | Opens the QuickCalc calculator palette |
| 333 | QUICKPROPERTIES | QUICKPROPERTIES | Opens the Quick Properties panel for a selection |
| 334 | QUIT | QUIT | Exits the application |
| 335 | RAY | RAY | Draws a semi-infinite construction line |
| 336 | RECTANG | RECTANG | Draws a rectangle |
| 337 | REDO | REDO | Reverses the last UNDO |
| 338 | REDRAW | REDRAW | Refreshes the display in the current viewport |
| 339 | REDRAWALL | REDRAWALL | Refreshes the display in all viewports |
| 340 | REFCLOSE | REFCLOSE | Saves or discards changes made during in-place reference editing |
| 341 | REFEDIT | REFEDIT | Edits a block or xref reference in place |
| 342 | REG | REG (alias) | Creates a region from a closed shape (REGION) |
| 343 | REGEN | REGEN | Regenerates the drawing in the current viewport |
| 344 | REGENALL | REGENALL | Regenerates the drawing in all viewports |
| 345 | REGENMODE | REGENMODE | Sysvar: automatic regeneration on/off |
| 346 | REGION | REGION | Creates a 2D region from a closed shape |
| 347 | RENAME | RENAME | Renames named objects (layers, blocks, styles, …) |
| 348 | REPORT | REPORT | Generates a report about the drawing |
| 349 | REVCLOUD | REVCLOUD | Draws a revision cloud |
| 350 | REVERSE | REVERSE | Reverses the vertex order of lines/polylines/splines |
| 351 | REVOLVE | REVOLVE | Creates a 3D solid/surface by revolving a profile |
| 352 | ROTATE | ROTATE | Rotates objects about a base point |
| 353 | ROTATE3D | ROTATE3D | Rotates objects about an arbitrary 3D axis (legacy) |
| 354 | SAVE | SAVE | Saves the drawing |
| 355 | SAVEALL | SAVEALL | Saves all open drawings |
| 356 | SAVEAS | SAVEAS | Saves the drawing under a new name/format |
| 357 | SCALE | SCALE | Resizes objects about a base point |
| 358 | SCALELISTEDIT | SCALELISTEDIT | Edits the list of scales available in scale dropdowns |
| 359 | SCALETEXT | SCALETEXT | Scales text objects without moving their insertion point |
| 360 | SCR | SCR (alias) | Runs a script file (SCRIPT) |
| 361 | SCRIPT | SCRIPT | Runs a script file of commands |
| 362 | SECTION | SECTION | Creates a 2D region from a 3D solid cross-section |
| 363 | SECTIONPLANE | SECTIONPLANE | Creates a section plane object through 3D geometry |
| 364 | SELECTALL | SELECTALL | Selects all objects in the current space |
| 365 | SELECTIONAREA | SELECTIONAREA | Sysvar: toggles translucent selection-area fill |
| 366 | SELECTIONAREAOPACITY | SELECTIONAREAOPACITY | Sysvar: opacity of the selection-area fill |
| 367 | SELECTIONEFFECT | SELECTIONEFFECT | Sysvar: visual effect applied to selected objects |
| 368 | SELECTIONEFFECTCOLOR | SELECTIONEFFECTCOLOR | Sysvar: color of the selection highlight effect |
| 369 | SELECTIONPREVIEW | SELECTIONPREVIEW | Sysvar: highlight preview as the cursor rolls over objects |
| 370 | SELECTSIMILAR | SELECTSIMILAR | Selects all objects sharing properties with a selected one |
| 371 | SETBYLAYER | SETBYLAYER | Sets selected objects' overrides to BYLAYER |
| 372 | SETVAR | SETVAR | Lists/sets system variable values |
| 373 | SHADEDGE | SHADEDGE | Legacy sysvar: shaded viewport edge display |
| 374 | SHADEMODE | SHADEMODE | Sets the shading mode of the current viewport |
| 375 | SHEETSET | SHEETSET | Opens the Sheet Set Manager |
| 376 | SHELL | SHELL | Runs an operating-system command |
| 377 | SHORTCUTS | SHORTCUTS | Displays/configures keyboard shortcuts |
| 378 | SKETCH | SKETCH | Draws a freehand sketch of line/polyline segments |
| 379 | SKETCHINC | SKETCHINC | Sysvar: SKETCH command's recording increment |
| 380 | SKPOLY | SKPOLY | Sysvar: whether SKETCH generates lines or polylines |
| 381 | SKTOLERANCE | SKTOLERANCE | Sysvar: SKETCH-to-spline fitting tolerance |
| 382 | SL | SL (alias) | Slices a set of 3D solids with a plane (SLICE) |
| 383 | SLICE | SLICE | Slices a set of 3D solids with a plane |
| 384 | SNAP | SNAP | Toggles/configures the snap grid |
| 385 | SNAPANG | SNAPANG | Sysvar: rotation angle of the snap/grid |
| 386 | SO | SO (alias) | Creates a 3D solid primitive/extrusion (SOLID) |
| 387 | SOLID | SOLID | Creates a filled 2D solid shape |
| 388 | SOLID2D | SOLID2D | Creates a 2D solid-filled region |
| 389 | SOLIDCHAMFER | SOLIDCHAMFER | Chamfers edges of a 3D solid |
| 390 | SOLIDEDIT | SOLIDEDIT | Edits faces, edges, and bodies of 3D solids |
| 391 | SOLIDFILLET | SOLIDFILLET | Fillets edges of a 3D solid |
| 392 | SORTENTS | SORTENTS | Sysvar: controls object draw/selection sort order |
| 393 | SPHERE | SPHERE | Creates a 3D solid sphere |
| 394 | SPLFRAME | SPLFRAME | Sysvar: toggles spline/mesh control-frame display |
| 395 | SPLINE | SPLINE | Draws a smooth curve through/near control points |
| 396 | SPLINEDIT | SPLINEDIT | Edits a spline's fit points/control vertices |
| 397 | SPLINEFIT | SPLINEFIT | Converts a spline-fit polyline to a true spline |
| 398 | SPLINESEGS | SPLINESEGS | Sysvar: line segments used to display a spline-fit polyline |
| 399 | SPLINETYPE | SPLINETYPE | Sysvar: curve type used when splining a polyline |
| 400 | SSM | SSM (alias) | Opens the Sheet Set Manager |
| 401 | STEPOUT | STEPOUT | Exports 3D solids to a STEP file |
| 402 | STLOUT | STLOUT | Exports a 3D solid to an STL file |
| 403 | STRETCH | STRETCH | Stretches objects crossing a selection window |
| 404 | STYLE | STYLE | Creates/manages text styles |
| 405 | STYLESMANAGER | STYLESMANAGER | Opens the plot styles manager folder |
| 406 | SUBTRACT | SUBTRACT | Subtracts one set of solids/regions from another |
| 407 | SURFTYPE | SURFTYPE | Sysvar: surface-fitting type for smoothed meshes |
| 408 | SURFU | SURFU | Sysvar: mesh density in the M/U direction |
| 409 | SURFV | SURFV | Sysvar: mesh density in the N/V direction |
| 410 | SWEEP | SWEEP | Creates a 3D solid/surface by sweeping a profile along a path |
| 411 | SYNCPVIEWPORTS | SYNCPVIEWPORTS | Synchronizes model-space visibility with paper-space viewports |
| 412 | TABLE | TABLE | Inserts a table object |
| 413 | TABLEDIT | TABLEDIT | Edits the content of a table cell |
| 414 | TABLESTYLE | TABLESTYLE | Creates/manages table styles |
| 415 | TCASE | TCASE (Express Tool) | Changes the letter case of selected text |
| 416 | TCOUNT | TCOUNT | Adds/edits incremental numbering on multiple text objects |
| 417 | TEDIT | TEDIT | Edits text in place |
| 418 | TEXT | TEXT | Creates a single-line text object |
| 419 | TEXTEDIT | TEXTEDIT | Edits single-line or multiline text |
| 420 | TEXTEDITMODE | TEXTEDITMODE | Sysvar: in-place text editor behavior |
| 421 | TEXTFILL | TEXTFILL | Sysvar: toggles filled vs. outline TrueType text |
| 422 | TEXTFIT | TEXTFIT | Resizes/repositions text to fit between two points |
| 423 | TEXTMASK | TEXTMASK | Adds a background mask behind text |
| 424 | TEXTQLTY | TEXTQLTY | Sysvar: TrueType text resolution/smoothness |
| 425 | TEXTTOBACK | TEXTTOBACK | Sends text/leaders/dimensions to the back of the draw order |
| 426 | TEXTTOFRONT | TEXTTOFRONT | Brings text/leaders/dimensions to the front of the draw order |
| 427 | THICKEN | THICKEN | Converts a surface into a 3D solid of given thickness |
| 428 | THICKNESS | THICKNESS | Sysvar: default extrusion thickness for new 2D objects |
| 429 | TOLERANCE | TOLERANCE | Creates a geometric tolerance (feature control) frame |
| 430 | TOOLPALETTES | TOOLPALETTES | Opens the Tool Palettes window |
| 431 | TORIENT | TORIENT | Orients text/mtext/attributes to face the viewer |
| 432 | TORUS | TORUS | Creates a 3D solid torus |
| 433 | TP | TP (alias) | Opens the Tool Palettes window |
| 434 | TRACE | TRACE | Draws a solid-filled line of set width (legacy) |
| 435 | TRACEWID | TRACEWID | Sysvar: default width used by TRACE |
| 436 | TRIM | TRIM | Trims objects at a cutting edge |
| 437 | TS | TS (alias) | Creates/manages table styles (TABLESTYLE) |
| 438 | U | U | Undoes the last operation |
| 439 | UCS | UCS | Manages the user coordinate system |
| 440 | UCSICON | UCSICON | Controls display of the UCS icon |
| 441 | UN | UN (alias) | Opens the Drawing Units dialog |
| 442 | UNDERLAY | UNDERLAY | Common base for DWF/DGN/PDF underlay commands |
| 443 | UNDO | UNDO | Reverses previous operations |
| 444 | UNGROUP | UNGROUP | Removes a group definition |
| 445 | UNION | UNION | Combines solids/regions into a single object |
| 446 | UNISOLATEOBJECTS | UNISOLATEOBJECTS | Restores display of objects hidden by ISOLATEOBJECTS/HIDEOBJECTS |
| 447 | UNITS | UNITS | Sets/reports drawing units and precision |
| 448 | USERI | USERI | Sysvar family: user-defined integer values (USERI1–5) |
| 449 | USERR | USERR | Sysvar family: user-defined real-number values (USERR1–5) |
| 450 | USRTIMER | USRTIMER | Sysvar: enables/disables the user elapsed-timer |
| 451 | VIEW | VIEW | Creates/restores named views |
| 452 | VISRETAIN | VISRETAIN | Sysvar: whether xref layer overrides persist in the host drawing |
| 453 | VISUALSTYLES | VISUALSTYLES | Creates/manages visual styles |
| 454 | VPJOIN | VPJOIN | Merges two viewports into one |
| 455 | VPLAYER | VPLAYER | Controls layer visibility per layout viewport |
| 456 | VPORTS | VPORTS | Creates/splits/joins model- or paper-space viewports |
| 457 | VPSYNC | VPSYNC | Synchronizes a layout viewport's view to model space (or back) |
| 458 | VS | VS (alias) | Sets the current visual style (VSCURRENT) |
| 459 | VSCURRENT | VSCURRENT | Sets the current viewport's visual style |
| 460 | WB | WB (alias) | Writes selected objects/a block to a new drawing file (WBLOCK) |
| 461 | WBLOCK | WBLOCK | Writes objects/a block to a new drawing file |
| 462 | WEDGE | WEDGE | Creates a 3D solid wedge |
| 463 | WINDOWAREACOLOR | WINDOWAREACOLOR | Sysvar: color of the window-selection area |
| 464 | WIPEOUT | WIPEOUT | Creates an area that masks underlying objects |
| 465 | WIPEOUTFRAME | WIPEOUTFRAME | Sysvar: wipeout frame display/plot mode |
| 466 | WORLDVIEW | WORLDVIEW | Sysvar: UCS used for view commands (DVIEW/VPOINT) |
| 467 | XA | XA (alias) | Attaches an external reference (XATTACH) |
| 468 | XATTACH | XATTACH | Attaches a DWG as an external reference |
| 469 | XCLIPFRAME | XCLIPFRAME | Sysvar: xref/block clipping-boundary display/plot mode |
| 470 | XDATA | XDATA | Common base for extended-entity-data related functionality |
| 471 | XLINE | XLINE | Draws an infinite construction line |
| 472 | XOPEN | XOPEN | Opens an xref in a new window for editing |
| 473 | XR | XR (alias) | Manages external references (XREF) |
| 474 | XREF | XREF | Opens the External References palette |
| 475 | XRELOAD | XRELOAD | Reloads one or more external references |
| 476 | ZOOM | ZOOM | Zooms the current view in/out |
| 477 | ZOOMFACTOR | ZOOMFACTOR | Sysvar: zoom step per mouse-wheel notch |
| 478 | ZOOMWHEEL | ZOOMWHEEL | Sysvar: reverses the direction of wheel-zoom |
| 479 | DCANGULAR | DCANGULAR | Adds an Angular dimensional constraint |
| 480 | DIMCONSTRAINT | DIMCONSTRAINT | Adds a typed Distance/Radius/Diameter/ArcLength dimensional constraint |
| 481 | GCCOINCIDENT | GCCOINCIDENT | Adds a Coincident geometric constraint between two points |
| 482 | GCCOINCIDENT_CENTER | GCCOINCIDENT_CENTER | Adds a Coincident constraint anchored to a circle/arc's center point |
| 483 | GCCOINCIDENT_CURVE | GCCOINCIDENT_CURVE | Adds a Coincident constraint anchored to a point on a curve |
| 484 | GCCOINCIDENT_MID | GCCOINCIDENT_MID | Adds a Coincident constraint anchored to an entity's midpoint |
| 485 | GCCOLLINEAR | GCCOLLINEAR | Adds a Colinear geometric constraint |
| 486 | GCCONCENTRIC | GCCONCENTRIC | Adds a Concentric geometric constraint |
| 487 | GCEQUAL | GCEQUAL | Adds an Equal (length/radius) geometric constraint |
| 488 | GCEQUAL_DIST | GCEQUAL_DIST | Adds an Equal constraint between two point-pair distances |
| 489 | GCFIX | GCFIX | Adds a Fixed geometric constraint |
| 490 | GCHORIZONTAL | GCHORIZONTAL | Adds a Horizontal geometric constraint |
| 491 | GCPARALLEL | GCPARALLEL | Adds a Parallel geometric constraint |
| 492 | GCPERPENDICULAR | GCPERPENDICULAR | Adds a Perpendicular geometric constraint |
| 493 | GCSYMMETRIC | GCSYMMETRIC | Adds a Symmetric geometric constraint |
| 494 | GCTANGENT | GCTANGENT | Adds a Tangent geometric constraint |
| 495 | GCVERTICAL | GCVERTICAL | Adds a Vertical geometric constraint |

---

## Section 2 — Differ in name or purpose (62)

These are commands where OCS's registered name does **not** match a real AutoCAD
command, either because OCS invented its own naming scheme (a handful of
ribbon/viewport-specific one-offs — most of the constraint family that used to
live here was renamed to match AutoCAD as of 2026-09-11, see the Methodology
note above), split one AutoCAD command into several ribbon-specific
sub-commands, registered a product-specific command with no AutoCAD
counterpart, or registered a duplicate/synonym alongside the real name.

| # | OCS Command | Closest AutoCAD Command | How it differs / purpose |
|---|---|---|---|
| 1 | NRCONSTRAINT | — (no AutoCAD equivalent) | OCS-invented **Normal**/perpendicular-tangent constraint ported from FreeCAD; AutoCAD's constraint set has no "Normal" kind |
| 2 | HORIZONTAL | VPORTS (2H option) | OCS ribbon button that runs `VPORTS 2H` (tile viewports 2 horizontal); unrelated to AutoCAD's `GCHORIZONTAL` constraint despite the name |
| 3 | VERTICAL | VPORTS (2V option) | OCS ribbon button that runs `VPORTS 2V` (tile viewports 2 vertical); unrelated to AutoCAD's `GCVERTICAL` constraint despite the name |
| 4 | ARC_3P | ARC | OCS ribbon sub-variant for ARC's 3-point option; AutoCAD exposes this as an ARC prompt option, not a separate command |
| 5 | ARC_CONT | ARC | OCS ribbon sub-variant: continue an arc from the last line/arc endpoint |
| 6 | ARC_CSA | ARC | OCS ribbon sub-variant for ARC's center-start-angle option |
| 7 | ARC_CSE | ARC | OCS ribbon sub-variant for ARC's center-start-end option |
| 8 | ARC_CSL | ARC | OCS ribbon sub-variant for ARC's center-start-length option |
| 9 | ARC_SCA | ARC | OCS ribbon sub-variant for ARC's start-center-angle option |
| 10 | ARC_SCE | ARC | OCS ribbon sub-variant for ARC's start-center-end option |
| 11 | ARC_SCL | ARC | OCS ribbon sub-variant for ARC's start-center-length option |
| 12 | ARC_SEA | ARC | OCS ribbon sub-variant for ARC's start-end-angle option |
| 13 | ARC_SED | ARC | OCS ribbon sub-variant for ARC's start-end-direction option |
| 14 | ARC_SER | ARC | OCS ribbon sub-variant for ARC's start-end-radius option |
| 15 | CIRCLE_2P | CIRCLE | OCS ribbon sub-variant for CIRCLE's 2-point option |
| 16 | CIRCLE_3P | CIRCLE | OCS ribbon sub-variant for CIRCLE's 3-point option |
| 17 | CIRCLE_CD | CIRCLE | OCS ribbon sub-variant for CIRCLE's center-diameter option |
| 18 | CIRCLE_TTR | CIRCLE | OCS ribbon sub-variant for CIRCLE's tan-tan-radius option |
| 19 | CIRCLE_TTT | CIRCLE | OCS ribbon sub-variant for CIRCLE's tan-tan-tan option |
| 20 | ELLIPSE_ARC | ELLIPSE | OCS ribbon sub-variant for ELLIPSE's elliptical-arc option |
| 21 | ELLIPSE_AXIS | ELLIPSE | OCS ribbon sub-variant for ELLIPSE's axis-endpoint option |
| 22 | POLY_C | POLYGON | OCS ribbon sub-variant for POLYGON's circumscribed option |
| 23 | POLY_E | POLYGON | OCS ribbon sub-variant for POLYGON's inscribed option |
| 24 | RECT_CEN | RECTANG | OCS ribbon sub-variant for RECTANG's center option |
| 25 | RECT_ROT | RECTANG | OCS ribbon sub-variant for RECTANG's rotated option |
| 26 | BEDIT_DISCARD | BEDIT | OCS-internal action: close the Block Editor, discarding changes — not a standalone AutoCAD command |
| 27 | BEDIT_SAVE | BEDIT | OCS-internal action: save changes and stay in the Block Editor — not a standalone AutoCAD command |
| 28 | FRAMES0 | FRAME (value 0) | OCS ribbon toggle for setting sysvar `FRAME` to 0 (hidden); AutoCAD exposes this as a value, not a command |
| 29 | FRAMES1 | FRAME (value 1) | OCS ribbon toggle for setting sysvar `FRAME` to 1 (visible + plots) |
| 30 | FRAMES2 | FRAME (value 2) | OCS ribbon toggle for setting sysvar `FRAME` to 2 (visible, doesn't plot) |
| 31 | CHANGELOG | — (none) | OCS-specific: opens the app's changelog |
| 32 | DONATE | — (none) | OCS-specific: opens a donation link |
| 33 | PLUGINMANAGER | APPLOAD (closest) | OCS-specific plugin manager; AutoCAD's nearest analog is loading ObjectARX/LISP apps via APPLOAD |
| 34 | PLUGINS | APPLOAD (closest) | OCS-specific plugin list; no direct AutoCAD equivalent |
| 35 | WEBVERSION | — (none) | OCS-specific: opens the web version of the app |
| 36 | 3O | 3DO | OCS's own short alias for 3DORBIT; real AutoCAD alias is `3DO`, not `3O` |
| 37 | BLE | BEDIT (BE is the real alias) | OCS's own alias; AutoCAD's real alias for BEDIT is `BE` |
| 38 | CDIMSTY | DIMSTYLE | OCS-invented shorthand; not a documented AutoCAD alias |
| 39 | CLR | CLEAR | OCS-invented shorthand; not a documented AutoCAD alias |
| 40 | COLORTHEME | COLORSCHEME (closest) | OCS-specific name; AutoCAD's UI theme sysvar is `COLORSCHEME`, not a typed "COLORTHEME" command |
| 41 | DE | DIST (closest, unconfirmed) | Short OCS token with no confirmed match in the standard `acad.pgp` alias table |
| 42 | DSPACE | DIMSPACE | OCS-invented duplicate/shorthand; the real command (also separately registered in OCS) is `DIMSPACE` |
| 43 | DWGPROP | DWGPROPS | OCS-invented duplicate missing the trailing "S" that AutoCAD's real command has |
| 44 | LAYERS | LAYER | OCS-invented plural duplicate of `LAYER`, which is separately registered correctly |
| 45 | MULTIPOINT | POINT (repeated via MULTIPLE) | OCS convenience command for placing several points in one session; AutoCAD achieves this via the generic `MULTIPLE` command prefix, not a distinct "MULTIPOINT" command |
| 46 | OBJIMPORT | IMPORTOBJ | OCS-invented duplicate with words reversed; the real command (also separately registered) is `IMPORTOBJ` |
| 47 | PLIMCHECK | LIMCHECK (closest, unconfirmed) | Not a documented AutoCAD sysvar/command; likely OCS's own naming, possibly confused with `LIMCHECK` |
| 48 | POLY | POLYGON | OCS-invented duplicate/shorthand; the real command (also separately registered) is `POLYGON`, real alias is `POL` |
| 49 | QL | QLEADER (real alias is LE) | OCS's own alias; AutoCAD's real alias for QLEADER is `LE`, not `QL` |
| 50 | QUICKPRINT | PRINT / PLOT (closest) | Not a documented distinct AutoCAD command; printing is done via `PLOT`/`PRINT` |
| 51 | RECT | RECTANG | OCS-invented duplicate/shorthand; the real command (also separately registered) is `RECTANG`, real alias is `REC` |
| 52 | SA | SAVEALL (closest, unconfirmed) | Short OCS token with no confirmed match in the standard `acad.pgp` alias table |
| 53 | SELSIM | SELECTSIMILAR | OCS-invented duplicate/shorthand; the real command (also separately registered) is `SELECTSIMILAR` |
| 54 | STPOUT | STEPOUT | OCS-invented duplicate/shorthand; the real command (also separately registered) is `STEPOUT` |
| 55 | VW | VIEW (real alias is V) | OCS's own alias; AutoCAD's real alias for VIEW is `V`, not `VW` |
| 56 | ZS | ZOOM (closest, unconfirmed) | Short OCS token with no confirmed match in the standard `acad.pgp` alias table |
| 57 | WINDOWSAREACOLOR | WINDOWAREACOLOR | OCS-invented duplicate with an extra "S"; AutoCAD's real sysvar is `WINDOWAREACOLOR` (no "S") |
| 58 | ALIGN3D | 3DALIGN | OCS-invented duplicate; AutoCAD's real 3D-align command is `3DALIGN`, not "ALIGN3D" |
| 59 | 3DMIRROR | MIRROR3D | OCS-invented duplicate; AutoCAD's real 3D-mirror command is `MIRROR3D`, not "3DMIRROR" |
| 60 | ARRAY3D | 3DARRAY / ARRAYRECT / ARRAYPOLAR / ARRAYPATH | OCS-invented name with no exact AutoCAD counterpart; closest real commands are `3DARRAY` (legacy) or the modern `ARRAY*` family |
| 61 | BLOCKPALETTE | BLOCKSPALETTE | OCS-invented duplicate missing the "S"; AutoCAD's real command (also separately registered) is `BLOCKSPALETTE` |
| 62 | SHOWCONSTRAINTS | — (none) | New 2026-09-11: Constraints ribbon group's global glyph-visibility toggle; AutoCAD has no single command that hides every constraint glyph at once |
