//! The pair kinds that are not fixed shapes: each reads its opening statement
//! and the sentence completing it into a [`Pair`].

use super::*;
use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::DelayedEffectAst;

pub(super) fn open_copy_for_each_target(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if is_each_copy_targets_different(next)
        && let Some(effect) =
            parse_copy_for_each_target_sentence(sentences, sentence_idx, sentence.lowered())?
    {
        return Ok(Some(Pair::CopyForEachTarget(effect)));
    }
    Ok(None)
}

pub(super) fn open_flashback_grant(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    // A replacement continuation owns the whole procedure, including its
    // diagnostics. Do not let this shorter reading reclaim a malformed one.
    if open_flashback_price_replacement(sentences, sentence_idx)?.is_some() {
        return Ok(None);
    }
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if let Some(shape) =
        sequence_grammar::parse_flashback_grant_shape(sentence.lowered(), next.lowered())
    {
        let collective = shape
            .target_tokens
            .first()
            .is_some_and(|t| t.is_word("each") || t.is_word("all"));
        let target_tokens = if collective {
            &shape.target_tokens[1..]
        } else {
            shape.target_tokens
        };
        let grantable = crate::model::CompilerGrantableCore::flashback_from_cards_mana_cost();
        let effect = if collective {
            let filter = crate::object_filters::parse_object_filter(target_tokens, false)?;
            EffectAst::ForEach(crate::cards::builders::ForEachEffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::subject_verb_grant_to_target(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    grantable,
                    crate::grant::GrantDuration::UntilEndOfTurn,
                )],
            })
        } else {
            EffectAst::subject_verb_grant_to_target(
                crate::effect_sentences::parse_target_phrase(target_tokens)?,
                grantable,
                crate::grant::GrantDuration::UntilEndOfTurn,
            )
        };
        return Ok(Some(Pair::FlashbackGrant(effect)));
    }
    if let Some(shape) =
        sequence_grammar::parse_escape_grant_shape(sentence.lowered(), next.lowered())
    {
        let grantable = crate::model::CompilerGrantableCore::DerivedAlternativeCast(
            ironsmith_core::DerivedAlternativeCast::escape_from_cards_mana_cost(
                shape.exile_count,
            ),
        );
        let effect = EffectAst::subject_verb_grant_to_target(
            crate::effect_sentences::parse_target_phrase(shape.target_tokens)?,
            grantable,
            crate::grant::GrantDuration::UntilEndOfTurn,
        );
        return Ok(Some(Pair::FlashbackGrant(effect)));
    }
    Ok(None)
}

pub(super) fn open_chosen_creature_type(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if choose_subtype_sentence(sentence)
        && let Some(effects) =
            crate::activation_and_restrictions::parse_choose_creature_type_then_become_type(
                sentence.lowered(),
                next.lowered(),
            )?
    {
        return Ok(Some(Pair::ChosenCreatureType(effects)));
    }
    Ok(None)
}

pub(super) fn open_delayed_upkeep_payment(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if let Some(shape) =
        sequence_grammar::parse_delayed_upkeep_payment_shape(sentence.lowered(), next.lowered())
    {
        return Ok(Some(Pair::DelayedUpkeepPayment(EffectAst::Delayed(
            DelayedEffectAst::DelayedUntilNextUpkeep {
                player: crate::cards::builders::PlayerAst::You,
                effects: vec![EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
                    effects: vec![EffectAst::subject_verb_lose_game(
                        crate::cards::builders::PlayerAst::You,
                    )],
                    player: crate::cards::builders::PlayerAst::You,
                    cost: ironsmith_core::TotalCost::mana(shape.mana),
                    before_delayed_step: false,
                })],
            },
        ))));
    }
    Ok(None)
}

pub(super) fn open_choose_then_rest(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if matches!(first_word, Some("choose" | "each"))
        && let Some(action) = effect_grammar::parse_rest_action_shape(next.lowered())
        && let Some(first_effects) = crate::grammar::primitives::probe_shape(
            crate::effect_sentences::parse_effect_sentence_lexed(sentence.lowered()),
        )
        && let [first] = first_effects.as_slice()
        && let Some(effects) =
            crate::effect_sentences::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::append_rest_action_after_choice(
                first.clone(),
                action,
            ) {
        return Ok(Some(Pair::ChooseThenRest(effects)));
    }
    Ok(None)
}

