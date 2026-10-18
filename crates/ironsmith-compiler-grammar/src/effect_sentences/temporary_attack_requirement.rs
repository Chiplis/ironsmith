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
    if let Some(effect) = parse_target_attacks_during_its_controllers_next_turn(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_during_that_players_next_turn_referenced_attack(tokens) {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_chosen_players_attack_each_other(tokens) {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_referenced_combat_requirement(tokens) {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_filtered_creatures_block_this_turn(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_only_typed_creatures_act_that_combat(tokens) {
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

/// "After this phase, there is an additional combat phase. Only land
/// creatures can attack during that combat phase." (Bumi, Unleashed): during
/// the combat phase the same instruction added, creatures without the named
/// card type can't attack (or block). Same rule start as the chosen-creatures
/// form (Last Night Together).
fn parse_only_typed_creatures_act_that_combat(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    use crate::effect::RestrictionStart;
    let body = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(body);
    let ["only", type_words @ .., "creatures", "can", action @ ("attack" | "block"), "during", "that", "combat", "phase"] =
        words.as_slice()
    else {
        return None;
    };
    let [type_word] = type_words else {
        return None;
    };
    let card_type = crate::util::parse_card_type(type_word)?;
    if card_type == crate::types::CardType::Creature {
        return None;
    }
    let others = crate::target::ObjectFilter::creature().without_type(card_type);
    let restriction = if *action == "attack" {
        Restriction::attack(others)
    } else {
        Restriction::block(others)
    };
    Some(EffectAst::subject_verb_cant_starting(
        restriction,
        Until::EndOfCombat,
        RestrictionStart::LastAddedCombatPhase,
        None,
    ))
}

/// "Creatures your opponents control block this turn if able" (Brutal
/// Hordechief): a resolving blocking requirement (CR 509.1c) over a
/// described creature set for the rest of this turn.
fn parse_filtered_creatures_block_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let body = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some((index, (), rest)) = crate::grammar::primitives::find_prefix(body, || {
        use winnow::Parser;
        winnow::combinator::alt((
            crate::grammar::primitives::phrase(&["block", "this", "turn", "if", "able"]),
            crate::grammar::primitives::phrase(&["blocks", "this", "turn", "if", "able"]),
        ))
        .void()
    }) else {
        return Ok(None);
    };
    if !crate::util::trim_edge_punctuation_tokens(rest).is_empty() || index == 0 {
        return Ok(None);
    }
    let subject = &body[..index];
    if subject.iter().any(|token| {
        token.is_any_word(&["target", "that", "those", "it", "they", "them", "chosen"])
    }) {
        return Ok(None);
    }
    let Some(filter) =
        crate::grammar::primitives::probe_shape(crate::object_filters::parse_object_filter(subject, false))
    else {
        return Ok(None);
    };
    if !filter.card_types.contains(&crate::types::CardType::Creature) {
        return Ok(None);
    }
    Ok(Some(EffectAst::subject_verb_cant(
        Restriction::must_block(filter),
        Until::EndOfTurn,
        None,
    )))
}

/// "Choose two target players. Until your next turn, each creature they
/// control attacks the other chosen player each combat if able." (The
/// Brothers' War): for each chosen (target) player, a requirement (CR 508.1d)
/// that the creatures that player controls attack the other chosen player.
/// The rule binds both players as it resolves; the attacking set stays live.
fn parse_chosen_players_attack_each_other(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    use crate::target::{ObjectFilter, PlayerFilter};
    let (duration, body) =
        if let Some(prefix) = crate::grammar::leaf::parse_leaf_turn_duration_prefix_tokens(tokens) {
            (prefix.duration, prefix.rest)
        } else {
            let suffix = crate::grammar::leaf::parse_leaf_turn_duration_suffix_tokens(tokens)?;
            (suffix.duration, suffix.rest)
        };
    let body = crate::util::trim_edge_punctuation_tokens(body);
    let words = crate::lexer::token_word_refs(body);
    if !matches!(
        words.as_slice(),
        [
            "each", "creature", "they", "control", "attacks", "the", "other", "chosen", "player",
            "each", "combat", "if", "able"
        ] | [
            "creatures", "they", "control", "attack", "the", "other", "chosen", "player", "each",
            "combat", "if", "able"
        ]
    ) {
        return None;
    }
    let duration = match duration {
        crate::grammar::leaf::LeafTurnDurationPhrase::ThisTurn
        | crate::grammar::leaf::LeafTurnDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
        crate::grammar::leaf::LeafTurnDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        crate::grammar::leaf::LeafTurnDurationPhrase::UntilYourNextTurnEnd => {
            Until::YourNextTurnEnd
        }
    };
    // The players were declared by the preceding "choose two target players".
    let chosen = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let attackers = ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer);
    let other = PlayerFilter::excluding(chosen.clone(), PlayerFilter::IteratedPlayer);
    Some(EffectAst::ForEach(
        crate::cards::builders::ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
            filter: chosen,
            effects: vec![EffectAst::subject_verb_cant(
                Restriction::must_attack_player(attackers, other),
                duration,
                None,
            )],
        },
    ))
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
    // "... attack Gideon Jura if able" (Gideon Jura) names a permanent they
    // must attack (CR 508.1d); its filter is read after the creature subject.
    let mut attacked_permanents = None;
    let (index, attacks_you) = if let Some((index, attacks_you, rest)) =
        primitives::find_prefix(body, || {
            (
                alt((primitives::kw("attack"), primitives::kw("attacks"))),
                winnow::combinator::opt(primitives::kw("you")),
                primitives::phrase(&["if", "able"]),
            )
                .map(|(_, you, ())| you.is_some())
        }) {
        if !crate::util::trim_edge_punctuation_tokens(rest).is_empty() {
            return Ok(None);
        }
        (index, attacks_you)
    } else {
        let Some(index) = body
            .iter()
            .position(|token| token.is_word("attack") || token.is_word("attacks"))
        else {
            return Ok(None);
        };
        let words = crate::lexer::token_word_refs(body);
        if !words.ends_with(&["if", "able"]) || body.len() < index + 4 {
            return Ok(None);
        }
        let permanent_tokens = &body[index + 1..body.len() - 2];
        if permanent_tokens.iter().any(|token| token.as_word().is_none()) {
            return Ok(None);
        }
        let Ok(permanents) = crate::object_filters::parse_object_filter(permanent_tokens, false)
        else {
            return Ok(None);
        };
        attacked_permanents = Some(permanents);
        (index, false)
    };
    if index == 0 {
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
                if let Some(permanents) = attacked_permanents {
                    Restriction::must_attack_permanents(filter, permanents)
                } else if attacks_you {
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

/// "Up to one target creature an opponent controls attacks Gideon during its
/// controller's next turn if able." (Gideon, Battle-Forged): a requirement
/// (CR 508.1d) on the target creature that begins with its controller's next
/// turn and lasts until that turn ends. Naming a permanent after "attacks"
/// requires it to attack that permanent; otherwise any defender satisfies it.
/// The creature is this ability's target; its controller is read as the
/// ability resolves.
fn parse_target_attacks_during_its_controllers_next_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::RestrictionStart;
    use crate::target::PlayerFilter;

    let body = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(during) = body.iter().rposition(|token| token.is_word("during")) else {
        return Ok(None);
    };
    let tail = crate::lexer::parser_token_word_refs(&body[during + 1..]);
    if !matches!(
        tail.as_slice(),
        ["its", "controllers" | "controller's", "next", "turn", "if", "able"]
            | ["its", "controller", "s", "next", "turn", "if", "able"]
    ) {
        return Ok(None);
    }
    let head = &body[..during];
    let Some(verb) = head
        .iter()
        .position(|token| token.is_word("attack") || token.is_word("attacks"))
    else {
        return Ok(None);
    };
    let subject = &head[..verb];
    if subject.is_empty()
        || !crate::activation_and_restrictions::starts_with_target_indicator(subject)
    {
        return Ok(None);
    }
    let target = crate::util::parse_target_phrase(subject)?;
    let Some(mut attackers) =
        crate::activation_and_restrictions::target_ast_to_object_filter(target.clone())
    else {
        return Ok(None);
    };
    if !attackers
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        return Ok(None);
    }
    crate::activation_and_restrictions::activation_restriction_clauses::ensure_it_tagged_constraint(
        &mut attackers,
    );
    let permanent_tokens = &head[verb + 1..];
    let restriction = if permanent_tokens.is_empty() {
        Restriction::must_attack(attackers)
    } else {
        if permanent_tokens.iter().any(|token| token.as_word().is_none()) {
            return Ok(None);
        }
        let Ok(permanents) = crate::object_filters::parse_object_filter(permanent_tokens, false)
        else {
            return Ok(None);
        };
        Restriction::must_attack_permanents(attackers, permanents)
    };
    Ok(Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::subject_verb_target_only(target),
            EffectAst::subject_verb_cant_starting(
                restriction,
                Until::EndOfTurn,
                RestrictionStart::NextTurn(PlayerFilter::ControllerOf(
                    crate::filter::ObjectRef::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind().into(),
                    ),
                )),
                None,
            ),
        ],
    }))
}

