use super::*;
use crate::text_cleanup::strip_parenthetical_text;

/// Render the normalized oracle surface used for storage and exports.
///
/// This surface is intentionally independent from the compiled AST renderer:
/// it is the card's oracle text with reminder text stripped and only minimal
/// stable formatting applied.
pub fn normalized_oracle_lines(def: &CardDefinition) -> Vec<String> {
    strip_parenthetical_text(&def.card.oracle_text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(normalize_canonical_oracle_line)
        .collect()
}

pub fn canonical_compiled_lines(def: &CardDefinition) -> Vec<String> {
    normalized_oracle_lines(def)
}

fn normalize_canonical_oracle_line(line: &str) -> String {
    line.replace(
        "At the beginning of each player's end step,",
        "At the beginning of each end step,",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardId;
    use crate::cards::CardDefinitionBuilder;

    #[test]
    fn normalized_oracle_lines_strip_reminder_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Reminder Test")
            .oracle_text("Flying (This creature can't be blocked except by creatures with flying or reach.)\nDraw a card.")
            .build();

        assert_eq!(
            normalized_oracle_lines(&def),
            vec!["Flying", "Draw a card."]
        );
    }

    #[test]
    fn normalized_oracle_lines_do_not_depend_on_compiled_ast() {
        let mut def = CardDefinitionBuilder::new(CardId::new(), "Oracle Only")
            .parse_text("Draw a card.")
            .expect("test definition should parse");
        def.card.oracle_text = "Destroy target creature.".to_string();

        assert_eq!(
            normalized_oracle_lines(&def),
            vec!["Destroy target creature."]
        );
    }
}
