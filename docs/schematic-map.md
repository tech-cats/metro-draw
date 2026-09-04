# Schematic Manifest and Rendering

## Overview

The repository provides Rust data models and YAML/JSON schemas for both the
topology manifest and the semantic schematic manifest. Topology manifests are
validated and rendered as topology graphs. Semantic schematic manifests pass
semantic and geometric validation and are rendered as SVG schematic maps. A
topology-to-schematic converter is not implemented.

The main dataflows are:

```text
Topology Manifest --> Topology Graph

Semantic Schematic Manifest --> Prepared Schematic --> Schematic Map
```

This document defines the boundary between the semantic schematic manifest and
the renderer's private prepared geometry. The current schematic map contains
station symbols and line strokes.

## Separation of concerns

The **semantic schematic manifest** is the standalone, human-editable YAML or
JSON document. It describes metro concepts: station and line identities,
localised names, line colours, station symbols, route visits, corners, and
global options. It is the source of truth for the schematic map.

The **prepared schematic** is a private, in-memory model derived from a valid
semantic schematic manifest. It describes only the geometric and visual data
required by the renderer: symbols, strokes, and resolved path points. It is
not a second authoring format and has no YAML or JSON representation.

Prepared-geometry validation is shared by `check` and the renderer. The renderer
derives circular arcs, symbol outlines, and SVG bounds while rendering; these
temporary calculations are implementation details rather than another
manifest or architectural stage. Output filesystem failures are separate from
manifest validity.

## Semantic schematic manifest

The semantic manifest retains the concepts needed to author and render a map.

```rust
// Visibility modifiers are omitted for brevity.
struct SchematicManifest {
    options: SchematicOptions,
    stations: Vec<SchematicStation>,
    corners: Vec<SchematicCorner>,
    lines: Vec<SchematicLine>,
}

struct SchematicStation {
    id: String,
    position: SchematicPoint,
    names: LocalizedNames,
    symbol: SchematicStationSymbol,
}

enum SchematicStationSymbol {
    Circle,
    Capsule {
        axis: OctilinearAxis,
        anchor_count: u8,
        anchor_interval: SchematicLength,
    },
}
```

`SchematicStationSymbol` describes the requested symbol rather than claiming
that every topological interchange must use a particular shape. Each station
requests a circle or a capsule, with capsule symbols carrying an explicit
axis and anchor configuration.

Lines describe semantic station visits and explicit layout corners. These are
not renderer path points: they are instructions from which the resolver
generates geometric objects.

```rust
struct SchematicLine {
    id: String,
    names: LocalizedNames,
    color: String,
    paths: Vec<SchematicPath>,
}

struct SchematicPath {
    visits: Vec<SchematicRouteVisit>,
    closed: bool,
}

enum SchematicRouteVisit {
    Station {
        station_id: String,
        port: SchematicStationPort,
    },
    Corner {
        corner_id: String,
    },
}

enum SchematicStationPort {
    SingleLine,
    Interchange(SchematicInterchangePort),
}

enum SchematicInterchangePort {
    MajorAxis,
    RisingOblique,
    FallingOblique,
    SinglePerpendicular,
    PerpendicularAnchor { index: u8 },
}

struct SchematicCorner {
    id: String,
    position: SchematicPoint,
    radius: SchematicLength,
}
```

`SingleLine` is the only port valid for a circle station. It selects the
station centre and permits the line to use any one octilinear axis.

`Interchange` is the only port valid for a capsule station. Its nested port is
relative to the capsule's major axis:

- `MajorAxis` passes through the centre along the major axis;
- `RisingOblique` passes through the centre along the axis obtained by a
  45-degree counter-clockwise rotation from the major axis;
- `FallingOblique` passes through the centre along the axis obtained by a
  45-degree clockwise rotation from the major axis;
- `SinglePerpendicular` passes through the centre perpendicular to the major
  axis when the station has exactly one perpendicular line; and
- `PerpendicularAnchor { index }` passes through one indexed anchor
  perpendicular to the major axis when the station has multiple perpendicular
  lines.

The rotations are visual rotations in the SVG coordinate system. Because an
`OctilinearAxis` is undirected, rotating either of its two direction
representatives in the stated direction gives the same resulting axis.

