//! Resolving, explicitly timed combat rules do not grant creature abilities.
use crate::cards::builders::{CardTextError, EffectAst};
use crate::effect::{Restriction, Until};
use crate::lexer::{OwnedLexToken, TokenKind};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    if tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Quote | TokenKind::Colon))
    {
        return Ok(None);
    }
    if let Some(effect) = parse_during_target_players_next_turn(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_referenced_combat_requirement(tokens) {
        return Ok(Some(effect));
    }
    let (duration, body) = if let Some(prefix) =
        crate::grammar::leaf::parse_leaf_turn_duration_prefix_tokens(tokens)
    {
        (prefix.duration, prefix.rest)
    } else if let Some(suffix) =
        crate::grammar::leaf::parse_leaf_turn_duration_suffix_tokens(tokens)
    {
        (suffix.duration, suffix.rest)
    } else {
        return Ok(None);
    };
    let body = crate::util::trim_edge_punctuation_tokens(body);
    let Some((index, (), rest)) = crate::grammar::primitives::find_prefix(body, || {
        use winnow::Parser;
        winnow::combinator::alt((
            crate::grammar::primitives::phrase(&["attack", "each", "combat", "if", "able"]),
            crate::grammar::primitives::phrase(&["attacks", "each", "combat", "if", "able"]),
        ))
        .void()
    }) else {
        return Ok(None);
    };
    if !crate::util::trim_edge_punctuation_tokens(rest).is_empty() || index == 0 {
        return Ok(None);
    }
    let subject = &body[..index];
    if subject
        .iter()
        .any(|token| token.is_any_word(&["target", "that", "it", "chosen"]))
    {
        return Ok(None);
    }
    let filter = crate::object_filters::parse_object_filter(subject, false)?;
    if !filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        return Ok(None);
    }
    let duration = match duration {
        crate::grammar::leaf::LeafTurnDurationPhrase::ThisTurn
        | crate::grammar::leaf::LeafTurnDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
        crate::grammar::leaf::LeafTurnDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        crate::grammar::leaf::LeafTurnDurationPhrase::UntilYourNextTurnEnd => {
            Until::YourNextTurnEnd
        }
    };
    Ok(Some(EffectAst::subject_verb_cant(
        Restriction::must_attack(filter),
        duration,
        None,
    )))
}

/// "During target player's next turn, each creature that player controls
/// attacks if able." (Rowan Kenrith): a combat requirement (CR 508.1d) over
/// the target player's creatures that begins with that player's next turn and
/// lasts until its end. The player is declared as this ability's target; the
/// requirement's controlled set stays live during that turn.
fn parse_during_target_players_next_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::cards::builders::TargetAst;
    use crate::effect::RestrictionStart;
    use crate::grammar::primitives;
    use crate::target::PlayerFilter;
    use winnow::Parser;
    use winnow::combinator::alt;

    let Some(((), after_during)) = primitives::parse_prefix(tokens, primitives::kw("during").void())
    else {
        return Ok(None);
    };
    let Some((player, after_player)) = primitives::parse_prefix(
        after_during,
        (
            primitives::kw("target"),
            alt((
                primitives::any_phrase(&[&["players"], &["player's"], &["player", "s"]])
                    .value(PlayerFilter::Any),
                primitives::any_phrase(&[&["opponents"], &["opponent's"], &["opponent", "s"]])
                    .value(PlayerFilter::Opponent),
            )),
            primitives::phrase(&["next", "turn"]),
        )
            .map(|(_, player, ())| player),
    ) else {
        return Ok(None);
    };
    let target_tokens = &after_during[..after_during.len() - after_player.len()];
    let body = crate::util::trim_edge_punctuation_tokens(after_player);
    // "... attack you if able" (Taunt) names the player they must attack
    // (CR 508.1d); without it any defender satisfies the requirement.
    let Some((index, attacks_you, rest)) = primitives::find_prefix(body, || {
        (
            alt((primitives::kw("attack"), primitives::kw("attacks"))),
            winnow::combinator::opt(primitives::kw("you")),
            primitives::phrase(&["if", "able"]),
        )
            .map(|(_, you, ())| you.is_some())
    }) else {
        return Ok(None);
    };
    if index == 0 || !crate::util::trim_edge_punctuation_tokens(rest).is_empty() {
        return Ok(None);
    }
    let mut subject = &body[..index];
    if let Some((_, rest)) = primitives::parse_prefix(
        subject,
        alt((primitives::kw("each"), primitives::kw("all"))),
    ) {
        subject = rest;
    }
    if subject
        .iter()
        .any(|token| token.is_any_word(&["target", "chosen", "it", "they"]))
    {
        return Ok(None);
    }
    let mut filter = crate::object_filters::parse_object_filter(subject, false)?;
    if !filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        return Ok(None);
    }
    // "that player controls" names the player this sentence targets.
    if filter.controller != Some(PlayerFilter::IteratedPlayer) {
        return Ok(None);
    }
    let targeted_player = PlayerFilter::Target(Box::new(player.clone()));
    filter.controller = Some(targeted_player.clone());
    Ok(Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::subject_verb_target_only(TargetAst::Player(
                player,
                crate::util::span_from_tokens(target_tokens),
            )),
            EffectAst::subject_verb_cant_starting(
                if attacks_you {
                    Restriction::must_attack_player(filter, PlayerFilter::You)
                } else {
                    Restriction::must_attack(filter)
                },
                Until::EndOfTurn,
                RestrictionStart::NextTurn(targeted_player),
                None,
            ),
        ],
    }))
}

