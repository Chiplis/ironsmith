use super::super::ir::{
    DelayedScheduleSurface, DocumentSemanticFacts, OverloadRewritePayload, PostpassRepairFacts,
};
use super::super::lexer::{OwnedLexToken, lex_line};
use super::primitives;
use winnow::combinator::opt;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverloadKeywordLine;

fn has_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn has_any_phrase(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases.iter().any(|phrase| has_phrase(tokens, phrase))
}

fn parse_overload_keyword_tokens(tokens: &[OwnedLexToken]) -> Option<OverloadKeywordLine> {
    primitives::parse_prefix(&tokens, primitives::kw("overload"))?;
    Some(OverloadKeywordLine)
}

fn parse_delayed_schedule_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DelayedScheduleSurface> {
    let start_next_turn =
        if has_any_phrase(tokens, &[&["next", "upkeep"], &["next", "turns", "upkeep"]])
            || has_any_phrase(
                tokens,
                &[
                    &["that", "turns", "end", "step"],
                    &["that", "players", "next", "upkeep"],
                    &["that", "players", "next", "end", "step"],
                    &["end", "step", "of", "that", "players", "next", "turn"],
                ],
            )
        {
            true
        } else if has_any_phrase(
            tokens,
            &[&["next", "end", "step"], &["next", "turns", "end", "step"]],
        ) {
            false
        } else {
            return None;
        };

    Some(DelayedScheduleSurface {
        start_next_turn,
        your_next_upkeep: has_phrase(tokens, &["your", "next", "upkeep"]),
        your_next_draw_step: has_phrase(tokens, &["your", "next", "draw", "step"]),
    })
}

fn clash_additional_buff_and_trample<'a>(
    input: &mut super::super::lexer::LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["if", "you", "win"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "that",
        "creature",
        "gets",
        "an",
        "additional",
        "+2/+2",
        "and",
        "gains",
        "trample",
    ])
    .parse_next(input)
}

fn parses_clash_additional_buff_and_trample(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || clash_additional_buff_and_trample).is_some()
}

fn parse_postpass_repair_facts(tokens: &[OwnedLexToken]) -> PostpassRepairFacts {
    PostpassRepairFacts {
        opponents_lose_life_one_or_more: has_phrase(
            &tokens,
            &["one", "or", "more", "opponents", "lose", "life"],
        ),
        clash_additional_buff_and_trample: parses_clash_additional_buff_and_trample(tokens),
        shroud_while_source_tapped: has_phrase(
            &tokens,
            &[
                "has", "shroud", "for", "as", "long", "as", "this", "creature", "remains", "tapped",
            ],
        ),
        target_creature_blocks_target_creature: has_phrase(
            &tokens,
            &[
                "target", "creature", "blocks", "target", "creature", "this", "turn", "if", "able",
            ],
        ),
        defending_creature_blocks_source: has_phrase(
            &tokens,
            &[
                "target",
                "creature",
                "defending",
                "player",
                "controls",
                "blocks",
                "it",
                "this",
                "combat",
                "if",
                "able",
            ],
        ),
        chosen_nonbasic_land_type_becomes_copy: has_phrase(
            &tokens,
            &["choose", "a", "nonbasic", "land", "type"],
        ) && has_phrase(
            &tokens,
            &[
                "each", "land", "you", "control", "of", "that", "type", "becomes", "a", "copy",
                "of", "target", "creature", "you", "control",
            ],
        ),
    }
}

fn parses_kicked_counter_spell_mana_value_replacement(tokens: &[OwnedLexToken]) -> bool {
    has_phrase(tokens, &["counter", "target", "spell"])
        && has_phrase(tokens, &["mana", "value"])
        && has_phrase(tokens, &["2", "or", "less"])
        && has_phrase(tokens, &["if", "this", "spell", "was", "kicked"])
        && has_phrase(tokens, &["counter", "that", "spell"])
        && has_phrase(tokens, &["4", "or", "less"])
        && has_phrase(tokens, &["instead"])
}

