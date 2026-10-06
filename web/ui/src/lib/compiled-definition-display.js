// Filter named fields in Rust's pretty Debug output, preserving enum payloads
// and list entries (including meaningful values such as Some(0)).
export function hideEmptyDefinitionFields(rawCompilation = "") {
  return rawCompilation
    .split("\n")
    .filter((line) => !/^\s*[a-zA-Z_][a-zA-Z_0-9]*:\s*(?:None|false|\[\]|\{\}|""|0(?:\.0+)?),?\s*$/.test(line))
    .join("\n");
}
