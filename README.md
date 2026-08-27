# Metro Draw

`mtrd` works with metro topology manifests. YAML is the primary human-editable
format, while JSON uses the same schema for WebUI interchange.

## Commands

```console
# Convert in either direction. File extensions select the formats.
mtrd convert map.yaml map.json
mtrd convert map.json map.yaml

# Check syntax, schema, and topology renderability.
mtrd check map.yaml

# Print the parsed map as canonical YAML, or as detailed Rust debug output.
mtrd check -v map.yaml
mtrd check -vv map.yaml

# Render the topology graph as SVG. These flag forms are equivalent.
mtrd render -to map.svg map.yaml
mtrd render -o map.svg -t map.yaml

# Without -o or -T, append .svg to the input path.
mtrd render --topology map.yaml

# Use -T to write ./mtrd-<microsecond timestamp>.svg.
mtrd render -tT map.yaml
```

`check` accepts `.yaml`, `.yml`, and `.json` inputs. A successful check means
the manifest can be processed by the topology renderer. It validates that:

- station and line IDs are non-empty and unique;
- every station referenced by a line path exists;
- both coordinates of every station are finite and within the renderer's
  supported numeric range;
- an open path contains at least two stations;
- a closed path contains at least three stations; and
- a station occurs at most once in a single path.

These checks concern the manifest itself. Rendering can still fail because of
an output filesystem error, such as an unwritable destination.

With `check -v`, `mtrd` prints the parsed map as canonical YAML after it passes
validation. With `check -vv`, it prints the detailed Rust debug representation.
