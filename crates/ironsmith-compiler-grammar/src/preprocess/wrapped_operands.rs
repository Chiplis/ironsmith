//! Bounded physical wraps at an unfinished source-reference operand.
use super::*;

pub(super) fn completes_source_operand(
    previous: &str,
    current: &crate::front_end::CstLine,
    document: &str,
) -> bool {
    let Some(tokens) = stage_tokens(previous) else { return false; };
    if !tokens.last().is_some_and(|token| token.is_any_word(&["from", "to", "onto", "with", "under"])) {
        return false;
    }
    let Some(content) = current.content_span else { return false; };
    current.nodes.iter().any(|node| {
        let crate::front_end::CstNodeKind::SelfReference(surface) = node.kind else { return false; };
        if node.span.start.byte != content.start.byte { return false; }
        let Some(tail) = document.get(node.span.end.byte..content.end.byte) else { return false; };
        let tail = tail.trim_start();
        let tail = if matches!(surface, crate::front_end::SelfReferenceSurface::This) {
            let Some(tokens) = stage_tokens(tail) else { return false; };
            if let Some(noun) = tokens.first().filter(|token| token.is_any_word(&[
                "permanent", "creature", "artifact", "enchantment", "land", "planeswalker", "battle", "spell",
            ])) {
                tail.get(noun.span.end..).unwrap_or("").trim_start()
            } else { tail }
        } else { tail };
        // A complete source operand ends here. A following source-subject
        // instruction ("This creature has ...") does not complete it.
        tail.is_empty() || tail.starts_with('.')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rules(text: &str) -> (PreprocessedDocument, Vec<String>) {
        let card = CardBuilder::new(crate::ids::CardId::new(), "Unusual Hound");
        let document = preprocess_document(card, text).unwrap();
        let rows = document.items.iter().filter_map(|item| match item {
            PreprocessedItem::Line(line) => Some(line.info.raw_line.clone()), _ => None,
        }).collect();
        (document, rows)
    }
    #[test]
    fn joins_only_the_wrapped_source_operand_and_retains_raw_document() {
        for operand in ["Unusual Hound", "this creature", "this card"] {
            let input = format!("Type: Enchantment Creature — Dog\nI, II — Draw a card.\nIII — Remove all lore counters from \n{operand}. It deals 3 damage to up to one target creature.");
            let (document, rows) = rules(&input);
            assert_eq!(document.cst.reconstruct_source(), input);
            assert_eq!(rows.len(), 2, "{rows:?}");
            assert!(rows[1].contains(&format!("from {operand}.")), "{rows:?}");
            assert!(rows[0].starts_with("I, II"));
        }
    }
    #[test]
    fn retains_blank_chapter_and_independent_ability_boundaries() {
        for ending in [
            "\nUnusual Hound. It deals 3 damage to any target.",
            "IV — Unusual Hound. It deals 3 damage to any target.",
            "Unusual Hound has flying.",
            "Whenever Unusual Hound attacks, draw a card.",
        ] {
            let input = format!("Type: Creature — Dog\nIII — Remove all lore counters from\n{ending}");
            let (document, rows) = rules(&input);
            assert_eq!(document.cst.reconstruct_source(), input);
            assert_eq!(rows.len(), 2, "{ending}: {rows:?}");
            assert!(rows[0].ends_with("from"));
        }
    }
}
