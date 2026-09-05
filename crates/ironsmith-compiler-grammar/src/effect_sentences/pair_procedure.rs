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

use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};
use super::sequence_rules::generic_subject_verb_sequences::{
    ordered_control_flow_programs, reference_linked_programs,
};

#[path = "pair_procedure/kinds.rs"]
mod kinds;
#[path = "pair_procedure/shapes.rs"]
mod shapes;
use shapes::*;

#[derive(PartialEq)]
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
/// A statement read together with the sentences completing it: the head word
/// that opens it, how many sentences it reads, and the reading.
struct Shape {
    id: RuleId,
    head: HeadDiscriminator,
    consumed: usize,
    read: fn(&[SentenceInput], usize) -> ParseOutcome<Pair>,
}

/// The pair shapes, in the order their programs were ranked. Every shape whose
/// head accepts the sentence is read; the longest complete reading is the
/// document's, as the registry kept the rule consuming the longest program,
/// and equal readings are one; two readings that disagree are an ambiguity.
const PAIR_SHAPES: &[Shape] = &[
    Shape {
        id: RuleId::new("participant-loot"),
        head: HeadDiscriminator::words(&["you"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, reference_linked_programs::parse_controller_defending_loot_then_greatest_mana_value_followup(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("participant-secret-choice"),
        head: HeadDiscriminator::words(&["you"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, reference_linked_programs::parse_participant_secret_object_choice_then_reveal_and_sacrifice(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("reciprocal-creature-control"),
        head: HeadDiscriminator::words(&["you", "untap"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, reference_linked_programs::parse_reciprocal_creature_control_sequence(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("same-controller-sacrifice-return"),
        head: HeadDiscriminator::words(&["choose"]),
        consumed: 3,
        read: |sentences, sentence_idx| {
            if !((super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "the", "same", "player"]) || super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "same", "player"]))) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, reference_linked_programs::parse_choose_same_controller_targets_then_sacrifice_one_return_other(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("same-controller-sacrifice"),
        head: HeadDiscriminator::words(&["choose"]),
        consumed: 2,
        read: |sentences, sentence_idx| {
            if !((super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "the", "same", "player"]) || super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["controlled", "by", "same", "player"]))) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, reference_linked_programs::parse_choose_same_controller_targets_then_sacrifice_one(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("choose-do-same-return"),
        head: HeadDiscriminator::words(&["choose"]),
        consumed: 2,
        read: |sentences, sentence_idx| {
            if !((super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["do", "same"]) || super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["do", "the", "same"]))) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, reference_linked_programs::parse_choose_then_do_same_for_filter_then_return_to_battlefield(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("chosen-name-reveal"),
        head: HeadDiscriminator::words(&["choose"]),
        consumed: 3,
        read: |sentences, sentence_idx| {
            if !(super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["card", "name"])) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, ordered_control_flow_programs::parse_choose_name_reveal_top_matching_hand_rest_graveyard(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("chosen-kind-consult"),
        head: HeadDiscriminator::words(&["choose"]),
        consumed: 3,
        read: |sentences, sentence_idx| {
            if !(super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["land", "or", "nonland"])) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, ordered_control_flow_programs::parse_choose_land_or_nonland_then_consult_to_hand_bottom(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("directional-adjacent-control"),
        head: HeadDiscriminator::words(&["starting"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, reference_linked_programs::parse_directional_adjacent_player_control(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("tagged-copy-retarget"),
        head: HeadDiscriminator::words(&["if", "for"]),
        consumed: 2,
        read: |sentences, sentence_idx| {
            if !(super::sequence_rules::sentence_head_word_is(sentences, sentence_idx + 1, "the")) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, reference_linked_programs::parse_for_each_tagged_copy_then_copy_targets_it(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("draw-reveal-mana-value"),
        head: HeadDiscriminator::words(&["draw"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, reference_linked_programs::parse_draw_reveal_then_triggering_creature_mana_value_result(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("mill-land-result-cast"),
        head: HeadDiscriminator::words(&["each"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, ordered_control_flow_programs::parse_each_player_mill_then_land_result_then_cast_one_milled_spell(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("target-modifier-counter-instead-common-damage"),
        head: HeadDiscriminator::words(&["target"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_target_modifier_counter_instead_then_common_damage(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("counter-spell-artifact-creature-battlefield-replacement"),
        head: HeadDiscriminator::words(&["counter"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_counter_spell_then_artifact_or_creature_enters_under_your_control(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("revealed-and-or-choice-destination-override"),
        head: HeadDiscriminator::words(&["reveal"]),
        consumed: 4,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_reveal_top_choose_and_or_hand_rest_bottom_with_destination_override(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("looked-battlefield-grant-rest-bottom"),
        head: HeadDiscriminator::words(&["look", "reveal"]),
        consumed: 4,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_top_cards_move_then_grant_rest_bottom(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("look-reveal-one-or-instead-two-rest-bottom"),
        head: HeadDiscriminator::words(&["look"]),
        consumed: 4,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_look_reveal_one_or_instead_two_then_rest_bottom(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("look-may-sacrifice-if-did-select-battlefield-rest-bottom"),
        head: HeadDiscriminator::words(&["look"]),
        consumed: 4,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("look-may-action-result-branches-move-looked-card"),
        head: HeadDiscriminator::words(&["look"]),
        consumed: 4,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_look_then_may_action_if_did_or_did_not_move_looked_card(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("reveal-top-optional-battlefield-then-hand-rest-graveyard"),
        head: HeadDiscriminator::words(&["reveal"]),
        consumed: 4,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_reveal_top_optional_battlefield_then_hand_rest_graveyard(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("destroy-historical-blocker-reanimation"),
        head: HeadDiscriminator::words(&["destroy"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, ordered_control_flow_programs::parse_destroy_historically_blocked_then_reanimate_from_historical_controller(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("destroy-for-each-destroyed-consult-exile-put-shuffle"),
        head: HeadDiscriminator::words(&["destroy"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::parse_destroy_for_each_destroyed_consult_exile_put_shuffle(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("look-at-top-may-put-with-counter-rest-bottom"),
        head: HeadDiscriminator::words(&["look"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_may_put_with_counter_then_rest_bottom(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("look-at-top-partition-face-down-filtered-permission"),
        head: HeadDiscriminator::words(&["look"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_partition_face_down_then_filtered_permission(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("look-at-top-exile-match-and-rest-bottom-cast-exiled"),
        head: HeadDiscriminator::words(&["look"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("search-player-names-card-conditional-put-then-shuffle"),
        head: HeadDiscriminator::words(&["search"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_search_then_player_names_card_conditional_put_then_shuffle(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("top-cards-one-hand-then-matching-to-zone-rest-graveyard"),
        head: HeadDiscriminator::words(&["look", "reveal"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_one_hand_then_matching_to_zone_rest_graveyard(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("reveal-top-one-hand-gain-mana-value-rest-graveyard"),
        head: HeadDiscriminator::words(&["reveal"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_reveal_top_one_hand_gain_mana_value_rest_graveyard(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("top-cards-choose-for-each-filter-one-battlefield-others-hand-rest-graveyard"),
        head: HeadDiscriminator::words(&["look", "reveal"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("top-cards-for-each-card-type-put-matching-into-hand-rest-bottom"),
        head: HeadDiscriminator::words(&["reveal"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("top-cards-for-each-card-type-among-spells-put-matching-into-hand-rest-bottom"),
        head: HeadDiscriminator::words(&["reveal"]),
        consumed: 3,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("iterative-library-procedure-sequence"),
        head: HeadDiscriminator::words(&["exile"]),
        consumed: 3,
        read: |sentences, sentence_idx| {
            if !(super::sequence_rules::sentence_head_word_is(sentences, sentence_idx + 2, "repeat")) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::parse_iterative_library_procedure_sequence(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("exile-face-down-pile-then-cloak-tapped"),
        head: HeadDiscriminator::words(&["if", "target", "you", "that", "they", "exile", "look", "reveal"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_exile_face_down_pile_then_cloak(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("each-player-shuffle-reveal-put-revealed-types-rest-bottom"),
        head: HeadDiscriminator::words(&["each"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::parse_each_player_shuffle_reveal_then_put_revealed_types_bottom(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("filtered-future-exile-then-return-next-end-step"),
        head: HeadDiscriminator::words(&["if"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_filtered_future_exile_then_return_next_end_step(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("delayed-dies-exile-top-power-choose-play"),
        head: HeadDiscriminator::words(&["when"]),
        consumed: 2,
        read: |sentences, sentence_idx| {
            if !(super::sequence_rules::sentence_head_is(sentences, sentence_idx, ("when", Some("that")))) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_delayed_dies_exile_top_power_choose_play(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("choose-card-type-then-reveal-and-put"),
        head: HeadDiscriminator::words(&["choose"]),
        consumed: 2,
        read: |sentences, sentence_idx| {
            if !(super::sequence_rules::sentence_words_contain(sentences, sentence_idx, &["card", "type"])) {
                return ParseOutcome::NoMatch;
            }
            statements(sentences, sentence_idx, super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(sentences, sentence_idx))
        },
    },
    Shape {
        id: RuleId::new("copy-for-each-target"),
        head: HeadDiscriminator::Any,
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_copy_for_each_target(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("consult-grant-play"),
        head: HeadDiscriminator::words(&["target", "exile", "you", "that", "they"]),
        consumed: 2,
        read: |sentences, sentence_idx| statements(sentences, sentence_idx, reference_linked_programs::parse_exile_until_match_grant_play_this_turn(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("flashback-grant"),
        head: HeadDiscriminator::words(&["target"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_flashback_grant(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("chosen-creature-type"),
        head: HeadDiscriminator::Any,
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_chosen_creature_type(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("delayed-upkeep-payment"),
        head: HeadDiscriminator::Any,
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_delayed_upkeep_payment(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("choose-then-rest"),
        head: HeadDiscriminator::words(&["choose", "each"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_choose_then_rest(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("target-chooses-cant-block"),
        head: HeadDiscriminator::words(&["target"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_target_chooses_cant_block(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("copy-next-spell-retarget"),
        head: HeadDiscriminator::words(&["copy"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_copy_next_spell_retarget(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("destroy-then-search-shuffle"),
        head: HeadDiscriminator::words(&["destroy"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_destroy_then_search_shuffle(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("search-two-disposition"),
        head: HeadDiscriminator::words(&["search"]),
        consumed: 3,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_search_two_disposition(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("tempting-offer-copy"),
        head: HeadDiscriminator::words(&["choose", "tempting"]),
        consumed: 4,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_tempting_offer_copy(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("history-counter-source"),
        head: HeadDiscriminator::words(&["put"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_history_counter_source(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("history-counter-enchanted"),
        head: HeadDiscriminator::words(&["put"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_history_counter_enchanted(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("choose-phase-then-skip"),
        head: HeadDiscriminator::words(&["that", "the"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_choose_phase_then_skip(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("each-player-pay-life-tokens"),
        head: HeadDiscriminator::words(&["starting"]),
        consumed: 3,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_each_player_pay_life_tokens(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("starting-each-player-optional-repeat"),
        head: HeadDiscriminator::words(&["starting"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_starting_each_player_optional_repeat(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("target-opponent-copy-retarget"),
        head: HeadDiscriminator::words(&["up"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_target_opponent_copy_retarget(sentences, sentence_idx)),
    },
    Shape {
        id: RuleId::new("opponents-sacrifice-or-discard-damage"),
        head: HeadDiscriminator::words(&["each"]),
        consumed: 2,
        read: |sentences, sentence_idx| reading(sentences, sentence_idx, kinds::open_opponents_sacrifice_or_discard_damage(sentences, sentence_idx)),
    },
];

/// A reading's outcome: the shape's error is a committed diagnostic on the
/// opening sentence.
fn reading(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    read: Result<Option<Pair>, CardTextError>,
) -> ParseOutcome<Pair> {
    let span = sentences
        .get(sentence_idx)
        .and_then(|sentence| crate::util::span_from_tokens(sentence.lowered()));
    match read {
        Ok(Some(pair)) => ParseOutcome::matched(pair, span),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
            RuleId::new("pair-shape"),
            span,
            error,
        )),
    }
}

/// The outcome of a shape parser that reads the statements themselves.
fn statements(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    read: Result<Option<Vec<EffectAst>>, CardTextError>,
) -> ParseOutcome<Pair> {
    reading(sentences, sentence_idx, read.map(|effects| effects.map(Pair::FixedShape)))
}

/// How many completing sentences a pair still awaits once opened.
fn remaining(pair: &Pair, consumed: usize) -> usize {
    match pair {
        Pair::FixedShape(_) => consumed - 1,
        Pair::SearchTwoDisposition(_) | Pair::EachPlayerPayLifeTokens(_) => 2,
        Pair::TemptingOfferCopy(_) => 3,
        _ => 1,
    }
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<PairGroup>, CardTextError> {
    if sentences.len() < sentence_idx + 2 {
        return Ok(None);
    }
    let head = super::sequence_rules::sentence_head_word(sentences, sentence_idx).unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for shape in PAIR_SHAPES {
        if !shape.head.accepts(head) || sentences.len() < sentence_idx + shape.consumed {
            continue;
        }
        match (shape.read)(sentences, sentence_idx).within(shape.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(shape.id, shape.head),
                (matched.value, shape.consumed),
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    let longest = candidates
        .iter()
        .map(|candidate| candidate.value.1)
        .max()
        .unwrap_or(0);
    candidates.retain(|candidate| candidate.value.1 == longest);
    let mut distinct: Vec<RegistryCandidate<(Pair, usize)>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    match resolve_registry_candidates(RuleId::new("pair-shape-registry"), distinct, diagnostics) {
        ParseOutcome::Match(matched) => {
            let (pair, consumed) = matched.value.value;
            Ok(Some(PairGroup {
                remaining: remaining(&pair, consumed),
                feature: matched.value.rule.as_str(),
                pair,
                completed: false,
                first_sentence: sentence_idx,
                consumed: 1,
            }))
        }
        ParseOutcome::NoMatch => Ok(None),
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
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
