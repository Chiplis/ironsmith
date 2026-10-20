use super::*;
use crate::cards::builders::ForEachEffectAst;

pub(super) fn opponent_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Opponent => Some(PlayerFilter::Opponent),
        ForEachParticipantScope::OpponentExceptDefending => Some(PlayerFilter::excluding(
            PlayerFilter::Opponent,
            PlayerFilter::Defending,
        )),
        ForEachParticipantScope::OpponentOfThatPlayer
        | ForEachParticipantScope::OpponentOfItsController
        | ForEachParticipantScope::Player
        | ForEachParticipantScope::PlayerExceptYou
        | ForEachParticipantScope::PlayerExceptTarget
        | ForEachParticipantScope::PlayerExceptItsController
        | ForEachParticipantScope::PlayerOnYourTeam
        | ForEachParticipantScope::Teammate => None,
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
        ForEachParticipantScope::Teammate => Some(PlayerFilter::Teammate),
        // "that player" is the player the surrounding clause already names.
        ForEachParticipantScope::OpponentOfThatPlayer => Some(PlayerFilter::OpponentOf(Box::new(
            PlayerFilter::IteratedPlayer,
        ))),
        ForEachParticipantScope::OpponentOfItsController => Some(PlayerFilter::OpponentOf(
            Box::new(PlayerFilter::ControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            ))),
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
        PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
            crate::tag::CompilerReferenceTag::Triggering.bind(),
        )),
    )
}

