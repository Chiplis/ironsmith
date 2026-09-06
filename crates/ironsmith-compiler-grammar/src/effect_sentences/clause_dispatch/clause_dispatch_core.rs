use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::ZoneMoveActionAst;
use crate::cards::builders::LibraryActionAst;
use super::*;

use crate::recognition::ParseOutcome;
#[path = "clause_dispatch_core/clause_readings.rs"]
mod clause_readings;

pub(super) fn parse_effect_clause_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }
    let stripped_instead = super::super::strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);
    let tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    if let Some(shape) = crate::grammar::effects::parse_shuffle_object_shape_lexed(tokens)
        && shape.owner_subject_target_tokens.is_some()
        && let Some(effects) = super::super::search_library::parse_shuffle_object_into_library_sentence(tokens)?
    {
        return Ok(EffectAst::Sequence { effects });
    }
    let input = clause_readings::Clause {
        tokens,
        read_by_cache: Default::default(),
    };
    match clause_readings::read_clause(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => clause_readings::diagnose(&input)?,
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let (verb, _) = find_verb(tokens).ok_or_else(|| {
        let clause = render_lower_words(tokens);
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "convert",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "unattach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "detain",
            "goad",
            "suspect",
            "end",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    let verb_shape = clause_grammar::parse_clause_subject_verb_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "could not split subject and verb in effect clause (clause: '{}')",
            render_lower_words(tokens)
        ))
    })?;
    let subject_tokens_storage = trim_commas(verb_shape.subject_tokens);
    let subject_tokens = subject_tokens_storage.as_slice();
    let rest = verb_shape.action_tokens;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));
    // The verb names the clause family; each family's typed shape reads before
    // the general verb dispatch below.
    match verb {
        Verb::Counter => {
            if let Some(entry) = tokens.windows(2).position(|pair| pair[0].is_word("enters") && pair[1].is_word("with"))
                && crate::util::is_source_reference_words(&crate::lexer::token_word_refs(&tokens[..entry]))
            {
                return parse_put_counters(&tokens[entry + 2..]);
            }
            if !subject_tokens.is_empty()
                && !tokens.first().is_some_and(|token| token.is_any_word(&["if", "unless", "when", "whenever"]))
                && contains_token_word(tokens, "on")
                && let Ok(effect) = parse_put_counters(tokens)
            {
                parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
                return Ok(effect);
            }
        }
        Verb::Get => {
            if let Some(effect) = parse_get_pump_clause(subject_tokens, rest, tokens)? {
                return Ok(effect);
            }
        }
        Verb::Sacrifice => {
            if let Some((subject, target)) =
                parse_controller_or_owner_of_target_subject(subject_tokens)
            {
                return parse_sacrifice(rest, Some(subject), Some(target));
            }
        }
        Verb::Put => {
            if let Some((SubjectAst::Player(PlayerAst::ItsOwner), target)) =
                parse_controller_or_owner_of_target_subject(subject_tokens)
                && is_pronoun_top_or_bottom_library_choice_put_tail(rest)
            {
                return Ok(EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { target }),
                ));
            }
        }
        _ => {}
    }
    let subject_word_view = ClauseDispatchCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}') [rule=combat-history-player-subject]",
            render_lower_words(tokens)
        )));
    }
    if matches!(verb, Verb::Gain)
        && !subject_tokens.is_empty()
        && let Some(shape) = clause_grammar::parse_protection_choice_shape(rest)
    {
        let target = parse_target_phrase(subject_tokens)?;
        return Ok(EffectAst::subject_verb_grant_protection_choice(
            target,
            match shape.chooser {
                clause_grammar::ProtectionChoiceChooserShape::You => PlayerAst::You,
                clause_grammar::ProtectionChoiceChooserShape::TargetController => {
                    PlayerAst::ItsController
                }
            },
            shape.includes_colorless,
            shape.includes_artifacts,
            shape.chooses_card_type,
        ));
    }
    if matches!(verb, Verb::Gain)
        && let Some(effects) =
            super::super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(EffectAst::Sequence { effects });
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Gain) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let parsed_actions = parse_ability_line(tail.ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(tail.ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !tail.ability_tokens.is_empty()
            && tail.trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_grant_abilities_to_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Lose) && clause_grammar::parse_shared_ability_gain_shape(rest).is_some()
    {
        let target = match clause_grammar::parse_reference_subject_shape(subject_tokens) {
            clause_grammar::ReferenceSubjectShape::Source => {
                TargetAst::Source(span_from_tokens(subject_tokens))
            }
            clause_grammar::ReferenceSubjectShape::Tagged => TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(subject_tokens),
            ),
            clause_grammar::ReferenceSubjectShape::Other => parse_target_phrase(subject_tokens)?,
        };
        return Ok(EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            Until::EndOfTurn,
        ));
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let ability_tokens = trim_edge_punctuation(tail.ability_tokens);
        let trailing_tokens = trim_edge_punctuation(tail.trailing_tokens);
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Deal)
        && let Some(effect) = parse_explicit_target_object_damage_source(subject_tokens, rest)?
    {
        return Ok(effect);
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let subject_words = crate::lexer::parser_token_word_refs(subject_tokens);
    let each_other_player = crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[&["each"], &["other"], &["player", "players"]],
    );
    let another_target_player = crate::word_primitives::parse_sequence_complete(
        &subject_words,
        &["another", "target", "player"],
    );
    let optional_target_player = if crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[
            &["up"],
            &["to"],
            &["one"],
            &["target"],
            &["player", "players"],
        ],
    ) {
        Some(TargetAst::WithCount(
            Box::new(TargetAst::Player(
                PlayerFilter::Any,
                span_from_tokens(subject_tokens),
            )),
            ChoiceCount::up_to(1),
        ))
    } else if crate::word_primitives::parse_sequence_prefix(&subject_words, &["up", "to"]) {
        let target = parse_target_phrase(subject_tokens)?;
        let is_optional_player = matches!(
            &target,
            TargetAst::WithCount(inner, count)
                if matches!(inner.as_ref(), TargetAst::Player(_, _))
                    && count.min == 0
                    && count.max == Some(1)
        );
        is_optional_player.then_some(target)
    } else {
        None
    };
    if matches!(verb, Verb::Return)
        && clause_grammar::is_return_tagged_reference_shape(subject_tokens)
    {
        let mut return_tokens = subject_tokens.to_vec();
        return_tokens.extend(rest.iter().cloned());
        return parse_effect_with_verb(verb, Some(SubjectAst::This), &return_tokens);
    }
    if matches!(verb, Verb::Put)
        && clause_grammar::is_exiled_cards_to_hand_shape(subject_tokens, rest)
    {
        let filter = parse_object_filter(subject_tokens, false)?;
        return Ok(EffectAst::subject_verb_return_all_to_hand(filter));
    }
    let relative_player_subject = if matches!(verb, Verb::Gain)
        && rest.first().is_some_and(|token| token.is_word("control"))
        && subject_tokens
            .first()
            .is_some_and(|token| token.is_word(TARGET_WORD))
    {
        match parse_target_phrase(subject_tokens) {
            Ok(target) => match &target {
                TargetAst::Player(filter, _)
                    if !matches!(filter, PlayerFilter::Any | PlayerFilter::Opponent) =>
                {
                    Some(target)
                }
                _ => None,
            },
            Err(_) => None,
        }
    } else {
        None
    };
    let mut effect = if let Some(target) = optional_target_player {
        let action = parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), action],
        }
    } else if another_target_player {
        let target = TargetAst::Player(
            PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::target_player()),
            span_from_tokens(subject_tokens),
        );
        let action = parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), action],
        }
    } else if let Some(target) = relative_player_subject {
        let source_relative_target = target_player_mentions_source_object(&target);
        let mut gain_control =
            parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        if source_relative_target {
            bind_gain_control_pronoun_to_source(&mut gain_control);
        }
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), gain_control],
        }
    } else if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = if each_other_player {
            SubjectAst::Player(PlayerAst::That)
        } else {
            parse_subject(subject_tokens)
        };
        if let Some(clause) = CommonPlayerActionClause::recognize(subject, verb, rest) {
            clause.lower()?
        } else {
            parse_effect_with_verb(verb, Some(subject), rest)?
        }
    };
    let authored_control_pronoun = {
        let rest_words = ClauseDispatchCompatWords::new(rest).to_word_refs();
        crate::word_primitives::sequence_occurs(&rest_words, &["they", "control"])
    };
    if matches!(verb, Verb::Return)
        && (crate::word_primitives::parse_sequence_complete(&subject_words, &["they"])
            || authored_control_pronoun)
        && let EffectAst::SubjectVerb(subject_verb) = &mut effect
        && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. }) = &mut subject_verb.action
    {
        fn mark_iterated_actor_pronoun(target: &mut TargetAst) {
            match target {
                TargetAst::Object(filter, ..) => {
                    filter.set_iterated_actor_pronoun_surface(true);
                }
                TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
                    mark_iterated_actor_pronoun(inner);
                }
                _ => {}
            }
        }
        mark_iterated_actor_pronoun(target);
    }
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        });
    }
    if each_other_player {
        effect = EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::NotYou,
            effects: vec![effect],
        });
    }
    Ok(effect)
}

pub(super) fn parse_passive_goad_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_passive_goad_shape(tokens) else {
        return Ok(None);
    };
    let target = match shape.target {
        clause_grammar::GoadTargetShape::TaggedToken => TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(tokens),
        ),
        clause_grammar::GoadTargetShape::Target(target_tokens) => {
            parse_target_phrase(target_tokens)?
        }
    };
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    let duration = if shape.for_rest_of_game {
        Until::Forever
    } else {
        Until::YourNextTurn
    };
    Ok(Some(EffectAst::subject_verb_goad_for(target, duration)))
}

pub fn parse_effect_clause_lexed(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_effect_clause(tokens)
}
