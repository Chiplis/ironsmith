//! Two-statement procedures whose second statement completes the first.
//!
//! Some sentences bind something the very next sentence completes: "Copy that
//! spell for each other creature that spell could target." followed by "Each
//! copy targets a different one of those creatures." (the second statement
//! selects the per-target reading of the first, which alone reads as a
//! counted copy); "Target instant or sorcery card in your graveyard gains
//! flashback until end of turn." followed by "The flashback cost is equal to
//! its mana cost." (the second supplies the keyword's parameter); "Choose a
//! creature type other than Wall." followed by "Target creature becomes that
//! type until end of turn." (the second refers to the choice). Each is a
//! procedure of two statements, opened by the first when the second follows,
//! as [`super::looked_procedure`] opens a viewed group only when a statement
//! over it follows.

use super::dispatch_entry::SentenceInput;
use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_copy_for_each_target_sentence;
use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::target_opponent_filter;
use crate::cards::builders::{
    CardTextError, ChoiceCount, ChooseOneModeAst, EffectAst, IfResultPredicate, ObjectFilter,
    PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, TargetAst,
};
use crate::effect::Value;
use crate::target::PlayerFilter;
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::lexer::LexedClause;
use crate::util::{helper_tag_for_tokens, trim_commas};
use crate::zone::Zone;
use crate::grammar::effects::{self as effect_grammar, generic_sequence_shapes as sequence_grammar};

use super::sequence_rules::generic_subject_verb_sequences::{
    ordered_control_flow_programs, reference_linked_programs,
};

#[path = "pair_procedure/shapes.rs"]
mod shapes;
use shapes::*;

enum Pair {
    /// The copy for each target, awaiting "Each copy targets a different one".
    CopyForEachTarget(EffectAst),
    /// The flashback grant, awaiting its cost.
    FlashbackGrant(EffectAst),
    /// The creature-type choice, awaiting the "becomes that type" statement;
    /// the two are read together when the second arrives.
    ChosenCreatureType(Vec<EffectAst>),
    /// "At the beginning of your next upkeep, pay {3}{U}{U}." awaiting "If
    /// you don't, you lose the game."
    DelayedUpkeepPayment(EffectAst),
    /// "Each player chooses a creature they control." awaiting "Destroy the
    /// rest.": the rest action bound to the choice.
    ChooseThenRest(Vec<EffectAst>),
    /// "Target opponent chooses a creature they control." awaiting "Other
    /// creatures they control can't block this turn."
    TargetChoosesCantBlock(Vec<EffectAst>),
    /// "Destroy all creatures, then search target opponent's library for …,
    /// put them into their graveyard." awaiting "Then that player shuffles."
    DestroyThenSearchShuffle(Vec<EffectAst>),
    /// "Search your library for two cards." awaiting "Put one into your hand
    /// and the other into your graveyard." and "Then shuffle." — two
    /// completing sentences.
    SearchTwoDisposition(Vec<EffectAst>),
    /// "Copy the next spell you cast this turn when you cast it." awaiting
    /// "You may choose new targets for the copy."
    CopyNextSpellRetarget(EffectAst),
    /// "Tempting offer — Choose target instant or sorcery spell." with the
    /// three sentences of the offer.
    TemptingOfferCopy(Vec<EffectAst>),
    /// A statement with a postfix combat-history condition, awaiting
    /// "Otherwise, …" (Wiitigo, Shape of the Wiitigo).
    HistoryCounterOtherwise(Vec<EffectAst>),
    /// "That player chooses draw step, main phase, or combat phase." awaiting
    /// "The player skips each instance of the chosen step or phase this turn."
    ChoosePhaseThenSkip(Vec<EffectAst>),
    /// "Starting with you, each player may …" awaiting "Repeat this process
    /// until no one …"
    StartingEachPlayerRepeat(Vec<EffectAst>),
    /// "Starting with you, each player may pay any amount of life." with the
    /// repeat and the tokens for the life paid — two completing sentences.
    EachPlayerPayLifeTokens(Vec<EffectAst>),
    /// "Up to one target opponent may also copy that spell." awaiting "They
    /// may choose new targets for that copy."
    TargetOpponentCopyRetarget(Vec<EffectAst>),
    /// "Each opponent may sacrifice a nonland permanent of their choice or
    /// discard a card." awaiting the damage to those who did neither.
    OpponentsSacrificeOrDiscardDamage(Vec<EffectAst>),
    /// A statement whose completing sentences are read together with it by
    /// one of the fixed-shape parsers below; `remaining` counts them.
    FixedShape(Vec<EffectAst>),
}