/// The combat rule a referenced-creature sentence imposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferencedCombatRule {
    /// "... attack(s) that combat if able" (CR 508.1d), in the combat phase
    /// the preceding instruction added (CR 500.8).
    MustAttackThatCombat,
    /// "... can't attack you or planeswalkers you control that combat"
    /// (CR 508.1c), in that added combat phase.
    CantAttackYouThatCombat,
    /// "... block(s) this turn if able" (CR 509.1c).
    MustBlockThisTurn,
}

/// A subject naming the creatures the preceding instruction affected:
/// "Each of those creatures", "Those creatures", "They" (Illusionist's
/// Gambit, Berserker's Frenzy).
fn is_referenced_creatures_subject(subject: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(subject);
    matches!(
        words.as_slice(),
        ["they"]
            | ["those", "creatures"]
            | ["each", "of", "those", "creatures"]
            | ["each", "of", "them"]
            | ["the", "chosen", "creatures"]
    )
}

/// "Each of those creatures attacks that combat if able.", "They can't attack
/// you or planeswalkers you control that combat." and "They block this turn
/// if able.": a combat requirement or restriction over exactly the creatures
/// the preceding instruction named.
fn parse_referenced_combat_requirement(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    use crate::effect::RestrictionStart;
    use crate::grammar::primitives;
    use winnow::Parser;
    use winnow::combinator::alt;

    let body = crate::util::trim_edge_punctuation_tokens(tokens);
    let (index, rule, rest) = primitives::find_prefix(body, || {
        alt((
            alt((
                primitives::phrase(&["attack", "that", "combat", "if", "able"]),
                primitives::phrase(&["attacks", "that", "combat", "if", "able"]),
            ))
            .value(ReferencedCombatRule::MustAttackThatCombat),
            alt((
                primitives::phrase(&[
                    "can't", "attack", "you", "or", "planeswalkers", "you", "control", "that",
                    "combat",
                ]),
                primitives::phrase(&[
                    "cant", "attack", "you", "or", "planeswalkers", "you", "control", "that",
                    "combat",
                ]),
            ))
            .value(ReferencedCombatRule::CantAttackYouThatCombat),
            alt((
                primitives::phrase(&["block", "this", "turn", "if", "able"]),
                primitives::phrase(&["blocks", "this", "turn", "if", "able"]),
            ))
            .value(ReferencedCombatRule::MustBlockThisTurn),
        ))
    })?;
    if index == 0 || !crate::util::trim_edge_punctuation_tokens(rest).is_empty() {
        return None;
    }
    if !is_referenced_creatures_subject(&body[..index]) {
        return None;
    }
    let creatures = crate::target::ObjectFilter::creature().match_tagged(
        crate::tag::CompilerReferenceTag::It.bind(),
        crate::target::TaggedOpbjectRelation::IsTaggedObject,
    );
    Some(match rule {
        ReferencedCombatRule::MustAttackThatCombat => EffectAst::subject_verb_cant_starting(
            Restriction::must_attack(creatures),
            Until::EndOfCombat,
            RestrictionStart::LastAddedCombatPhase,
            None,
        ),
        ReferencedCombatRule::CantAttackYouThatCombat => EffectAst::subject_verb_cant_starting(
            Restriction::attack_player_or_planeswalkers_controlled_by(
                creatures,
                crate::target::PlayerFilter::You,
            ),
            Until::EndOfCombat,
            RestrictionStart::LastAddedCombatPhase,
            None,
        ),
        ReferencedCombatRule::MustBlockThisTurn => EffectAst::subject_verb_cant(
            Restriction::must_block(creatures),
            Until::EndOfTurn,
            None,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_temporary_rule_is_distinct_from_an_untimed_or_quoted_ability() {
        for text in [
            "Until your next turn, creatures your opponents control attack each combat if able.",
            "Creatures your opponents control attack each combat if able until your next turn.",
        ] {
            let effect = parse(&crate::lexer::lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap();
            let debug = format!("{effect:?}");
            assert!(debug.contains("MustAttack"));
            assert!(debug.contains("YourNextTurn"));
            assert!(!debug.contains("GrantAbilities"));
        }
        for text in [
            "Creatures your opponents control attack each combat if able.",
            "Until your next turn, creatures your opponents control have \"This creature attacks each combat if able.\"",
            "Until your next turn, creatures your opponents control attack each combat if able and can't block.",
            "Until your next turn, target creature attacks each combat if able.",
        ] {
            assert!(
                parse(&crate::lexer::lex_line(text, 0).unwrap())
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
    }
}
