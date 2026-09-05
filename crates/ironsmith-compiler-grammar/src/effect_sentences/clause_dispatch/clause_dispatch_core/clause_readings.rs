//! The typed readings of one effect clause — the choice clauses, permissions,
//! restrictions, the trailing-"if" and "unless" shapes, the direct clause
//! shapes — formerly a first-match ladder in `clause_dispatch_core`. What no
//! reading claims goes to the verb-driven structural parse.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// One effect clause, with its leading "instead" and "then" already stripped.
#[path = "clause_readings/part_1.rs"]
mod part_1;
#[path = "clause_readings/part_2.rs"]
mod part_2;
#[path = "clause_readings/part_3.rs"]
mod part_3;
#[path = "clause_readings/part_4.rs"]
mod part_4;

pub(super) struct Clause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl Clause<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = CLAUSE_READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the clause.
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(effect)) => ParseOutcome::matched(effect, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("clause-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test (the diagnoses written before it), and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&Clause<'_>) -> bool,
    read: fn(&Clause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const CLAUSE_REGISTRY: RuleId = RuleId::new("clause-reading-registry");

/// The readings, in the order they were ranked.
const CLAUSE_READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("any-player-or-opponent-may"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_any_player_or_opponent_may(input)),
    },
    Reading {
        id: RuleId::new("any-player-may-sacrifice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_any_player_may_sacrifice(input)),
    },
    Reading {
        id: RuleId::new("assigns-no-combat-damage-then-coordinated"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(part_1::read_assigns_no_combat_damage_then_coordinated(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("conditional-become-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_conditional_become_pair(input)),
    },
    Reading {
        id: RuleId::new("counter-linked-land-subtype-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_counter_linked_land_subtype_followup(input)),
    },
    Reading {
        id: RuleId::new("prevent-damage-sentence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_prevent_damage_sentence(input)),
    },
    Reading {
        id: RuleId::new("heal-damage"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_heal_damage(input)),
    },
    Reading {
        id: RuleId::new("conditional-return-then-turn-face-up"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_conditional_return_then_turn_face_up(input)),
    },
    Reading {
        id: RuleId::new("anaphoric-destroy-battlefield-guard"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_anaphoric_destroy_battlefield_guard(input)),
    },
    Reading {
        id: RuleId::new("trailing-if-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("anaphoric-destroy-battlefield-guard")
                && !input.read_by("conditional-return-then-turn-face-up")
        },
        read: |input| input.outcome(part_1::read_trailing_if_clause(input)),
    },
    Reading {
        id: RuleId::new("may-cast-it"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_may_cast_it(input)),
    },
    Reading {
        id: RuleId::new("play-exiled-cards-for-as-long-as-exiled"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_play_exiled_cards_for_as_long_as_exiled(input)),
    },
    Reading {
        id: RuleId::new("cast-target-from-your-graveyard-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(part_1::read_cast_target_from_your_graveyard_this_turn(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("cast-or-play-tagged"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("may-cast-it")
        },
        read: |input| input.outcome(part_1::read_cast_or_play_tagged(input)),
    },
    Reading {
        id: RuleId::new("cast-any-number-from-among-tagged"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_cast_any_number_from_among_tagged(input)),
    },
    Reading {
        id: RuleId::new("cast-single-spell-from-among-hand-cards"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_cast_single_spell_from_among_hand_cards(input)),
    },
    Reading {
        id: RuleId::new("mana-any-type-cast-tagged-this-way"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_mana_any_type_cast_tagged_this_way(input)),
    },
    Reading {
        id: RuleId::new("leading-may-additional-land-plays"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("cast-or-play-tagged")
                && !input.read_by("may-cast-it")
                && !input.read_by("trailing-if-clause")
                // Readings ranked above this one that read the input read it.
                && !input.read_by("any-player-may-sacrifice")
                && !input.read_by("any-player-or-opponent-may")
        },
        read: |input| input.outcome(part_2::read_leading_may_additional_land_plays(input)),
    },
    Reading {
        id: RuleId::new("tagged-plural-pump"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_tagged_plural_pump(input)),
    },
    Reading {
        id: RuleId::new("for-each-prevent-damage"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_for_each_prevent_damage(input)),
    },
    Reading {
        id: RuleId::new("for-each-counter-group-removed-this-way"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_for_each_counter_group_removed_this_way(input)),
    },
    Reading {
        id: RuleId::new("turn-target-face-up"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_turn_target_face_up(input)),
    },
    Reading {
        id: RuleId::new("direct-clause-shape"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_direct_clause_shape(input)),
    },
    Reading {
        id: RuleId::new("shared-ability-gain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_shared_ability_gain(input)),
    },
    Reading {
        id: RuleId::new("take-extra-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_take_extra_turn(input)),
    },
    Reading {
        id: RuleId::new("additional-phase"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_additional_phase(input)),
    },
    Reading {
        id: RuleId::new("mana-replacement-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_mana_replacement_clause(input)),
    },
    Reading {
        id: RuleId::new("for-each-card-payment"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_2::read_for_each_card_payment(input)),
    },
    Reading {
        id: RuleId::new("opponent-return-choice"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_2::read_opponent_return_choice(input)),
    },
    Reading {
        id: RuleId::new("delayed-next-step-unless-pays"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_2::read_delayed_next_step_unless_pays(input)),
    },
    Reading {
        id: RuleId::new("each-opponent-exiles-card-from-their-hand-or-permanent-they-control"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| {
            input.outcome(
                part_2::read_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(
                    input,
                ),
            )
        },
    },
    Reading {
        id: RuleId::new("clause-primitives"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("conditional-return-then-turn-face-up")
                && !input.read_by("trailing-if-clause")
                // Readings ranked above this one that read the input read it.
                && !input.read_by("may-cast-it")
        },
        read: |input| input.outcome(part_2::read_clause_primitives(input)),
    },
    Reading {
        id: RuleId::new("unless-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("clause-primitives")
        },
        read: |input| input.outcome(part_2::read_unless_clause(input)),
    },
    Reading {
        id: RuleId::new("has-base-power"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_2::read_has_base_power(input)),
    },
    Reading {
        id: RuleId::new("has-base-power-toughness"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_2::read_has_base_power_toughness(input)),
    },
    Reading {
        id: RuleId::new("passive-sacrifice-by-controller"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_2::read_passive_sacrifice_by_controller(input)),
    },
    Reading {
        id: RuleId::new("copular-base-pt-animation"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_2::read_copular_base_pt_animation(input)),
    },
    Reading {
        id: RuleId::new("participant-choice-then-return-chosen-set"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| {
            input.outcome(part_3::read_participant_choice_then_return_chosen_set(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("choose-color"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_choose_color(input)),
    },
    Reading {
        id: RuleId::new("choose-creature-type"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_choose_creature_type(input)),
    },
    Reading {
        id: RuleId::new("choose-land-type"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_choose_land_type(input)),
    },
    Reading {
        id: RuleId::new("choose-subtype-family"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("choose-land-type")
        },
        read: |input| input.outcome(part_3::read_choose_subtype_family(input)),
    },
    Reading {
        id: RuleId::new("choose-card-type"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_choose_card_type(input)),
    },
    Reading {
        id: RuleId::new("choose-player"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_choose_player(input)),
    },
    Reading {
        id: RuleId::new("ordered-choose-all"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_ordered_choose_all(input)),
    },
    Reading {
        id: RuleId::new("choose-target"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_choose_target(input)),
    },
    Reading {
        id: RuleId::new("you-choose-player"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("choose-player")
        },
        read: |input| input.outcome(part_3::read_you_choose_player(input)),
    },
    Reading {
        id: RuleId::new("target-player-choose-objects-with-count"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("choose-creature-type")
        },
        read: |input| input.outcome(part_3::read_target_player_choose_objects_with_count(input)),
    },
    Reading {
        id: RuleId::new("you-choose-objects-with-count"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("choose-card-type")
                && !input.read_by("choose-creature-type")
                && !input.read_by("choose-land-type")
                && !input.read_by("choose-subtype-family")
                && !input.read_by("choose-target")
                && !input.read_by("clause-primitives")
                && !input.read_by("ordered-choose-all")
                && !input.read_by("target-player-choose-objects-with-count")
                && !input.read_by("you-choose-player")
        },
        read: |input| input.outcome(part_3::read_you_choose_objects_with_count(input)),
    },
    Reading {
        id: RuleId::new("assigns-no-combat-damage"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_assigns_no_combat_damage(input)),
    },
    Reading {
        id: RuleId::new("targeted-negated-restriction"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_targeted_negated_restriction(input)),
    },
    Reading {
        id: RuleId::new("target-only"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("clause-primitives")
                && !input.read_by("has-base-power")
                && !input.read_by("target-player-choose-objects-with-count")
        },
        read: |input| input.outcome(part_3::read_target_only(input)),
    },
    Reading {
        id: RuleId::new("embedded-choose-target"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("choose-target")
                && !input.read_by("clause-primitives")
                && !input.read_by("target-player-choose-objects-with-count")
        },
        read: |input| input.outcome(part_3::read_embedded_choose_target(input)),
    },
    Reading {
        id: RuleId::new("next-turn-cant"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_3::read_next_turn_cant(input)),
    },
    Reading {
        id: RuleId::new("restriction-duration-cant"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_4::read_restriction_duration_cant(input)),
    },
    Reading {
        id: RuleId::new("hexproof-targeting-override"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_4::read_hexproof_targeting_override(input)),
    },
    Reading {
        id: RuleId::new("cast-target-without-paying"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_4::read_cast_target_without_paying(input)),
    },
    Reading {
        id: RuleId::new("passive-goad"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_4::read_passive_goad(input)),
    },
    Reading {
        id: RuleId::new("control-player"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
        },
        read: |input| input.outcome(part_4::read_control_player(input)),
    },
    Reading {
        id: RuleId::new("trailing-if-fallback"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            let clause_word_view = ClauseDispatchCompatWords::new(tokens);
            let clause_words = clause_word_view.to_word_refs();
            !(is_mana_replacement_clause_words(&clause_words))
                && !(is_mana_trigger_additional_clause_words(&clause_words))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("clause-primitives")
                && !input.read_by("trailing-if-clause")
        },
        read: |input| input.outcome(part_4::read_trailing_if_fallback(input)),
    },
];

pub(super) fn read_clause(input: &Clause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in CLAUSE_READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<EffectAst>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{CLAUSE_REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(CLAUSE_REGISTRY, distinct, diagnostics);
    match &outcome {
        ParseOutcome::Match(matched) => {
            crate::parse_trace::event(format!(
                "{CLAUSE_REGISTRY}: {} read the input",
                matched.value.rule
            ));
        }
        ParseOutcome::Error(diagnostic) => {
            crate::parse_trace::event(format!("{CLAUSE_REGISTRY}: error: {}", diagnostic.message));
        }
        ParseOutcome::NoMatch => {}
    }
    outcome
}

/// The diagnoses that stood between the readings: a clause no reading claims
/// fails here before the structural parse sees it, as it did in the ladder.
pub(super) fn diagnose(input: &Clause<'_>) -> Result<(), CardTextError> {
    let tokens = input.tokens;
    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}') [rule=mana-replacement]",
            clause_words.join(" ")
        )));
    }
    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}') [rule=mana-trigger-additional]",
            clause_words.join(" ")
        )));
    }
    Ok(())
}