`anchor_count` is the number of perpendicular lines through the capsule. Zero
means that no perpendicular port is valid. One uses `SinglePerpendicular` at
the centre and does not create an indexed port. A value greater than one uses
exactly the indexed ports `0..anchor_count`; `SinglePerpendicular`,
`RisingOblique`, and `FallingOblique` are then invalid. Excluding the oblique
centre ports avoids their conflict with multiple perpendicular lines.

A semantic corner is the intersection of the unrounded incoming and outgoing
legs. It requests one circular fillet with the configured radius. Arbitrary
Bézier control points, splines, and additional curve semantics are not
supported.

## Coordinates, axes, and lengths

The positive x-axis points to the right and the positive y-axis points down,
matching SVG. This is separate from the topology renderer's Cartesian
projection: schematic coordinates are authored in this SVG-aligned system and
are not reused from projected topology-renderer coordinates.

`SchematicPoint` is serialised as exactly `[x, y]`. Both coordinates are finite
`f64` values. `SchematicLength` is serialised as an `f64` scalar and accepts
only finite, strictly positive values.

```rust
struct SchematicPoint {
    x: f64,
    y: f64,
}

struct SchematicLength(f64);

enum OctilinearAxis {
    Horizontal,
    FallingDiagonal,
    Vertical,
    RisingDiagonal,
}
```

`SchematicLength` is a newtype so construction and deserialisation have one
enforcement point for its finite, positive invariant. `OctilinearAxis` is
undirected; algorithms derive travel direction internally where needed.

Capsule anchors use these deterministic increasing-index directions:

| Axis              | Increasing anchor-index direction |
| ----------------- | --------------------------------- |
| `Horizontal`      | west to east                      |
| `FallingDiagonal` | northwest to southeast            |
| `Vertical`        | north to south                    |
| `RisingDiagonal`  | southwest to northeast            |

For station centre $\vec{c}$, anchor count $k$, anchor interval $a$, and the
representative unit vector $\vec{e}$, anchor $i$ is derived as:

$$
\vec{p}_i = \vec{c} + \left(i - \frac{k - 1}{2}\right)a\vec{e},
\qquad 0 \le i < k.
$$

The formula is evaluated only when $k > 1$. When $k = 1$, the one
perpendicular line uses `SinglePerpendicular` at the capsule centre.
`anchor_interval` therefore affects geometry only when $k > 1$. The semantic
manifest does not store calculated capsule-anchor positions.

## Prepared schematic

The resolver lowers semantic objects into a private geometric model.

```rust
struct PreparedSchematic<'a> {
    line_width: f64,
    symbols: Vec<PreparedSymbol<'a>>,
    strokes: Vec<PreparedStroke<'a>>,
}

struct PreparedSymbol<'a> {
    station: &'a SchematicStation,
    center: Point,
    shape: PreparedShape,
    fill: &'a str,
    stroke: &'a str,
    stroke_width: f64,
    stroke_alignment: SchematicStrokeAlignment,
}

enum PreparedShape {
    Circle {
        diameter: f64,
    },
    Capsule {
        axis: OctilinearAxis,
        diameter: f64,
        length: f64,
    },
}
```

Each semantic station generates one prepared symbol. The retained station
reference supplies the stable station ID written to the SVG; it does not make
the prepared model an authoring contract.

The resolver also lowers each semantic line to a visual stroke. Stroke IDs are
the semantic line IDs and are written to the SVG.

```rust
struct PreparedStroke<'a> {
    id: &'a str,
    color: &'a str,
    paths: Vec<PreparedPath<'a>>,
}

struct PreparedPath<'a> {
    points: Vec<PreparedPathPoint<'a>>,
    closed: bool,
}

struct PreparedPathPoint<'a> {
    position: Point,
    kind: PreparedPointKind<'a>,
}

enum PreparedPointKind<'a> {
    Anchor {
        station: &'a str,
        permitted_axis: Option<OctilinearAxis>,
    },
    Corner {
        id: &'a str,
        radius: f64,
    },
}
```