pub(super) fn open_target_chooses_cant_block(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("target")
        && let Some(effects) =
            crate::activation_and_restrictions::parse_target_player_chooses_then_other_cant_block(
                sentence.lowered(),
                next.lowered(),
            )?
    {
        return Ok(Some(Pair::TargetChoosesCantBlock(effects)));
    }
    Ok(None)
}

/// "copy the next [instant or sorcery] spell [with mana value 2 or less]
/// you cast this turn when you cast it": the spell filter, if any. `None`
/// when the sentence is not this shape; `Some(None)` for any spell.
fn copy_next_spell_filter(
    sentence: &SentenceInput,
) -> Result<Option<Option<crate::target::ObjectFilter>>, CardTextError> {
    const HEAD: [&str; 3] = ["copy", "the", "next"];
    const TAIL: [&str; 8] = ["you", "cast", "this", "turn", "when", "you", "cast", "it"];
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence.lowered());
    let words = crate::lexer::token_word_refs(tokens);
    if words.len() <= HEAD.len() + TAIL.len()
        || words.len() != tokens.len()
        || words[..HEAD.len()] != HEAD
        || words[words.len() - TAIL.len()..] != TAIL
    {
        return Ok(None);
    }
    let middle = &tokens[HEAD.len()..tokens.len() - TAIL.len()];
    let middle_words = &words[HEAD.len()..words.len() - TAIL.len()];
    if middle_words == ["spell"] {
        return Ok(Some(None));
    }
    if !middle_words.contains(&"spell") {
        return Ok(None);
    }
    let Ok(mut filter) = crate::object_filters::parse_object_filter(middle, false) else {
        return Ok(None);
    };
    filter.zone = None;
    Ok(Some(Some(filter)))
}

fn is_copy_retarget_sentence(sentence: &SentenceInput) -> bool {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    crate::word_primitives::parse_sequence_complete(
        &words,
        &["you", "may", "choose", "new", "targets", "for", "the", "copy"],
    ) || crate::word_primitives::parse_sequence_complete(
        &words,
        &["you", "may", "choose", "new", "targets", "for", "the", "copies"],
    )
}

fn copy_next_spell_delayed_trigger(
    filter: Option<crate::target::ObjectFilter>,
    effects: Vec<EffectAst>,
) -> EffectAst {
    EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
        trigger: crate::cards::builders::TriggerSpec::SpellCast {
            filter,
            mana_source_filter: None,
            caster: crate::target::PlayerFilter::You,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        },
        effects,
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    })
}

fn copy_triggering_spell(count: i32) -> EffectAst {
    EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None),
        crate::effect::Value::Fixed(count),
        PlayerAst::You,
        true,
        false,
        Vec::new(),
    )
}

pub(super) fn open_copy_next_spell_retarget(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if let Some(filter) = copy_next_spell_filter(sentence)?
        && is_copy_retarget_sentence(next)
    {
        return Ok(Some(Pair::CopyNextSpellRetarget(
            copy_next_spell_delayed_trigger(filter, vec![copy_triggering_spell(1)]),
        )));
    }
    Ok(None)
}

