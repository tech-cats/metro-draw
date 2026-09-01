/// `serde_yaml` emits all sequences in block style. Manifest positions use flow
/// style to keep human-edited YAML compact.
pub(crate) fn inline_yaml_positions(yaml: String) -> String {
    let lines: Vec<_> = yaml.lines().collect();
    let mut output = String::with_capacity(yaml.len());
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if line.trim() == "position:"
            && let (Some(x), Some(y)) = (lines.get(index + 1), lines.get(index + 2))
            && let (Some(x), Some(y)) = (x.trim().strip_prefix("- "), y.trim().strip_prefix("- "))
        {
            let indentation = &line[..line.len() - line.trim_start().len()];
            output.push_str(indentation);
            output.push_str("position: [");
            output.push_str(x);
            output.push_str(", ");
            output.push_str(y);
            output.push_str("]\n");
            index += 3;
            continue;
        }

        output.push_str(line);
        output.push('\n');
        index += 1;
    }

    output
}