pub(super) fn wrap_players(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Any {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
    } else {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
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
    // "Any number of target players may each search their library ..."
    // (Turtle Tracks): the option word sits between the participant list and
    // `each`; it belongs to each participant's instruction.
    let leading_may = shape
        .target_tokens
        .last()
        .is_some_and(|token| token.is_word("may"));
    let target_tokens = if leading_may {
        &shape.target_tokens[..shape.target_tokens.len() - 1]
    } else {
        shape.target_tokens
    };
    let Ok(target) = parse_target_phrase(target_tokens) else {
        return Ok(None);
    };
    // A non-player target slice means the `each` belongs to a later phrase
    // ("discards all cards of each of the sacrificed creature's colors"),
    // not to a target-player participant list: decline so the ordinary action
    // family reads the complete clause.
    let filter = match target {
        TargetAst::Player(filter, _) => filter,
        TargetAst::WithCount(inner, _) => match *inner {
            TargetAst::Player(filter, _) => filter,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    // The participant after `each` is the actor of the trailing instruction.
    // Supplying that subject before parsing also lets possessive dynamic
    // values such as "half their library" bind to the iterated player rather
    // than falling back to the spell's controller.
    let effects = if leading_may {
        let mut with_may = Vec::with_capacity(shape.effect_tokens.len() + 1);
        with_may.push(OwnedLexToken::word(
            "may".to_string(),
            TextSpan::synthetic(),
        ));
        with_may.extend_from_slice(shape.effect_tokens);
        parse_maybe_effects(&with_may, true, true)?
    } else if for_each_shapes::contains_may(shape.effect_tokens) {
        parse_maybe_effects(shape.effect_tokens, true, true)?
    } else {
        let normalized = prepend_that_player_subject(shape.effect_tokens);
        parse_maybe_effects(&normalized, true, false)?
    };
    Ok(Some(EffectAst::ForEach(
        ForEachEffectAst::ForEachTargetPlayers {
            count: shape.count,
            filter,
            effects,
        },
    )))
}

use crate::recognition::ParseOutcome;
#[path = "participant_scopes/for_each_player_readings.rs"]
mod for_each_player_readings;

pub fn parse_for_each_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if has_independent_participant_continuation(tokens) {
        return Ok(None);
    }
    let Some(outer) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    let Some(iteration_filter) = player_filter(outer.scope) else {
        return Ok(None);
    };
    // "Each player with fewer cards in hand than ... draws" / "each player
    // with exactly 13 life ...": a `with` qualifier restricts the iterated
    // players. The participant readings below would read it as the body's
    // subject and silently drop it; the qualified-player clause owns it.
    if outer
        .inner_tokens
        .first()
        .is_some_and(|token| token.is_word("with"))
    {
        return Ok(None);
    }
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
        ParseOutcome::Match(matched) => {
            return Ok(Some(if outer.participant_is_actor {
                matched.value.value
            } else {
                sequential_participant_body(matched.value.value)
            }));
        }
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    // "Each player creates X Treasure tokens ..., where X is the number of
    // descent counters on this enchantment." (Descent into Avernus): the
    // value definition scopes the whole participant body.
    let (inner_tokens, body_where_x) =
        match find_word_phrase(outer.inner_tokens, &["where", "x", "is"]).and_then(|where_idx| {
            let value = parse_participant_body_where_x_value(outer.inner_tokens)?;
            Some((
                crate::lexer::trim_lexed_commas(&outer.inner_tokens[..where_idx]).to_vec(),
                value,
            ))
        }) {
            Some((body, value)) if !body.is_empty() => (body, Some(value)),
            _ => (outer.inner_tokens.to_vec(), None),
        };
    let inner_tokens = inner_tokens.as_slice();
    let mut effects = if outer.participant_is_actor && !participant_may {
        if let Some(effects) = parse_quantified_participant_actor_program(inner_tokens)? {
            effects
        } else {
            let normalized = prepend_that_player_subject(inner_tokens);
            parse_maybe_effects(&normalized, true, true)?
        }
    } else {
        let normalized = prepend_that_player_life_total_subject(inner_tokens);
        parse_maybe_effects(&normalized, true, outer.participant_is_actor)?
    };
    if let Some(value) = body_where_x {
        replace_unbound_x_in_effects_anywhere(&mut effects, &value, &clause_text)?;
    }
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
    exclude_participant_choice_from_other_discard(&mut effects, &mut None);
    let iteration_filter = reanchor_other_player_copy_filter(iteration_filter, &effects);
    Ok(Some(if outer.participant_is_actor {
        wrap_players(&iteration_filter, effects)
    } else {
        sequential_participant_body(wrap_players(&iteration_filter, effects))
    }))
}

/// "Each player reveals their hand, chooses <cards> from it, then discards
/// all other nonland cards" (Noxious Vapors): within one participant's body,
/// "other" in a discard after a choice is relative to the chosen cards, not
/// to the source. Keep the source exclusion and also exclude the chosen set
/// by identity.
fn exclude_participant_choice_from_other_discard(
    effects: &mut [EffectAst],
    chosen: &mut Option<crate::cards::builders::TagKey>,
) {
    fn exclude(filter: &mut crate::target::ObjectFilter, chosen: &crate::cards::builders::TagKey) {
        if !filter.other
            || filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *chosen
                    && constraint.relation == crate::target::TaggedOpbjectRelation::IsNotTaggedObject
            })
        {
            return;
        }
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: chosen.clone(),
                relation: crate::target::TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }
    for effect in effects {
        match effect {
            EffectAst::Sequence { effects }
            | EffectAst::CommaThen { effects }
            | EffectAst::SourceSentence { effects, .. } => {
                exclude_participant_choice_from_other_discard(effects, chosen);
            }
            // "reveals their hand, chooses ..., then discards ..." is one
            // coordinated participant program; walk its members in order.
            EffectAst::Coordination(coordination) => {
                for member in &mut coordination.members {
                    exclude_participant_choice_from_other_discard(&mut member.effects, chosen);
                }
            }
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }) => {
                *chosen = Some(tag.key.clone());
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ZoneMoves(
                        crate::cards::builders::ZoneMoveActionAst::Discard { count, filter, .. },
                    ),
                ..
            }) => {
                let Some(chosen) = chosen.as_ref() else {
                    continue;
                };
                if let Some(filter) = filter.as_mut() {
                    exclude(filter, chosen);
                }
                if let crate::effect::Value::Count(counted) = count {
                    exclude(counted, chosen);
                }
            }
            _ => {}
        }
    }
}