/// "Copy the next instant or sorcery spell with mana value 2 or less you
/// cast this turn when you cast it. If this creature was kicked, copy that
/// spell twice instead. You may choose new targets for the copies." (Sea Gate
/// Stormcaller)
pub(super) fn copy_next_spell_kicked_retarget(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(sentence), Some(kicked), Some(retarget)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    let Some(filter) = copy_next_spell_filter(sentence)? else {
        return Ok(None);
    };
    if !is_copy_retarget_sentence(retarget) {
        return Ok(None);
    }
    let kicked_tokens: Vec<_> = crate::util::trim_edge_punctuation_tokens(kicked.lowered())
        .iter()
        .filter(|token| token.kind != crate::lexer::TokenKind::Comma)
        .cloned()
        .collect();
    let kicked_tokens = kicked_tokens.as_slice();
    let kicked_words = crate::lexer::token_word_refs(kicked_tokens);
    const TAIL: [&str; 5] = ["copy", "that", "spell", "twice", "instead"];
    if kicked_words.len() != kicked_tokens.len()
        || kicked_words.len() < 4 + TAIL.len()
        || kicked_words[0] != "if"
        || kicked_words[kicked_words.len() - TAIL.len()..] != TAIL
    {
        return Ok(None);
    }
    let condition_tokens = crate::util::trim_edge_punctuation_tokens(
        &kicked_tokens[1..kicked_tokens.len() - TAIL.len()],
    );
    let Ok(predicate) = crate::grammar::filters::parse_condition_predicate_lexed(condition_tokens)
    else {
        return Ok(None);
    };
    if !matches!(
        predicate,
        crate::cards::builders::PredicateAst::TurnHistory(
            crate::cards::builders::TurnHistoryPredicateAst::SourceWasKicked { .. }
        )
    ) {
        return Ok(None);
    }
    // The kicked check runs while the enters trigger resolves, so it reads
    // this creature's own cast record rather than a delayed source's; the
    // twice-copy arm replaces the default one-copy delayed trigger.
    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true: vec![copy_next_spell_delayed_trigger(
            filter.clone(),
            vec![copy_triggering_spell(2)],
        )],
        if_false: vec![copy_next_spell_delayed_trigger(
            filter,
            vec![copy_triggering_spell(1)],
        )],
        attach_to_previous_ability: false,
    }]))
}

pub(super) fn open_destroy_then_search_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("destroy")
        && let Some(effects) = destroy_all_then_search_shuffle(sentence, next)?
    {
        return Ok(Some(Pair::DestroyThenSearchShuffle(effects)));
    }
    Ok(None)
}

pub(super) fn open_search_two_disposition(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("search")
        && let Some(third) = sentences.get(sentence_idx + 2)
        && let Some(effects) = search_two_disposition_then_shuffle(sentence, next, third)?
    {
        return Ok(Some(Pair::SearchTwoDisposition(effects)));
    }
    Ok(None)
}

pub(super) fn open_tempting_offer_copy(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if matches!(first_word, Some("choose" | "tempting"))
        && let [third, fourth, ..] = sentences.get(sentence_idx + 2..).unwrap_or(&[])
        && effect_grammar::is_tempting_offer_copy_sequence(
            sentence.lowered(),
            next.lowered(),
            third.lowered(),
            fourth.lowered(),
        )
    {
        return Ok(Some(Pair::TemptingOfferCopy(tempting_offer_copy_effects())));
    }
    Ok(None)
}

pub(super) fn open_history_counter_source(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("put") {
        if let Some(effects) = history_counter_source(sentence, next)? {
            return Ok(Some(Pair::HistoryCounterOtherwise(effects)));
        }
    }
    Ok(None)
}

pub(super) fn open_history_counter_enchanted(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("put") {
        if let Some(effects) = history_counter_enchanted(sentence, next)? {
            return Ok(Some(Pair::HistoryCounterOtherwise(effects)));
        }
    }
    Ok(None)
}

pub(super) fn open_choose_phase_then_skip(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if matches!(first_word, Some("that" | "the"))
        && let Some(effects) = choose_phase_then_skip(sentence, next)?
    {
        return Ok(Some(Pair::ChoosePhaseThenSkip(effects)));
    }
    Ok(None)
}

pub(super) fn open_each_player_pay_life_tokens(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("starting") {
        if let Some(third) = sentences.get(sentence_idx + 2)
            && let Some(effects) = each_player_pay_life_tokens(sentence, next, third)?
        {
            return Ok(Some(Pair::EachPlayerPayLifeTokens(effects)));
        }
    }
    Ok(None)
}

pub(super) fn open_starting_each_player_optional_repeat(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("starting") {
        if let Some(effects) = starting_each_player_optional_repeat(sentence, next)? {
            return Ok(Some(Pair::StartingEachPlayerRepeat(effects)));
        }
    }
    Ok(None)
}