pub(super) struct PairGroup {
    pair: Pair,
    /// How many completing sentences remain to be consumed.
    remaining: usize,
    /// The feature the fixed-shape parsers' programs reported.
    feature: &'static str,
    completed: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn is_each_copy_targets_different(sentence: &SentenceInput) -> bool {
    effect_grammar::each_copy_targets_different_shape(sentence.lowered())
}

fn choose_creature_type_sentence(sentence: &SentenceInput) -> bool {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    words.first() == Some(&"choose")
        && crate::word_primitives::sequence_occurs(&words, &["creature", "type"])
}

/// Open a procedure at a sentence the next sentence completes.
/// A shape's reading, with its error set aside for the caller to raise only
/// when nothing else recognizes the document.
fn deferring<T>(
    parsed: Result<Option<T>, CardTextError>,
    deferred: &mut Option<CardTextError>,
) -> Option<T> {
    match parsed {
        Ok(value) => value,
        Err(error) => {
            deferred.get_or_insert(error);
            None
        }
    }
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<PairGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let group = |pair| {
        let remaining = match &pair {
            Pair::SearchTwoDisposition(_) | Pair::EachPlayerPayLifeTokens(_) => 2,
            Pair::TemptingOfferCopy(_) => 3,
            _ => 1,
        };
        Ok(Some(PairGroup {
            pair,
            remaining,
            feature: "pair",
            completed: false,
            first_sentence: sentence_idx,
            consumed: 1,
        }))
    };
    // Statements read together with the sentences that complete them, in the
    // order their programs were ranked.
    let head = |words: &[&str]| super::sequence_rules::sentence_head_word_in(sentences, sentence_idx, words);
    let next_head = |word: &str| super::sequence_rules::sentence_head_word_is(sentences, sentence_idx + 1, word);
    let fixed = |effects: Vec<EffectAst>, consumed: usize, feature: &'static str| {
        Ok(Some(PairGroup {
            pair: Pair::FixedShape(effects),
            remaining: consumed - 1,
            feature,
            completed: false,
            first_sentence: sentence_idx,
            consumed: 1,
        }))
    };
    let has = |consumed: usize| sentences.len() >= sentence_idx + consumed;
    // A shape that errs is not this document unless nothing recognizes it:
    // its error waits behind any shape or procedure that does, as a rule's
    // committed error waited behind any rule that matched.
    let mut deferred: Option<CardTextError> = None;
    if has(2) && head(&["you"])
        && let Some(effects) = deferring(reference_linked_programs::parse_controller_defending_loot_then_greatest_mana_value_followup(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "participant-loot");
    }
    if has(2) && head(&["you"])
        && let Some(effects) = deferring(reference_linked_programs::parse_participant_secret_object_choice_then_reveal_and_sacrifice(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "participant-secret-choice");
    }
    if has(3) && head(&["you", "untap"])
        && let Some(effects) = deferring(reference_linked_programs::parse_reciprocal_creature_control_sequence(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "reciprocal-creature-control");
    }
    if has(3) && head(&["choose"])
        && (super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "the", "same", "player"]) || super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "same", "player"]))
        && let Some(effects) = deferring(reference_linked_programs::parse_choose_same_controller_targets_then_sacrifice_one_return_other(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "same-controller-sacrifice-return");
    }
    if has(2) && head(&["choose"])
        && (super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "the", "same", "player"]) || super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "same", "player"]))
        && let Some(effects) = deferring(reference_linked_programs::parse_choose_same_controller_targets_then_sacrifice_one(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "same-controller-sacrifice");
    }
    if has(2) && head(&["choose"])
        && (super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["do", "same"]) || super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["do", "the", "same"]))
        && let Some(effects) = deferring(reference_linked_programs::parse_choose_then_do_same_for_filter_then_return_to_battlefield(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "choose-do-same-return");
    }
    if has(3) && head(&["choose"])
        && super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["card", "name"])
        && let Some(effects) = deferring(ordered_control_flow_programs::parse_choose_name_reveal_top_matching_hand_rest_graveyard(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "chosen-name-reveal");
    }
    if has(3) && head(&["choose"])
        && super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["land", "or", "nonland"])
        && let Some(effects) = deferring(ordered_control_flow_programs::parse_choose_land_or_nonland_then_consult_to_hand_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "chosen-kind-consult");
    }
    if has(2) && head(&["starting"])
        && let Some(effects) = deferring(reference_linked_programs::parse_directional_adjacent_player_control(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "directional-adjacent-control");
    }
    if has(2) && head(&["if", "for"]) && next_head("the")
        && let Some(effects) = deferring(reference_linked_programs::parse_for_each_tagged_copy_then_copy_targets_it(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "tagged-copy-retarget");
    }
    if has(2) && head(&["draw"])
        && let Some(effects) = deferring(reference_linked_programs::parse_draw_reveal_then_triggering_creature_mana_value_result(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "draw-reveal-mana-value");
    }
    if has(3) && head(&["each"])
        && let Some(effects) = deferring(ordered_control_flow_programs::parse_each_player_mill_then_land_result_then_cast_one_milled_spell(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "mill-land-result-cast");
    }
    if has(3) && head(&["target"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_target_modifier_counter_instead_then_common_damage(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "target-modifier-counter-instead-common-damage");
    }
    if has(2) && head(&["counter"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_counter_spell_then_artifact_or_creature_enters_under_your_control(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "counter-spell-artifact-creature-battlefield-replacement");
    }
    if has(4) && head(&["reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_reveal_top_choose_and_or_hand_rest_bottom_with_destination_override(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 4, "revealed-and-or-choice-destination-override");
    }
    if has(4) && head(&["look", "reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_top_cards_move_then_grant_rest_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 4, "looked-battlefield-grant-rest-bottom");
    }
    if has(4) && head(&["look"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_look_reveal_one_or_instead_two_then_rest_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 4, "look-reveal-one-or-instead-two-rest-bottom");
    }
    if has(4) && head(&["look"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 4, "look-may-sacrifice-if-did-select-battlefield-rest-bottom");
    }
    if has(4) && head(&["look"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_look_then_may_action_if_did_or_did_not_move_looked_card(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 4, "look-may-action-result-branches-move-looked-card");
    }
    if has(4) && head(&["reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_reveal_top_optional_battlefield_then_hand_rest_graveyard(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 4, "reveal-top-optional-battlefield-then-hand-rest-graveyard");
    }
    if has(3) && head(&["destroy"])
        && let Some(effects) = deferring(ordered_control_flow_programs::parse_destroy_historically_blocked_then_reanimate_from_historical_controller(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "destroy-historical-blocker-reanimation");
    }
    if has(3) && head(&["destroy"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::parse_destroy_for_each_destroyed_consult_exile_put_shuffle(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "destroy-for-each-destroyed-consult-exile-put-shuffle");
    }
    if has(3) && head(&["look"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_may_put_with_counter_then_rest_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "look-at-top-may-put-with-counter-rest-bottom");
    }
    if has(3) && head(&["look"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_partition_face_down_then_filtered_permission(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "look-at-top-partition-face-down-filtered-permission");
    }
    if has(3) && head(&["look"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "look-at-top-exile-match-and-rest-bottom-cast-exiled");
    }
    if has(3) && head(&["search"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_search_then_player_names_card_conditional_put_then_shuffle(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "search-player-names-card-conditional-put-then-shuffle");
    }
    if has(3) && head(&["look", "reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_one_hand_then_matching_to_zone_rest_graveyard(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "top-cards-one-hand-then-matching-to-zone-rest-graveyard");
    }
    if has(3) && head(&["reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_reveal_top_one_hand_gain_mana_value_rest_graveyard(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "reveal-top-one-hand-gain-mana-value-rest-graveyard");
    }
    if has(3) && head(&["look", "reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "top-cards-choose-for-each-filter-one-battlefield-others-hand-rest-graveyard");
    }
    if has(3) && head(&["reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "top-cards-for-each-card-type-put-matching-into-hand-rest-bottom");
    }
    if has(3) && head(&["reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "top-cards-for-each-card-type-among-spells-put-matching-into-hand-rest-bottom");
    }
    if has(3) && head(&["exile"]) && super::sequence_rules::sentence_head_word_is(sentences, sentence_idx + 2, "repeat")
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::parse_iterative_library_procedure_sequence(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 3, "iterative-library-procedure-sequence");
    }
    if has(2) && head(&["if", "target", "you", "that", "they", "exile", "look", "reveal"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_exile_face_down_pile_then_cloak(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "exile-face-down-pile-then-cloak-tapped");
    }
    if has(2) && head(&["each"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::parse_each_player_shuffle_reveal_then_put_revealed_types_bottom(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "each-player-shuffle-reveal-put-revealed-types-rest-bottom");
    }
    if has(2) && head(&["if"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_filtered_future_exile_then_return_next_end_step(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "filtered-future-exile-then-return-next-end-step");
    }
    if has(2) && head(&["when"]) && super::sequence_rules::sentence_head_is(sentences, sentence_idx, ("when", Some("that")))
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_delayed_dies_exile_top_power_choose_play(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "delayed-dies-exile-top-power-choose-play");
    }
    if has(2) && head(&["choose"]) && super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["card", "type"])
        && let Some(effects) = deferring(super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "choose-card-type-then-reveal-and-put");
    }
    if is_each_copy_targets_different(next)
        && let Some(effect) =
            deferring(parse_copy_for_each_target_sentence(sentences, sentence_idx, sentence.lowered()), &mut deferred)
    {
        return group(Pair::CopyForEachTarget(effect));
    }
    if has(2) && head(&["target", "exile", "you", "that", "they"])
        && let Some(effects) = deferring(reference_linked_programs::parse_exile_until_match_grant_play_this_turn(sentences, sentence_idx), &mut deferred)
    {
        return fixed(effects, 2, "consult-grant-play");
    }
    // The grant names its target ("Target instant or sorcery card in your
    // graveyard gains flashback"); "Each instant and sorcery card ..." is a
    // different statement this does not read.
    if crate::lexer::token_word_refs(sentence.lowered()).first() == Some(&"target")
        && let Some(shape) =
            sequence_grammar::parse_flashback_grant_shape(sentence.lowered(), next.lowered())
    {
        let target = super::parse_target_phrase(shape.target_tokens)?;
        return group(Pair::FlashbackGrant(EffectAst::subject_verb_grant_to_target(
            target,
            crate::model::CompilerGrantableCore::flashback_from_cards_mana_cost(),
            crate::grant::GrantDuration::UntilEndOfTurn,
        )));
    }
    if choose_creature_type_sentence(sentence)
        && let Some(effects) =
            deferring(crate::activation_and_restrictions::parse_choose_creature_type_then_become_type(
                sentence.lowered(),
                next.lowered(),
            ), &mut deferred)
    {
        return group(Pair::ChosenCreatureType(effects));
    }
    if let Some(shape) =
        sequence_grammar::parse_delayed_upkeep_payment_shape(sentence.lowered(), next.lowered())
    {
        return group(Pair::DelayedUpkeepPayment(EffectAst::DelayedUntilNextUpkeep {
            player: crate::cards::builders::PlayerAst::You,
            effects: vec![EffectAst::UnlessPays {
                effects: vec![EffectAst::subject_verb_lose_game(
                    crate::cards::builders::PlayerAst::You,
                )],
                player: crate::cards::builders::PlayerAst::You,
                cost: ironsmith_core::TotalCost::mana(shape.mana),
                before_delayed_step: false,
            }],
        }));
    }
    let first_word = crate::lexer::token_word_refs(sentence.lowered())
        .first()
        .copied();
    if matches!(first_word, Some("choose" | "each"))
        && let Some(action) = effect_grammar::parse_rest_action_shape(next.lowered())
        && let Some(first_effects) = crate::grammar::primitives::probe_shape(
            super::parse_effect_sentence_lexed(sentence.lowered()),
        )
        && let [first] = first_effects.as_slice()
        && let Some(effects) =
            super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::append_rest_action_after_choice(
                first.clone(),
                action,
            )
    {
        return group(Pair::ChooseThenRest(effects));
    }
    if first_word == Some("target")
        && let Some(effects) =
            deferring(crate::activation_and_restrictions::parse_target_player_chooses_then_other_cant_block(
                sentence.lowered(),
                next.lowered(),
            ), &mut deferred)
    {
        return group(Pair::TargetChoosesCantBlock(effects));
    }
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
        )
    {
        return group(Pair::CopyNextSpellRetarget(EffectAst::DelayedTriggerThisTurn {
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
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None),
                crate::effect::Value::Fixed(1),
                PlayerAst::You,
                true,
                false,
                Vec::new(),
            )],
            one_shot: true,
            until_end_of_combat: false,
            attach_to_previous_ability: false,
        }));
    }
    if first_word == Some("destroy")
        && let Some(effects) = deferring(destroy_all_then_search_shuffle(sentence, next), &mut deferred)
    {
        return group(Pair::DestroyThenSearchShuffle(effects));
    }
    if first_word == Some("search")
        && let Some(third) = sentences.get(sentence_idx + 2)
        && let Some(effects) = deferring(search_two_disposition_then_shuffle(sentence, next, third), &mut deferred)
    {
        return group(Pair::SearchTwoDisposition(effects));
    }
    if matches!(first_word, Some("choose" | "tempting"))
        && let [third, fourth, ..] = sentences.get(sentence_idx + 2..).unwrap_or(&[])
        && effect_grammar::is_tempting_offer_copy_sequence(
            sentence.lowered(),
            next.lowered(),
            third.lowered(),
            fourth.lowered(),
        )
    {
        return group(Pair::TemptingOfferCopy(tempting_offer_copy_effects()));
    }
    if first_word == Some("put") {
        if let Some(effects) = deferring(history_counter_source(sentence, next), &mut deferred) {
            return group(Pair::HistoryCounterOtherwise(effects));
        }
        if let Some(effects) = deferring(history_counter_enchanted(sentence, next), &mut deferred) {
            return group(Pair::HistoryCounterOtherwise(effects));
        }
    }
    if matches!(first_word, Some("that" | "the"))
        && let Some(effects) = deferring(choose_phase_then_skip(sentence, next), &mut deferred)
    {
        return group(Pair::ChoosePhaseThenSkip(effects));
    }
    if first_word == Some("starting") {
        if let Some(third) = sentences.get(sentence_idx + 2)
            && let Some(effects) = deferring(each_player_pay_life_tokens(sentence, next, third), &mut deferred)
        {
            return group(Pair::EachPlayerPayLifeTokens(effects));
        }
        if let Some(effects) = deferring(starting_each_player_optional_repeat(sentence, next), &mut deferred) {
            return group(Pair::StartingEachPlayerRepeat(effects));
        }
    }
    if first_word == Some("up")
        && let Some(effects) = deferring(target_opponent_copy_retarget(sentence, next), &mut deferred)
    {
        return group(Pair::TargetOpponentCopyRetarget(effects));
    }
    if first_word == Some("each")
        && let Some(effects) = deferring(opponents_sacrifice_or_discard_damage(sentence, next), &mut deferred)
    {
        return group(Pair::OpponentsSacrificeOrDiscardDamage(effects));
    }
    match deferred {
        Some(error) => Err(error),
        None => Ok(None),
    }
}

/// Continue with the completing statement; false for anything else.
pub(super) fn continue_with(
    group: &mut PairGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.completed {
        return Ok(false);
    }
    let completes = match &group.pair {
        Pair::CopyForEachTarget(_) => is_each_copy_targets_different(sentence),
        // The opener read the completing sentences; these are the ones it read.
        Pair::FlashbackGrant(_)
        | Pair::ChosenCreatureType(_)
        | Pair::DelayedUpkeepPayment(_)
        | Pair::ChooseThenRest(_)
        | Pair::TargetChoosesCantBlock(_)
        | Pair::DestroyThenSearchShuffle(_)
        | Pair::SearchTwoDisposition(_)
        | Pair::CopyNextSpellRetarget(_)
        | Pair::TemptingOfferCopy(_)
        | Pair::HistoryCounterOtherwise(_)
        | Pair::ChoosePhaseThenSkip(_)
        | Pair::StartingEachPlayerRepeat(_)
        | Pair::EachPlayerPayLifeTokens(_)
        | Pair::TargetOpponentCopyRetarget(_)
        | Pair::OpponentsSacrificeOrDiscardDamage(_)
        | Pair::FixedShape(_) => true,
    };
    if !completes {
        return Ok(false);
    }
    group.remaining -= 1;
    group.completed = group.remaining == 0;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn feature_tag(group: &PairGroup) -> &'static str {
    match group.pair {
        Pair::CopyForEachTarget(_) => "copy-target-assignment",
        Pair::FlashbackGrant(_) => "flashback-cost-followup",
        Pair::ChosenCreatureType(_) => "choose-creature-type",
        Pair::DelayedUpkeepPayment(_) => "delayed-upkeep-payment",
        Pair::ChooseThenRest(_) => "choose-then-rest",
        Pair::TargetChoosesCantBlock(_) => "target-chooses-cant-block",
        Pair::DestroyThenSearchShuffle(_) => "destroy-search-shuffle",
        Pair::SearchTwoDisposition(_) => "search-two-disposition",
        Pair::CopyNextSpellRetarget(_) => "copy-next-spell-retarget",
        Pair::TemptingOfferCopy(_) => "tempting-offer-copy",
        Pair::HistoryCounterOtherwise(_) => "history-counter-otherwise",
        Pair::ChoosePhaseThenSkip(_) => "choose-phase-then-skip",
        Pair::StartingEachPlayerRepeat(_) => "starting-each-player-repeat",
        Pair::EachPlayerPayLifeTokens(_) => "each-player-pay-life-tokens",
        Pair::TargetOpponentCopyRetarget(_) => "target-opponent-copy-retarget",
        Pair::OpponentsSacrificeOrDiscardDamage(_) => "opponents-sacrifice-or-discard-damage",
        Pair::FixedShape(_) => group.feature,
    }
}

pub(super) fn finish(group: PairGroup) -> Vec<EffectAst> {
    match group.pair {
        Pair::CopyForEachTarget(effect)
        | Pair::FlashbackGrant(effect)
        | Pair::DelayedUpkeepPayment(effect)
        | Pair::CopyNextSpellRetarget(effect) => vec![effect],
        Pair::ChosenCreatureType(effects)
        | Pair::ChooseThenRest(effects)
        | Pair::TargetChoosesCantBlock(effects)
        | Pair::DestroyThenSearchShuffle(effects)
        | Pair::SearchTwoDisposition(effects)
        | Pair::TemptingOfferCopy(effects)
        | Pair::HistoryCounterOtherwise(effects)
        | Pair::ChoosePhaseThenSkip(effects)
        | Pair::StartingEachPlayerRepeat(effects)
        | Pair::EachPlayerPayLifeTokens(effects)
        | Pair::TargetOpponentCopyRetarget(effects)
        | Pair::OpponentsSacrificeOrDiscardDamage(effects)
        | Pair::FixedShape(effects) => effects,
    }
}
