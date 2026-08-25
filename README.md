# Metro Draw

`mtrd` works with metro map manifests. YAML is the primary human-editable
format; JSON is supported for WebUI interchange.

```console
# Convert in either direction. File extensions select the formats.
mtrd convert map.yaml map.json
mtrd convert map.json map.yaml

# Check syntax and schema.
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
