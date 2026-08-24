use super::*;

pub fn parse_assigns_no_combat_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<AssignsNoCombatDamageShape<'_>> {
    let (subject_tokens, tail_tokens) = primitives::split_lexed_once_on_separator(tokens, || {
        alt((primitives::kw("assign"), primitives::kw("assigns"))).void()
    })?;
    primitives::parse_prefix(tail_tokens, primitives::phrase(&["no", "combat", "damage"]))?;
    let duration = primitives::parse_all(
        tail_tokens,
        (
            primitives::phrase(&["no", "combat", "damage"]),
            opt(alt((
                primitives::phrase(&["this", "turn"]).value(Until::EndOfTurn),
                primitives::phrase(&["this", "combat"]).value(Until::EndOfCombat),
            ))),
            primitives::sentence_end(),
        )
            .map(|(_, duration, _)| duration.unwrap_or(Until::EndOfTurn)),
        "assigns-no-combat-damage duration",
    );
    let Ok(duration) = duration else {
        return Some(AssignsNoCombatDamageShape::Unsupported);
    };
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let subject_words = crate::lexer::parser_token_word_refs(subject_tokens);
    let source = if subject_tokens.is_empty()
        || crate::word_primitives::parse_any_sequence_complete(
            &subject_words,
            &[&["this"], &["this", "creature"]],
        )
        || exact(
            subject_tokens,
            primitives::any_phrase(&[&["this"], &["this", "creature"]]).void(),
        ) {
        AssignDamageSourceShape::Source
    } else if exact(subject_tokens, primitives::kw("it").void()) {
        AssignDamageSourceShape::Tagged
    } else {
        AssignDamageSourceShape::Target(subject_tokens)
    };
    Some(AssignsNoCombatDamageShape::Supported { source, duration })
}
