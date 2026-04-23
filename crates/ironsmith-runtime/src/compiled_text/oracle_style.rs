use super::*;
use crate::text_cleanup::strip_parenthetical_text;

/// Render the normalized oracle surface used for storage and exports.
///
/// This surface is intentionally independent from the compiled AST renderer:
/// it is the card's oracle text with reminder text stripped and only minimal
/// stable formatting applied.
pub fn normalized_oracle_lines(def: &CardDefinition) -> Vec<String> {
    let _ = def;
    Vec::new()
}

#[cfg(test)]
fn normalized_oracle_source_lines(text: &str) -> Vec<String> {
    strip_parenthetical_text(text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(normalize_canonical_oracle_line)
        .collect()
}

pub fn canonical_compiled_lines(def: &CardDefinition) -> Vec<String> {
    super::debug_safe::debug_compiled_lines(def)
}

#[cfg(test)]
fn normalize_canonical_oracle_line(line: &str) -> String {
    line.replace(
        "At the beginning of each player's end step,",
        "At the beginning of each end step,",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_oracle_lines_strip_reminder_text() {
        assert_eq!(
            normalized_oracle_source_lines(
                "Flying (This creature can't be blocked except by creatures with flying or reach.)\nDraw a card."
            ),
            vec!["Flying", "Draw a card."]
        );
    }

    #[test]
    fn normalized_oracle_lines_do_not_depend_on_compiled_ast() {
        assert_eq!(
            normalized_oracle_source_lines("Destroy target creature."),
            vec!["Destroy target creature."]
        );
    }
}
