//! The pair kinds that are not fixed shapes: each reads its opening statement
//! and the sentence completing it into a [`Pair`].

use super::*;

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
            parse_copy_for_each_target_sentence(sentences, sentence_idx, sentence.lowered())? {
        return Ok(Some(Pair::CopyForEachTarget(effect)));
    }
    Ok(None)
}

pub(super) fn open_flashback_grant(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Pair>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if crate::lexer::token_word_refs(sentence.lowered()).first() == Some(&"target")
        && let Some(shape) =
            sequence_grammar::parse_flashback_grant_shape(sentence.lowered(), next.lowered()) {
        let target = crate::effect_sentences::parse_target_phrase(shape.target_tokens)?;
        return Ok(Some(Pair::FlashbackGrant(EffectAst::subject_verb_grant_to_target(
            target,
            crate::model::CompilerGrantableCore::flashback_from_cards_mana_cost(),
            crate::grant::GrantDuration::UntilEndOfTurn,
        ))));
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
    if choose_creature_type_sentence(sentence)
        && let Some(effects) =
            crate::activation_and_restrictions::parse_choose_creature_type_then_become_type(
                sentence.lowered(),
                next.lowered(),
            )? {
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
        sequence_grammar::parse_delayed_upkeep_payment_shape(sentence.lowered(), next.lowered()) {
        return Ok(Some(Pair::DelayedUpkeepPayment(EffectAst::DelayedUntilNextUpkeep {
            player: crate::cards::builders::PlayerAst::You,
            effects: vec![EffectAst::UnlessPays {
                effects: vec![EffectAst::subject_verb_lose_game(
                    crate::cards::builders::PlayerAst::You,
                )],
                player: crate::cards::builders::PlayerAst::You,
                cost: ironsmith_core::TotalCost::mana(shape.mana),
                before_delayed_step: false,
            }],
        })));
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
            )? {
        return Ok(Some(Pair::TargetChoosesCantBlock(effects)));
    }
    Ok(None)
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
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if first_word == Some("copy")
        && crate::word_primitives::parse_sequence_complete(
            &crate::lexer::token_word_refs(sentence.lowered()),
            &[
                "copy", "the", "next", "spell", "you", "cast", "this", "turn", "when", "you", "cast",
                "it",
            ],
        )
        && crate::word_primitives::parse_sequence_complete(
            &crate::lexer::token_word_refs(next.lowered()),
            &["you", "may", "choose", "new", "targets", "for", "the", "copy"],
        ) {
        return Ok(Some(Pair::CopyNextSpellRetarget(EffectAst::DelayedTriggerThisTurn {
            trigger: crate::cards::builders::TriggerSpec::SpellCast {
                filter: None,
                mana_source_filter: None,
                caster: crate::target::PlayerFilter::You,
                timing: None,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            },
            effects: vec![EffectAst::subject_verb_copy_spell(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None),
                crate::effect::Value::Fixed(1),
                PlayerAst::You,
                true,
                false,
                Vec::new(),
            )],
            one_shot: true,
            until_end_of_combat: false,
            attach_to_previous_ability: false,
        })));
    }
    Ok(None)
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
        && let Some(effects) = destroy_all_then_search_shuffle(sentence, next)? {
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
        && let Some(effects) = search_two_disposition_then_shuffle(sentence, next, third)? {
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
        ) {
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
        && let Some(effects) = choose_phase_then_skip(sentence, next)? {
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
        && let Some(effects) = target_opponent_copy_retarget(sentence, next)? {
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
        && let Some(effects) = opponents_sacrifice_or_discard_damage(sentence, next)? {
        return Ok(Some(Pair::OpponentsSacrificeOrDiscardDamage(effects)));
    }
    Ok(None)
}
