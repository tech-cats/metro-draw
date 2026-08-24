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
```