/// "During that player's next turn, the chosen creatures attack if able, and
/// other creatures can't attack." (Oracle en-Vec): over that player's next
/// turn, a requirement (CR 508.1d) that exactly the referenced creatures
/// attack, plus a restriction (CR 508.1c) that every other creature can't.
/// Both freeze the referenced set as the ability resolves. "That player" is
/// the discourse player (`IteratedPlayer`, rebound by lowering outside an
/// iteration), as in the next-turn attack bans.
fn parse_during_that_players_next_turn_referenced_attack(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    use crate::effect::RestrictionStart;
    use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};

    let body = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::parser_token_word_refs(body);
    let rest = words
        .strip_prefix(&["during", "that", "players", "next", "turn"][..])
        .or_else(|| words.strip_prefix(&["during", "that", "player's", "next", "turn"][..]))
        .or_else(|| words.strip_prefix(&["during", "that", "player", "s", "next", "turn"][..]))
        .or_else(|| words.strip_prefix(&["during", "their", "next", "turn"][..]))?;
    let (subject, after_requirement) = [
        &["the", "chosen", "creatures"][..],
        &["those", "creatures"][..],
        &["each", "of", "those", "creatures"][..],
        &["each", "of", "the", "chosen", "creatures"][..],
    ]
    .into_iter()
    .find_map(|subject| {
        let after = rest.strip_prefix(subject)?;
        let after = after
            .strip_prefix(&["attack", "if", "able"][..])
            .or_else(|| after.strip_prefix(&["attacks", "if", "able"][..]))?;
        Some((subject, after))
    })?;
    let _ = subject;
    let others_cant_attack = match after_requirement {
        [] => false,
        ["and", "other", "creatures", "cant" | "can't" | "cannot", "attack"] => true,
        _ => return None,
    };
    let referenced = |relation| {
        ObjectFilter::creature()
            .match_tagged(crate::tag::CompilerReferenceTag::It.bind(), relation)
    };
    let mut effects = vec![EffectAst::subject_verb_cant_starting(
        Restriction::must_attack(referenced(TaggedOpbjectRelation::IsTaggedObject)),
        Until::EndOfTurn,
        RestrictionStart::NextTurn(PlayerFilter::IteratedPlayer),
        None,
    )];
    if others_cant_attack {
        effects.push(EffectAst::subject_verb_cant_starting(
            Restriction::attack(referenced(TaggedOpbjectRelation::IsNotTaggedObject)),
            Until::EndOfTurn,
            RestrictionStart::NextTurn(PlayerFilter::IteratedPlayer),
            None,
        ));
    }
    Some(EffectAst::Sequence { effects })
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
    MustAttackThisTurn,
}

/// A subject naming the creatures the preceding instruction affected:
/// "Each of those creatures", "Those creatures", "They" (Illusionist's
/// Gambit, Berserker's Frenzy).
fn is_referenced_creatures_subject(subject: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(subject);
    matches!(
        words.as_slice(),
        ["they"]
            | ["it"]
            | ["the", "creature"]
            | ["that", "creature"]
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
pub(super) fn parse_referenced_combat_requirement(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
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
            alt((
                primitives::phrase(&["attack", "this", "turn", "if", "able"]),
                primitives::phrase(&["attacks", "this", "turn", "if", "able"]),
            ))
            .value(ReferencedCombatRule::MustAttackThisTurn),
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
        ReferencedCombatRule::MustAttackThisTurn => EffectAst::subject_verb_cant(
            Restriction::must_attack(creatures),
            Until::EndOfTurn,
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
