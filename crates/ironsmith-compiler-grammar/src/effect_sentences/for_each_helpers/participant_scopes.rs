use crate::cards::builders::ForEachEffectAst;
use super::*;

pub(super) fn opponent_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Opponent => Some(PlayerFilter::Opponent),
        ForEachParticipantScope::OpponentExceptDefending => Some(PlayerFilter::excluding(
            PlayerFilter::Opponent,
            PlayerFilter::Defending,
        )),
        ForEachParticipantScope::Player
        | ForEachParticipantScope::PlayerExceptYou
        | ForEachParticipantScope::PlayerExceptTarget
        | ForEachParticipantScope::PlayerExceptItsController
        | ForEachParticipantScope::PlayerOnYourTeam => None,
    }
}

pub(super) fn player_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Player => Some(PlayerFilter::Any),
        ForEachParticipantScope::PlayerExceptYou => Some(PlayerFilter::NotYou),
        ForEachParticipantScope::PlayerExceptTarget => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::target_player(),
        )),
        ForEachParticipantScope::PlayerExceptItsController => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::ControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            )),
        )),
        ForEachParticipantScope::PlayerOnYourTeam => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::Opponent,
        )),
        ForEachParticipantScope::Opponent | ForEachParticipantScope::OpponentExceptDefending => {
            None
        }
    }
}

/// In `each other player may copy that spell`, "other" is relative to the
/// player who controls the referenced spell, not necessarily the ability's
/// controller. Keep ordinary `each other player` clauses controller-relative,
/// but anchor this typed stack-copy shape to the triggering stack object.
pub(super) fn reanchor_other_player_copy_filter(
    filter: PlayerFilter,
    effects: &[EffectAst],
) -> PlayerFilter {
    if filter != PlayerFilter::NotYou || !effects.iter().any(effect_copies_triggering_stack_object)
    {
        return filter;
    }
    PlayerFilter::excluding(
        PlayerFilter::Any,
        PlayerFilter::AliasedControllerOf(ObjectRef::tagged(crate::tag::CompilerReferenceTag::Triggering.bind())),
    )
}

pub(super) fn wrap_players(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Any {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
    } else {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        })
    }
}

pub fn parse_for_each_target_players_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_players_shape(tokens) else {
        return Ok(None);
    };
    if shape.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after target-player each clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }
    // `target player <action> ... for each <counted set>` contains the same
    // lexical markers as `N target players <qualifier> each <action>`. The
    // shape parser intentionally keeps the qualifier open-ended, so require
    // that its proposed target slice is actually a target phrase before
    // claiming the clause. Otherwise the ordinary action family (for example,
    // discard) must receive the complete `for each` count suffix.
    let Ok(target) = parse_target_phrase(shape.target_tokens) else {
        return Ok(None);
    };
    let filter = match target {
        TargetAst::Player(filter, _) => filter,
        TargetAst::WithCount(inner, _) => match *inner {
            TargetAst::Player(filter, _) => filter,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "expected player target in target-player each clause (clause: '{}')",
                    LexedClause::new(tokens).text()
                )));
            }
        },
        _ => {
            return Err(CardTextError::ParseError(format!(
                "expected player target in target-player each clause (clause: '{}')",
                LexedClause::new(tokens).text()
            )));
        }
    };
    // The participant after `each` is the actor of the trailing instruction.
    // Supplying that subject before parsing also lets possessive dynamic
    // values such as "half their library" bind to the iterated player rather
    // than falling back to the spell's controller.
    let effects = if for_each_shapes::contains_may(shape.effect_tokens) {
        parse_maybe_effects(shape.effect_tokens, true, true)?
    } else {
        let normalized = prepend_that_player_subject(shape.effect_tokens);
        parse_maybe_effects(&normalized, true, false)?
    };
    Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers {
        count: shape.count,
        filter,
        effects,
    })))
}

use crate::recognition::ParseOutcome;
#[path = "participant_scopes/for_each_player_readings.rs"]
mod for_each_player_readings;

pub fn parse_for_each_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if has_independent_participant_continuation(tokens) { return Ok(None); }
    let Some(outer) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    let Some(iteration_filter) = player_filter(outer.scope) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let slot_chooser = if outer.participant_is_actor {
        PlayerAst::That
    } else {
        PlayerAst::You
    };
    let input = for_each_player_readings::ParticipantClause {
        tokens,
        outer: &outer,
        iteration_filter: iteration_filter.clone(),
        clause_text: &clause_text,
        slot_chooser,
    };
    match for_each_player_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let mut effects = if outer.participant_is_actor && !participant_may {
        if let Some(effects) = parse_quantified_participant_actor_program(outer.inner_tokens)? {
            effects
        } else {
            let normalized = prepend_that_player_subject(outer.inner_tokens);
            parse_maybe_effects(&normalized, true, true)?
        }
    } else {
        let normalized = prepend_that_player_life_total_subject(outer.inner_tokens);
        parse_maybe_effects(&normalized, true, outer.participant_is_actor)?
    };
    if !outer.participant_is_actor {
        force_implicit_token_controller_you(&mut effects);
    }
    if participant_chooses {
        if outer.participant_is_actor
            && !outer.inner_tokens.iter().any(|token| token.is_word("you"))
        {
            bind_quantified_participant_actor(&mut effects);
        }
        bind_implicit_choose_chooser(
            &mut effects,
            if outer.participant_is_actor {
                PlayerAst::That
            } else {
                PlayerAst::You
            },
        );
        stabilize_standalone_participant_choice_tag(&mut effects, outer.inner_tokens);
    }
    let iteration_filter = reanchor_other_player_copy_filter(iteration_filter, &effects);
    Ok(Some(wrap_players(&iteration_filter, effects)))
}