pub(super) fn open_target_opponent_copy_retarget(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("up")
        && let Some(effects) = target_opponent_copy_retarget(sentence, next)?
    {
        return Ok(Some(Pair::TargetOpponentCopyRetarget(effects)));
    }
    Ok(None)
}

pub(super) fn open_opponents_sacrifice_or_discard_damage(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("each")
        && let Some(effects) = opponents_sacrifice_or_discard_damage(sentence, next)?
    {
        return Ok(Some(Pair::OpponentsSacrificeOrDiscardDamage(effects)));
    }
    Ok(None)
}

#[cfg(test)]
mod catalog_grant_tests {
    use super::*;
    #[test]
    fn collective_flashback_grant_is_a_snapshot_iteration() {
        let tokens = crate::lexer::lex_line("Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.", 0).unwrap();
        let sentences = crate::lexer::split_lexed_sentences(&tokens)
            .into_iter()
            .map(SentenceInput::from_lexed)
            .collect::<Vec<_>>();
        assert!(open_flashback_grant(&sentences, 0).unwrap().is_some());
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
        assert!(
            format!("{effects:?}").contains("ForEachObject"),
            "{effects:#?}"
        );
    }
}

/// The delayed selector owns both spell riders; neither refers to the
/// resolving instant or creates a post-cast trigger.
pub(super) fn open_next_spell_riders(
    sentences: &[SentenceInput], index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = sentences.get(index) else { return Ok(None); };
    let Some(second) = sentences.get(index + 1) else { return Ok(None); };
    let Some(third) = sentences.get(index + 2) else { return Ok(None); };
    let Some(shape) = effect_grammar::parse_next_spell_grant_tokens(first.lowered())? else { return Ok(None); };
    if shape.ability != effect_grammar::NextSpellGrantAbilitySurface::CastTiming {
        return Ok(None);
    }
    let rider_tokens = second.lowered();
    let complete_rider = matches!(rider_tokens, [that, spell, cant, be, countered]
        if that.is_word("that") && spell.is_word("spell")
            && cant.is_any_word(&["cant", "can't"]) && be.is_word("be")
            && countered.is_word("countered"));
    if !complete_rider {
        // The word projection is only a malformed-shape diagnostic, never
        // evidence that the complete sentence was consumed.
        if matches!(crate::lexer::token_word_refs(rider_tokens).as_slice(),
            ["that", "spell", "cant" | "can't", "be", "countered"])
        {
            return Err(CardTextError::ParseError(
                "unexpected token in next-spell uncounterable continuation".into(),
            ));
        }
        return Ok(None);
    }
    let Some(counter) = effect_grammar::counter_marker_shapes::parse_tagged_enters_additional_tokens(third.lowered()) else {
        return Ok(None);
    };
    if !counter.descriptor.additional || counter.descriptor.fewer { return Ok(None); }
    let count = i32::try_from(counter.descriptor.count).map_err(|_| CardTextError::ParseError(
        "next-spell entry counter count is outside the supported range".into(),
    ))?;
    let mut effects = Vec::new();
    for filter in shape.filters {
        effects.push(EffectAst::subject_verb_next_play_timing_this_turn(shape.player, filter.clone(), false));
        let riders = [
            crate::cards::builders::GrantedAbilityAst::KeywordAction(Box::new(crate::payload::KeywordAction::CantBeCountered)),
            crate::cards::builders::GrantedAbilityAst::StaticAbility(Box::new(
                crate::cards::builders::StaticAbilityAst::from(
                    crate::model::CompilerStaticAbilityCore::enters_with_counters_value(counter.descriptor.counter_type, Value::Fixed(count)),
                ),
            )),
        ];
        for ability in riders {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer, shape.player,
                SubjectVerbActionAst::Grants(crate::cards::builders::GrantActionAst::GrantNextSpellAbilityThisTurn {
                    filter: filter.clone(), ability: Box::new(ability),
                    mode: ironsmith_core::NextSpellGrantMode::IncarnationAbility,
                }),
            ));
        }
    }
    Ok(Some(effects))
}

