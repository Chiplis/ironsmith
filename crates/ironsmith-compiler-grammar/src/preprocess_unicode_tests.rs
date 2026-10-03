use super::*;
use crate::diagnostics::TextSpan;
use crate::ids::CardId;
use ironsmith_core::card::CardBuilder;

fn original_slice<'a>(line: &'a NormalizedLine, needle: &str) -> &'a str {
    let start = line.normalized.find(needle).expect("normalized phrase");
    let end = start + needle.len();
    let span = crate::util::map_span_to_original(
        TextSpan {
            line: 0,
            start,
            end,
        },
        &line.normalized,
        &line.original,
        &line.char_map,
    );
    let original = line
        .original
        .get(span.start..span.end)
        .expect("UTF-8 source span");
    assert_eq!(Some(original), line.source_slice(start..end));
    original
}

#[test]
fn unchanged_unicode_tokens_keep_character_maps_and_authored_case() {
    let raw = "Éowyn, Café’s Guardian deals 2 damage to any target.";
    let line = normalize_line_for_parse_text(raw, "", "", true).unwrap();
    assert_eq!(line.normalized, raw.to_ascii_lowercase());
    assert_eq!(line.char_map, (0..raw.chars().count()).collect::<Vec<_>>());
    for phrase in ["Éowyn", "café’s", "guardian", "deals", "target"] {
        let authored = original_slice(&line, phrase);
        assert!(authored.eq_ignore_ascii_case(phrase));
    }
    assert_eq!(original_slice(&line, "café’s guardian"), "Café’s Guardian");
}

#[test]
fn unicode_self_references_retain_full_names_and_following_token_offsets() {
    for (name, raw_name) in [
        ("altaïr", "Altaïr"),
        ("Éowyn, fearless knight", "Éowyn, Fearless Knight"),
        (
            "asmoranomardicadaistinaculdacar",
            "Asmoranomardicadaistinaculdacar",
        ),
    ] {
        let raw = format!("Whenever {raw_name} attacks, draw a card.");
        let line = normalize_line_for_parse_text(&raw, name, name, false).unwrap();
        assert_eq!(line.normalized, "whenever this attacks, draw a card.");
        assert_eq!(original_slice(&line, "this"), raw_name);
        assert_eq!(original_slice(&line, "attacks"), "attacks");
        assert_eq!(original_slice(&line, "card."), "card.");
        assert!(
            line.char_map
                .iter()
                .all(|index| *index < raw.chars().count())
        );
    }
}

#[test]
fn terminal_unicode_self_reference_keeps_its_last_source_character() {
    let line = normalize_line_for_parse_text("Untap Altaïr", "altaïr", "altaïr", false).unwrap();
    assert_eq!(line.normalized, "untap this");
    assert_eq!(original_slice(&line, "this"), "Altaïr");
    let end = line.normalized.len();
    let span = crate::util::map_span_to_original(
        TextSpan {
            line: 0,
            start: end,
            end,
        },
        &line.normalized,
        &line.original,
        &line.char_map,
    );
    assert_eq!(span.start, line.original.len());
    assert_eq!(span.end, span.start);
}

#[test]
fn wrapped_activation_unicode_source_map_stays_in_original_coordinates() {
    let line =
        normalize_line_for_parse_text("({T}: Untap Altaïr.)", "altaïr", "altaïr", false).unwrap();
    assert_eq!(line.normalized, "{t}: untap this.");
    assert_eq!(original_slice(&line, "this"), "Altaïr");
    assert_eq!(original_slice(&line, "untap"), "Untap");
}

#[test]
fn altair_oracle_preprocessing_preserves_unicode_source_spans() {
    // Frozen Oracle text for 43dc4b0c-e437-4abc-a3fb-48bd3098de88. This test
    // covers preprocessing, not support for the later memory-counter program.
    let raw = "First strike\nWhenever Altaïr attacks, exile up to one target Assassin creature card from your graveyard with a memory counter on it. Then for each creature card you own in exile with a memory counter on it, create a tapped and attacking token that's a copy of it. Exile those tokens at end of combat.";
    let document = preprocess_document(CardBuilder::new(CardId::new(), "Altaïr Ibn-La'Ahad"), raw)
        .expect("Altaïr must preprocess without a UTF-8 slicing panic");
    let mut saw_source = false;
    for item in &document.items {
        let PreprocessedItem::Line(line) = item else {
            continue;
        };
        let normalized = &line.info.normalized;
        for token in &line.tokens {
            let span = crate::util::map_span_to_original(
                token.span,
                &normalized.normalized,
                &normalized.original,
                &normalized.char_map,
            );
            let authored = normalized
                .original
                .get(span.start..span.end)
                .expect("every token maps to a valid source byte span");
            assert_eq!(
                Some(authored),
                normalized.source_slice(token.span.start..token.span.end)
            );
            if token.slice == "this" {
                assert_eq!(authored, "Altaïr");
                saw_source = true;
            } else {
                assert!(
                    authored.eq_ignore_ascii_case(&token.slice),
                    "{} -> {authored}",
                    token.slice
                );
            }
        }
    }
    assert!(
        saw_source,
        "the short Unicode self-reference must be recognized"
    );
}

#[test]
fn unicode_resolution_tail_keeps_terminal_punctuation_at_its_source() {
    let raw = "Return Éowyn as it resolves.";
    let line = normalize_line_for_parse_text(raw, "", "", true).unwrap();
    assert_eq!(line.normalized, "return Éowyn.");
    let start = line.normalized.find("Éowyn").unwrap();
    let span = crate::util::map_span_to_original(
        TextSpan {
            line: 0,
            start,
            end: start + "Éowyn".len(),
        },
        &line.normalized,
        &line.original,
        &line.char_map,
    );
    assert_eq!(&line.original[span.start..span.end], "Éowyn");
    assert_eq!(original_slice(&line, "."), ".");
}