/// Recognizes every source-dependent fact required after lowering while the
/// front end still owns the lexed document. The overload payload is a typed
/// rewrite request that the front end consumes to build a semantic overload
/// branch before preparation and lowering.
pub(crate) fn parse_document_semantic_facts(text: &str) -> DocumentSemanticFacts {
    let mut document_tokens = Vec::new();
    let mut delayed_schedule_surfaces = Vec::new();
    let mut overload_keyword_line_index = None;
    let mut overload_target_spans = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let tokens = lex_line(line.trim(), index).unwrap_or_default();
        document_tokens.extend(tokens.iter().cloned());

        if parse_overload_keyword_tokens(&tokens).is_some() {
            overload_keyword_line_index.get_or_insert(index);
        } else {
            overload_target_spans.extend(
                tokens
                    .iter()
                    .filter(|token| token.is_word("target"))
                    .map(|token| token.span),
            );
        }

        if let Some(surface) = parse_delayed_schedule_surface_tokens(&tokens) {
            delayed_schedule_surfaces.push(surface);
        }
    }

    DocumentSemanticFacts {
        overload_rewrite: overload_keyword_line_index.map(|keyword_line_index| {
            OverloadRewritePayload {
                keyword_line_index,
                target_spans: overload_target_spans,
            }
        }),
        delayed_schedule_surfaces,
        kicked_counter_spell_mana_value_replacement:
            parses_kicked_counter_spell_mana_value_replacement(&document_tokens),
        postpass_repairs: parse_postpass_repair_facts(&document_tokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_delayed_schedule_and_player_facts() {
        let parsed =
            parse_document_semantic_facts("At the beginning of your next upkeep, draw a card.")
                .delayed_schedule_surfaces;
        assert_eq!(
            parsed,
            vec![DelayedScheduleSurface {
                start_next_turn: true,
                your_next_upkeep: true,
                your_next_draw_step: false,
            }]
        );
    }

    #[test]
    fn parses_generic_repair_facts() {
        let facts = parse_document_semantic_facts(
            "Target creature blocks target creature this turn if able.",
        )
        .postpass_repairs;
        assert!(facts.target_creature_blocks_target_creature);
    }

    #[test]
    fn parses_clash_win_buff_repair_fact() {
        let facts = parse_document_semantic_facts(
            "Target creature gets +2/+2 until end of turn. Clash with an opponent. If you win, that creature gets an additional +2/+2 and gains trample until end of turn.",
        )
        .postpass_repairs;
        assert!(facts.clash_additional_buff_and_trample);
    }

    #[test]
    fn builds_typed_overload_rewrite_payload() {
        let facts = parse_document_semantic_facts(
            "Return target creature to its owner's hand.\nOverload {1}{U}",
        );
        let payload = facts
            .overload_rewrite
            .expect("overload should request a rewrite");
        assert_eq!(payload.keyword_line_index, 1);
        assert_eq!(payload.target_spans.len(), 1);
        assert_eq!(payload.target_spans[0].line, 0);
    }

    #[test]
    fn recognizes_kicked_counter_spell_replacement_fact() {
        let facts = parse_document_semantic_facts(
            "Kicker {2}\nCounter target spell if its mana value is 2 or less. If this spell was kicked, counter that spell if its mana value is 4 or less instead.",
        );
        assert!(facts.kicked_counter_spell_mana_value_replacement);
    }

    #[test]
    fn rejects_nearby_kicked_counter_spell_replacement_surface() {
        let facts = parse_document_semantic_facts(
            "Kicker {2}\nCounter target spell if its mana value is 2 or less. If this spell was kicked, counter that spell if its mana value is 5 or less instead.",
        );
        assert!(!facts.kicked_counter_spell_mana_value_replacement);
    }
}