/// An "instead if" price replaces the earlier grant. Resolve the condition
/// once and create one price, so the default and replacement cannot coexist.
pub(super) fn open_flashback_price_replacement(
    sentences: &[SentenceInput], index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = sentences.get(index) else { return Ok(None); };
    let Some(second) = sentences.get(index + 1) else { return Ok(None); };
    let Some(third) = sentences.get(index + 2) else { return Ok(None); };
    let Some(shape) = sequence_grammar::parse_flashback_grant_shape(first.lowered(), second.lowered()) else { return Ok(None); };
    let tokens = third.lowered();
    let Some(split) = crate::slice_primitives::find_window_by(tokens, 2, |tokens| tokens[0].is_word("instead") && tokens[1].is_word("if")) else { return Ok(None); };
    let Some((subject, method)) = crate::effect_sentences::flashback_grants::fixed_grant_parts(&tokens[..split])? else { return Ok(None); };
    if !matches!(subject, [that, card] if that.is_word("that") && card.is_word("card")) {
        return Err(CardTextError::ParseError(
            "flashback price replacement requires the complete subject 'that card'".into(),
        ));
    }
    let target = crate::effect_sentences::parse_target_phrase(shape.target_tokens)?;
    let predicate = crate::grammar::filters::parse_condition_predicate_lexed(&tokens[split + 2..])?;
    let tag = helper_tag_for_tokens(first.lowered(), "flashback_target");
    let grant = |grantable| EffectAst::subject_verb_grant_to_target(
        TargetAst::Tagged(tag.clone(), None), grantable, crate::grant::GrantDuration::UntilEndOfTurn,
    );
    Ok(Some(vec![
        EffectAst::TagReferenced {
            effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
            tag: tag.clone(),
        },
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true: vec![grant(crate::model::CompilerGrantableCore::AlternativeCast(method))],
            if_false: vec![grant(crate::model::CompilerGrantableCore::flashback_from_cards_mana_cost())],
        }),
    ]))
}

#[cfg(test)]
mod next_play_continuation_tests {
    use super::*;

    fn parse(text: &str) -> Result<Vec<EffectAst>, CardTextError> {
        let tokens = crate::lexer::lex_line(text, 0)?;
        crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
    }

    #[test]
    fn complete_next_spell_riders_keep_their_shared_selector() {
        for contraction in ["can't", "can’t", "cant"] {
            let effects = parse(&format!(
                "The next creature spell you cast this turn can be cast as though it had flash. That spell {contraction} be countered. That creature enters with an additional +1/+1 counter on it."
            )).unwrap();
            let debug = format!("{effects:?}");
            assert_eq!(debug.matches("IncarnationAbility").count(), 2, "{debug}");
            assert_eq!(debug.matches("CastTiming").count(), 1, "{debug}");
        }
    }

    #[test]
    fn next_spell_riders_reject_symbols_and_punctuation_in_the_fixed_sentence() {
        for sentence in [
            "That spell can't be countered {R}",
            "That {R} spell can't be countered",
            "That spell: can't be countered",
            "That spell can't be countered:",
            "That spell can't, be countered",
        ] {
            let text = format!(
                "The next creature spell you cast this turn can be cast as though it had flash. {sentence}. That creature enters with an additional +1/+1 counter on it."
            );
            assert!(parse(&text).is_err(), "malformed continuation was reclaimed: {text}");
        }
    }

    #[test]
    fn complete_flashback_replacement_keeps_one_target_and_mutually_exclusive_prices() {
        let effects = parse("Target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. That card gains flashback {0} until end of turn instead if this creature is saddled.").unwrap();
        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[0], EffectAst::TagReferenced { .. }));
        assert!(matches!(&effects[1], EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        }) if if_true.len() == 1 && if_false.len() == 1));
    }

    #[test]
    fn flashback_replacement_rejects_symbols_and_punctuation_in_its_subject() {
        for subject in ["That {R} card", "That card {R}", "That: card", "That card:", "That, card"] {
            let text = format!(
                "Target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. {subject} gains flashback {{0}} until end of turn instead if this creature is saddled."
            );
            assert!(parse(&text).is_err(), "shorter flashback grant reclaimed: {text}");
        }
    }
}
