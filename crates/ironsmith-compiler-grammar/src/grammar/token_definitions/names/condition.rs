use super::*;

pub(in super::super) fn artifact_surface_name(words: &[&str], named: Option<&str>) -> String {
    if let Some(named) = named {
        return named.to_string();
    }
    for word in words {
        if !matches!(
            *word,
            "artifact"
                | "token"
                | "tokens"
                | "named"
                | "colorless"
                | "white"
                | "blue"
                | "black"
                | "red"
                | "green"
        ) {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                let mut name = first.to_uppercase().to_string();
                name.push_str(chars.as_str());
                return name;
            }
        }
    }
    "Artifact".to_string()
}
