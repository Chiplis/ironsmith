/// One-based lines declaring optional parser callbacks in already masked Rust source.
pub(super) fn registry_option_protocol_lines(masked: &str) -> Vec<usize> {
    let lines = masked.lines().collect::<Vec<_>>();
    let mut findings = Vec::new();
    let mut index = 0usize;
    let mut registry_type_depth = None::<usize>;
    let mut brace_depth = 0usize;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if (trimmed.contains("enum ") || trimmed.contains("struct "))
            && ["Rule", "Registry", "Recognizer", "Handler"]
                .iter()
                .any(|marker| trimmed.contains(marker))
            && trimmed.contains('{')
        {
            registry_type_depth = Some(brace_depth + line.matches('{').count());
        }

        let type_alias = trimmed.contains("type ")
            && trimmed.contains('=')
            && [
                "Rule",
                "Registry",
                "Recognizer",
                "Handler",
                "Parser",
                "ParseFn",
            ]
            .iter()
            .any(|marker| trimmed.contains(marker));
        let stored_function_pointer = registry_type_depth.is_some() && trimmed.contains("fn(");
        if type_alias || stored_function_pointer {
            let start = index;
            let mut declaration = String::new();
            loop {
                declaration.push_str(lines[index]);
                let complete = if type_alias {
                    lines[index].contains(';')
                } else {
                    lines[index].contains(',') || lines[index].contains(';')
                };
                if complete || index + 1 == lines.len() {
                    break;
                }
                index += 1;
            }
            let compact = declaration
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            if compact.contains("Result<Option<") || compact.contains("->Option<") {
                findings.push(start + 1);
            }
        }

        brace_depth += line.matches('{').count();
        brace_depth = brace_depth.saturating_sub(line.matches('}').count());
        if registry_type_depth.is_some_and(|depth| brace_depth < depth) {
            registry_type_depth = None;
        }
        index += 1;
    }

    findings
}