An anchor point cannot change direction. If it is intermediate, its incoming
and outgoing legs must be collinear; when `permitted_axis` is set, both legs
must also use that axis. An endpoint has only one incident leg and that leg
must use the permitted axis when one is set.

A corner point is the only path point that can change direction. The resolver
therefore follows this generation rule:

- circle and capsule ports generate `PreparedPointKind::Anchor` values, which
  cannot bend;
- semantic corners generate `PreparedPointKind::Corner` values, which may
  bend.

There is deliberately no general `can_bend` field. Encoding bend capability in
the point-kind variant prevents a station-generated anchor from being marked
as bendable. A change of direction requires an explicit corner outside the
station symbol.

Semantic corner and station-port references are resolved directly to prepared
points without approximate coordinate comparison. They do not permit shared
route segments.

No two lines may use the same station port, and a corner may belong to only one
line. Geometric validation also rejects overlapping legs, including the same
unordered pair of path points appearing in different paths of one stroke.
Consequently, no pair of path points can carry multiple strokes.

## Resolution rules

The semantic-to-prepared resolver deterministically performs these steps:

1. Index and validate station, corner, and line identities and references.
2. Resolve every station visit and semantic corner to an inline prepared path
   point.
3. Lower every semantic line to one prepared stroke.
4. Resolve station styles and generate one prepared symbol for every station.
5. Validate the resulting path geometry and SVG bounds.

Repeated references to the same semantic station port resolve to the same
anchor, but only within one line.

A `SingleLine` port generates an anchor at the circle centre with no fixed
permitted axis. Interchange ports generate anchors with these positions and
axes:

- `MajorAxis` uses the capsule centre and major axis;
- `RisingOblique` uses the capsule centre and the axis 45 degrees
  counter-clockwise from the major axis;
- `FallingOblique` uses the capsule centre and the axis 45 degrees clockwise
  from the major axis;
- `SinglePerpendicular` uses the capsule centre and the axis perpendicular to
  the major axis; and
- `PerpendicularAnchor { index }` uses the derived indexed position and the
  axis perpendicular to the major axis.

Every perpendicular port implied by `anchor_count` must be referenced by
exactly one line.

## Global options

```rust
struct SchematicOptions {
    background: SchematicBackgroundOptions,
    lines: SchematicLineOptions,
    stations: SchematicStationOptions,
}
```

`background` contains either an opaque `color` or `transparent: true`.
`transparent: false` may accompany a colour and is omitted from canonical
output, while `transparent: true` conflicts with a colour. `color` also accepts
`colour` on input and is canonicalised to `color` on output.
`lines.width` is the width of every line stroke. `stations.common` and
`stations.interchange` independently define the fill and stroke of circle and
capsule symbols. A common fill has a `diameter`; an interchange fill has a
`width`, which is the capsule's minimum minor-axis width. Both resolved sizes
are at least `lines.width`. A capsule's major-axis length is:

$$
L_{\mathrm{capsule}} =
\begin{cases}
2d, & k \le 1, \\
d + (k - 1)a, & k > 1,
\end{cases}
$$

where $d$ is its resolved minor-axis diameter. For adjacent perpendicular
strokes not to overlap, the anchor interval must satisfy:

$$
a \ge w_{\mathrm{line}}.
$$

Every station stroke has a positive `width` and an `alignment` of `inside`,
`center`, or `outside`. The British spelling `centre` is accepted on input and
canonicalised to `center` on output. Common-station colours use an explicitly
tagged policy: `unified` supplies one `value`, while `follow-line` takes the
colour of the station's line. Interchange fill and stroke colours are fixed
colour strings. Every `color` field also accepts the British spelling `colour`
on input and canonicalises it to `color` on output.

## Serialisation contract

YAML is the primary human-editable format. JSON uses an equivalent schema for
Web interchange. Every structure rejects unknown fields, enum variants use an
explicit `type` tag, and field names use `snake_case`.

The semantic schematic manifest is the public authoring contract. Prepared
geometry is private, derived in memory, and is not a competing semantic source
of truth.

