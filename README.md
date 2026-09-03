# MTRD: MeTRo Draw

`mtrd` works with metro topology and schematic manifests. YAML is the primary
human-editable format, while JSON uses the same schemas for WebUI interchange.

## Commands

```console
# Print the installed version. These flag forms are equivalent.
mtrd --version
mtrd -V

# Convert in either direction. File extensions select the formats.
mtrd convert map.yaml map.json
mtrd convert map.json map.yaml

# Check topology syntax, schema, and renderability.
mtrd check -t topology.yaml

# Check a schematic manifest's syntax and schema.
mtrd check -s schematic.yaml

# The long manifest-kind flags and verbosity modes are also supported.
mtrd check --topology -v topology.yaml
mtrd check --schematic -vv schematic.yaml

# Render the topology graph as SVG. These flag forms are equivalent.
mtrd render -to map.svg map.yaml
mtrd render -o map.svg -t map.yaml

# Render a schematic manifest as an SVG schematic map.
mtrd render -s schematic.yaml
mtrd render --schematic -o schematic.svg schematic.json

# Without -o or -T, append .svg to the input path.
mtrd render --topology map.yaml

# Use -T to write ./mtrd-<microsecond timestamp>.svg.
mtrd render -tT map.yaml
```

`check` requires exactly one of `-t`/`--topology` or `-s`/`--schematic` and
accepts `.yaml`, `.yml`, and `.json` inputs. For a topology manifest, a
successful check means the manifest can be processed by the topology renderer.
It validates that:

- station and line IDs are non-empty and unique;
- every station referenced by a line path exists;
- both coordinates of every station are finite and within the renderer's
  supported numeric range;
- an open path contains at least two stations;
- a closed path contains at least three stations; and
- a station occurs at most once in a single path.

These checks concern the manifest itself. Rendering can still fail because of
an output filesystem error, such as an unwritable destination.

For a schematic manifest, a successful check means it satisfies the strict
`SchematicManifest` YAML/JSON schema and every semantic and geometric
invariant required by the renderer. This includes valid IDs and references,
station-port compatibility, complete interchange anchors, octilinear legs,
explicit and feasible corners, exclusive path geometry, finite render bounds,
finite positions, finite positive lengths, and rejection of unknown fields.

With `check -v`, `mtrd` prints the parsed map as canonical YAML after it passes
validation. With `check -vv`, it prints the detailed Rust debug representation.

## Schematic manifest library API

The library also defines the semantic schematic-map schema documented in
[`docs/schematic-map/v4.md`](docs/schematic-map/v4.md). `SchematicManifest`
supports strict YAML and equivalent JSON serialisation through `from_yaml`,
`to_yaml`, `from_json`, and `to_json`. Schematic positions are `[x, y]` arrays,
lengths are finite positive scalars, and unknown fields are rejected.

Global `options` group line styling under `lines` and station styling under
`stations.common` and `stations.interchange`. Common-station colours support
the tagged `unified` and `follow-line` policies. Station stroke alignment is
`inside`, `center`, or `outside`; `centre` is accepted on input and serialised
canonically as `center`. Likewise, every `color` field in topology and
schematic manifests also accepts `colour` on input and is serialised
canonically as `color`.

`render_schematic_svg` validates, resolves, and renders this schema in the
library. The `mtrd render -s` CLI is a thin file adapter around that API and
uses the same output naming options as topology rendering. It draws line
strokes, rounded explicit corners, circles, and oriented interchange capsules;
labels, legends, titles, and line marks remain deferred.
[`examples/schematic.yaml`](examples/schematic.yaml) is a representative
semantic manifest.