The library exposes the semantic types with `Schematic`-prefixed names.
`SchematicManifest::{from_yaml, to_yaml, from_json, to_json}` provide the
format boundary. Schematic points and lengths enforce their scalar invariants
during construction and deserialisation.

Documentation prose uses British English. Rust and schema identifiers use
North American English to follow their surrounding conventions; for example,
the code uses `SingleLine` and canonical YAML uses `single_line` and `color`.
British `centre` and `colour` spellings are accepted as input aliases.

The `Interchange` station-port variant contains a nested
`SchematicInterchangePort`, serialised under `interchange` as shown in the
example below. Both enum layers retain their explicit `type` tag. A station
visit therefore uses `port.interchange.type`, without a repeated `port.port`
path.

The existing localised-name rule is preserved: `names[locale][0]` is the
canonical name and later values are aliases. Every names list contains at least
one non-empty value.

This semantic fragment demonstrates an explicit corner between two stations:

```yaml
options:
  background:
    color: "#ffffff"
  lines:
    width: 8.0
  stations:
    common:
      fill:
        diameter: 18.0
        color:
          type: unified
          value: "#ffffff"
      stroke:
        width: 2.0
        alignment: center
        color:
          type: follow-line
    interchange:
      fill:
        width: 18.0
        color: "#ffffff"
      stroke:
        width: 2.0
        alignment: outside
        color: "#000000"

stations:
  - id: west
    position: [0.0, 40.0]
    names:
      en: [West]
    symbol:
      type: circle
  - id: central
    position: [80.0, 80.0]
    names:
      en: [Central]
    symbol:
      type: capsule
      axis: rising_diagonal
      anchor_count: 1
      anchor_interval: 24.0

corners:
  - id: west-corner
    position: [40.0, 40.0]
    radius: 8.0

lines:
  - id: line-a
    names:
      en: [Line A]
    color: "#e2231a"
    paths:
      - visits:
          - type: station
            station_id: west
            port:
              type: single_line
          - type: corner
            corner_id: west-corner
          - type: station
            station_id: central
            port:
              type: interchange
              interchange:
                type: single_perpendicular
        closed: false
```

Its private prepared path contains the resolved west anchor, corner, and
central perpendicular anchor positions in that order. There is deliberately
no corresponding geometric YAML or JSON document.

## Validation contract

Semantic deserialisation enforces the strict field and enum-tag schema.
Semantic resolution additionally rejects:

- empty or duplicate station, corner, and line IDs;
- empty locale keys or localised-name lists without a non-empty canonical
  value;
- unknown station and corner references;
- unreferenced semantic corners;
- non-finite positions and non-finite or non-positive lengths;
- a port incompatible with its station symbol;
- `SinglePerpendicular` when `anchor_count` is not one;
- `PerpendicularAnchor` when `anchor_count <= 1` or its index is outside
  `0..anchor_count`;
- perpendicular-port references that do not match `anchor_count`: none when it
  is zero, exactly `SinglePerpendicular` when it is one, and every indexed
  `PerpendicularAnchor` exactly once when it is greater than one;
- `RisingOblique` or `FallingOblique` when `anchor_count > 1`;
- a station port referenced by more than one line;
- a corner referenced by more than one line;
- a capsule with multiple indexed anchors whose `anchor_interval` is less than
  `line_width`;
- open semantic paths with fewer than two station visits or endpoints that are
  not station visits;
- closed semantic paths with fewer than three station visits; and
- a repeated station visit or corner reference within one path.

Prepared-geometry validation rejects:

- consecutive points at the same coordinate;
- non-octilinear legs;
- an intermediate anchor whose legs are not collinear;
- a leg at an anchor that violates its permitted axis;
- corners with collinear legs, 180-degree reversals, or radii that do not fit;
- overlapping legs, including a duplicate unordered path-point pair;
- geometry whose derived coordinates or SVG bounds are not finite.

`validate_schematic` and the renderer use the same preparation and bounds
validation pipeline before SVG path commands are derived. The renderer does
not maintain a second, weaker set of geometry rules.

## Current scope

Labels and label placement, legends, titles, line marks, arbitrary Bézier and
spline paths, independently authored capsule-anchor coordinates, and
per-element SVG styling are not part of the current manifest or renderer.
