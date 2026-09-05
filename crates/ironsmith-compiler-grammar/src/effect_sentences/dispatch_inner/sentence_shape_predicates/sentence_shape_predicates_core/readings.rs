//! The readings of one effect sentence: every typed sentence shape the
//! dispatcher knew ("win the game", "deals damage ...", the for-each
//! families, the delayed and conditional shapes, ...), formerly a first-match
//! ladder of 103 rungs. Every admitted reading runs; two readings that
//! disagree are an ambiguity. What no reading claims goes to the chain parser.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
    resolve_registry_candidates,
};

#[path = "readings/part_1.rs"]
mod part_1;
#[path = "readings/part_2.rs"]
mod part_2;
#[path = "readings/part_3.rs"]
mod part_3;
#[path = "readings/part_4.rs"]
mod part_4;
#[path = "readings/part_5.rs"]
mod part_5;

/// One effect sentence, with the composition readings that claim it computed
/// once on demand: the ladder ranked some composition rungs ahead of specific
/// readings, so a specific reading ranked after such a rung never saw a
/// sentence that rung *read* — only its reading, not its head, decides.
pub(super) struct Sentence<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    claims: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl<'a> Sentence<'a> {
    pub(super) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self {
            tokens,
            claims: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    /// Whether the composition reading `id` reads this sentence.
    fn claimed_by(&self, id: &'static str) -> bool {
        if let Some(claimed) = self.claims.borrow().get(id) {
            return *claimed;
        }
        let claimed = SENTENCE_COMPOSITION
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.claims.borrow_mut().insert(id, claimed);
        claimed
    }
}

impl Sentence<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the sentence.
    fn outcome(
        &self,
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(effects)) => ParseOutcome::matched(effects, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("sentence-reading"),
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
    admits: fn(&Sentence<'_>) -> bool,
    read: fn(&Sentence<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const SENTENCE_REGISTRY: RuleId = RuleId::new("sentence-dispatch-registry");
pub(super) const COMPOSITION_REGISTRY: RuleId = RuleId::new("sentence-composition-registry");

/// The readings, in the order they were ranked.
const SENTENCE_READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("win-the-game"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_win_the_game(input)),
    },
    Reading {
        id: RuleId::new("source-and-blocked-creatures-top-library-shuffle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(part_1::read_source_and_blocked_creatures_top_library_shuffle(input))
        },
    },
    Reading {
        id: RuleId::new("deals-damage-word-view"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_deals_damage_word_view(input)),
    },
    Reading {
        id: RuleId::new("becomes-word-view"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_becomes_word_view(input)),
    },
    Reading {
        id: RuleId::new("and-can-attack"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_and_can_attack(input)),
    },
    Reading {
        id: RuleId::new("can-attack-as-though-no-defender"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_can_attack_as_though_no_defender(input)),
    },
    Reading {
        id: RuleId::new("each-prior-affected-object-controller-mana-value-life"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(part_1::read_each_prior_affected_object_controller_mana_value_life(input))
        },
    },
    Reading {
        id: RuleId::new("destroy-attached-object-then-source-damage-to-controller"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(
                part_1::read_destroy_attached_object_then_source_damage_to_controller(input),
            )
        },
    },
    Reading {
        id: RuleId::new("as-you-cast-from-zone-this-turn-grant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_as_you_cast_from_zone_this_turn_grant(input)),
    },
    Reading {
        id: RuleId::new("sentence-delayed-next-step-unless-pays"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_sentence_delayed_next_step_unless_pays(input)),
    },
    Reading {
        id: RuleId::new("attacking-doesnt-tap-if-source-untapped"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_attacking_doesnt_tap_if_source_untapped(input)),
    },
    Reading {
        id: RuleId::new("trailing-if-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_trailing_if_clause(input)),
    },
    Reading {
        id: RuleId::new("each-player-exile-sacrifice-return-exiled"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(part_1::read_each_player_exile_sacrifice_return_exiled(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("may-have-any-number-tagged-phase-out"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_may_have_any_number_tagged_phase_out(input)),
    },
    Reading {
        id: RuleId::new("if-you-dont"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_if_you_dont(input)),
    },
    Reading {
        id: RuleId::new("sentence-damage-unless-controller-has-source-deal-damage"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| {
            input.outcome(
                part_1::read_sentence_damage_unless_controller_has_source_deal_damage(input),
            )
        },
    },
    Reading {
        id: RuleId::new("shared-color-target-fanout"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_1::read_shared_color_target_fanout(input)),
    },
    Reading {
        id: RuleId::new("keyword-bundle-pump"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_1::read_keyword_bundle_pump(input)),
    },
    Reading {
        id: RuleId::new("coordinated-leading-duration"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A shared-color fanout ("target creature and each other creature that shares a color with it ...") is the fanout reader's.
                && !crate::word_primitives::sequence_occurs(&crate::lexer::parser_token_word_refs(tokens), &["shares", "a", "color"])
                // A where-X sentence is read with its binding by the and-split composition.
                && sentence_shapes::parse_where_x_sentence_tokens(tokens).is_none()
        },
        read: |input| input.outcome(part_1::read_coordinated_leading_duration(input)),
    },
    Reading {
        id: RuleId::new("explicit-assign-no-combat-damage-followup"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| {
            input.outcome(part_1::read_explicit_assign_no_combat_damage_followup(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("source-gets-unblockable"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_1::read_source_gets_unblockable(input)),
    },
    Reading {
        id: RuleId::new("target-gets-unblockable"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_1::read_target_gets_unblockable(input)),
    },
    Reading {
        id: RuleId::new("may-cast-it"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_2::read_may_cast_it(input)),
    },
    Reading {
        id: RuleId::new("generic-top-cards-cloak-counted-rest-bottom"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| {
            input.outcome(part_2::read_generic_top_cards_cloak_counted_rest_bottom(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("coordinated-cant-restrictions"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
        },
        read: |input| input.outcome(part_2::read_coordinated_cant_restrictions(input)),
    },
    Reading {
        id: RuleId::new("roll-dice-choose-one-result"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
        },
        read: |input| input.outcome(part_2::read_roll_dice_choose_one_result(input)),
    },
    Reading {
        id: RuleId::new("sentence-delayed-timing-suffix"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
        },
        read: |input| input.outcome(part_2::read_sentence_delayed_timing_suffix(input)),
    },
    Reading {
        id: RuleId::new("until-duration-triggered"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_until_duration_triggered(input)),
    },
    Reading {
        id: RuleId::new("sentence-delayed-trigger-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_sentence_delayed_trigger_this_turn(input)),
    },
    Reading {
        id: RuleId::new("compound-damage-fanout"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_compound_damage_fanout(input)),
    },
    Reading {
        id: RuleId::new("player-villainous-choice"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_player_villainous_choice(input)),
    },
    Reading {
        id: RuleId::new("consult-disposition-bundle"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_consult_disposition_bundle(input)),
    },
    Reading {
        id: RuleId::new("future-zone-replacement"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A delayed "this turn" trigger or a "when that dies this turn" sentence is the delayed reader's.
                && crate::grammar::effects::delayed_sentence_shapes::parse_delayed_this_turn_shape(tokens).is_none() && crate::grammar::effects::delayed_sentence_shapes::parse_delayed_dies_shape(tokens).is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_future_zone_replacement(input)),
    },
    Reading {
        id: RuleId::new("delayed-schedule-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_delayed_schedule_sentence(input)),
    },
    Reading {
        id: RuleId::new("sentence-you-and-attacking-player-each-draw-and-lose"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| {
            input.outcome(part_2::read_sentence_you_and_attacking_player_each_draw_and_lose(input))
        },
    },
    Reading {
        id: RuleId::new("if-any-tagged"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_if_any_tagged(input)),
    },
    Reading {
        id: RuleId::new("if-enters"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_if_enters(input)),
    },
    Reading {
        id: RuleId::new("generic-damage-replacement-counters"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_generic_damage_replacement_counters(input)),
    },
    Reading {
        id: RuleId::new("redirect-next-damage"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_redirect_next_damage(input)),
    },
    Reading {
        id: RuleId::new("prevent-next-time-damage"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_2::read_prevent_next_time_damage(input)),
    },
    Reading {
        id: RuleId::new("choice-complement"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_choice_complement(input)),
    },
    Reading {
        id: RuleId::new("cast-or-play-tagged"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // "You may cast it ..." is the may-cast-it statement's.
                && !(super::super::super::parse_leading_player_may_lexed(tokens).is_some() && parse_may_cast_it_sentence(tokens).is_some())
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_cast_or_play_tagged(input)),
    },
    Reading {
        id: RuleId::new("create-token-then-copy-spell-chain"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_create_token_then_copy_spell_chain(input)),
    },
    Reading {
        id: RuleId::new("copy-spell"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // "If you don't ..." and "if any of ..." copies are the conditional readers'.
                && { let words = crate::lexer::parser_token_word_refs(tokens); !(crate::word_primitives::parse_sequence_prefix(&words, &["if", "you", "dont"]) || crate::word_primitives::parse_sequence_prefix(&words, &["if", "any"])) }
                // A conditional copy ("if ..., copy that spell") is the conditional readers'.
                && { let conditional = if tokens.first().is_some_and(|token| token.is_word("then")) { &tokens[1..] } else { tokens }; !conditional.first().is_some_and(|token| token.is_word("if")) }
                // A copy coordinated with another action ("copy that spell ..., and you may choose new targets") is the coordinated chain's.
                && {
                    let segments = super::super::super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens);
                    !(segments.len() >= 2 && segments.iter().all(|segment| super::super::super::lex_chain_helpers::segment_has_effect_head_lexed(segment)))
                }
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_copy_spell(input)),
    },
    Reading {
        id: RuleId::new("scaled-target-power"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_scaled_target_power(input)),
    },
    Reading {
        id: RuleId::new("next-spell-grant-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_next_spell_grant_sentence(input)),
    },
    Reading {
        id: RuleId::new("matching-spell-cost-reduction"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_matching_spell_cost_reduction(input)),
    },
    Reading {
        id: RuleId::new("manifest-dread-graveyard-card-to-hand"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_manifest_dread_graveyard_card_to_hand(input)),
    },
    Reading {
        id: RuleId::new("spell-cast-this-way-tax"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_spell_cast_this_way_tax(input)),
    },
    Reading {
        id: RuleId::new("attack-or-block-then-prohibition"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_attack_or_block_then_prohibition(input)),
    },
    Reading {
        id: RuleId::new("optional-companion-fanout"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // "... attacks or blocks this combat if able" is the attack-or-block prohibition's.
                && !crate::word_primitives::sequence_occurs(&crate::lexer::parser_token_word_refs(tokens), &["attacks", "or", "blocks"])
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| input.outcome(part_3::read_optional_companion_fanout(input)),
    },
    Reading {
        id: RuleId::new("controller-and-defending-player-discard-or-sacrifice"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
        },
        read: |input| {
            input.outcome(part_3::read_controller_and_defending_player_discard_or_sacrifice(input))
        },
    },
    Reading {
        id: RuleId::new("target-relative-combat-set"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_3::read_target_relative_combat_set(input)),
    },
    Reading {
        id: RuleId::new("conjoined-must-be-blocked"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_3::read_conjoined_must_be_blocked(input)),
    },
    Reading {
        id: RuleId::new("destroy-then-temporary-cant-attack-block-chain"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| {
            input.outcome(part_3::read_destroy_then_temporary_cant_attack_block_chain(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("cant-gain-life-replacement"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_3::read_cant_gain_life_replacement(input)),
    },
    Reading {
        id: RuleId::new("reveal-source-exiled-permanents-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_3::read_reveal_source_exiled_permanents_sentence(input)),
    },
    Reading {
        id: RuleId::new("put-cards-from-single-graveyard-on-bottom-owner-library"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| {
            input.outcome(
                part_3::read_put_cards_from_single_graveyard_on_bottom_owner_library(input),
            )
        },
    },
    Reading {
        id: RuleId::new("vote-affinity"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_3::read_vote_affinity(input)),
    },
    Reading {
        id: RuleId::new("vote"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_3::read_vote(input)),
    },
    Reading {
        id: RuleId::new("keyword-mechanic"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_3::read_keyword_mechanic(input)),
    },
    Reading {
        id: RuleId::new("for-each-counter-removed"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_counter_removed(input)),
    },
    Reading {
        id: RuleId::new("for-each-counter-group-removed-this-way"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_counter_group_removed_this_way(input)),
    },
    Reading {
        id: RuleId::new("for-each-prevent-damage"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_prevent_damage(input)),
    },
    Reading {
        id: RuleId::new("for-each-destroyed-this-way"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_destroyed_this_way(input)),
    },
    Reading {
        id: RuleId::new("for-each-sacrificed-this-way"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_sacrificed_this_way(input)),
    },
    Reading {
        id: RuleId::new("for-each-put-into-graveyard-this-way"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_put_into_graveyard_this_way(input)),
    },
    Reading {
        id: RuleId::new("for-each-exiled-this-way"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_exiled_this_way(input)),
    },
    Reading {
        id: RuleId::new("each-chosen-player-search-put-top"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_each_chosen_player_search_put_top(input)),
    },
    Reading {
        id: RuleId::new("for-each-mana-symbol-spent-effect"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_mana_symbol_spent_effect(input)),
    },
    Reading {
        id: RuleId::new("for-each-spent-mana-effect"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_spent_mana_effect(input)),
    },
    Reading {
        id: RuleId::new("for-each-dynamic-target-effect"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
        },
        read: |input| input.outcome(part_4::read_for_each_dynamic_target_effect(input)),
    },
    Reading {
        id: RuleId::new("delayed-until-next-end-step"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A delayed schedule sentence is the schedule reader's.
                && effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens).is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
        },
        read: |input| input.outcome(part_4::read_delayed_until_next_end_step(input)),
    },
    Reading {
        id: RuleId::new("delayed-next-combat-phase-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
        },
        read: |input| input.outcome(part_4::read_delayed_next_combat_phase_this_turn(input)),
    },
    Reading {
        id: RuleId::new("it-is-aura-enchantment-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
        },
        read: |input| input.outcome(part_4::read_it_is_aura_enchantment_sentence(input)),
    },
    Reading {
        id: RuleId::new("quoted-ability-shared-color-fanout"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
        },
        read: |input| input.outcome(part_4::read_quoted_ability_shared_color_fanout(input)),
    },
    Reading {
        id: RuleId::new("quoted-ability-conditional"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // "It's an Aura enchantment with ..." is the aura reader's.
                && effect_grammar::sentence_predicate_shapes::parse_aura_enchantment_tokens(tokens).is_none()
                // A shared-color fanout ("... each other creature that shares a color with it") is the fanout reader's.
                && !crate::word_primitives::sequence_occurs(&crate::lexer::parser_token_word_refs(tokens), &["shares", "a", "color"])
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_4::read_quoted_ability_conditional(input)),
    },
    Reading {
        id: RuleId::new("source-tapped-gain-duration"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_4::read_source_tapped_gain_duration(input)),
    },
    Reading {
        id: RuleId::new("immediate-sacrifice-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A sacrifice with a delayed timing suffix is the delayed statement's.
                && !crate::grammar::effects::delayed_step_shapes::parse_delayed_timing_marker_shape(tokens).is_some_and(|marker| marker.start_word != 0)
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_4::read_immediate_sacrifice_sentence(input)),
    },
    Reading {
        id: RuleId::new("end-of-combat-remainder"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_4::read_end_of_combat_remainder(input)),
    },
    Reading {
        id: RuleId::new("additional-phase"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_5::read_additional_phase(input)),
    },
    Reading {
        id: RuleId::new("triggering-object-had-counters-create"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_5::read_triggering_object_had_counters_create(input)),
    },
    Reading {
        id: RuleId::new(
            "sentence-each-player-reveals-top-count-put-permanents-onto-battlefield-rest-graveyard",
        ),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| {
            input.outcome(part_5::read_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(input))
        },
    },
    Reading {
        id: RuleId::new("consult-traversal-with-inline-followup"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
                // A consult with a disposition bundle is the bundle's.
                && super::super::super::bundle_rules::parse_consult_disposition_bundle(tokens).is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_5::read_consult_traversal_with_inline_followup(input)),
    },
    Reading {
        id: RuleId::new("where-x-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
                // "If you do, ... where X is ..." is the result-prefix reading's.
                && split_leading_result_prefix_lexed(tokens).is_none()
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_5::read_where_x_sentence(input)),
    },
    Reading {
        id: RuleId::new("gain-ability"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_5::read_gain_ability(input)),
    },
    Reading {
        id: RuleId::new("exile-then-return-same-object"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| input.outcome(part_5::read_exile_then_return_same_object(input)),
    },
    Reading {
        id: RuleId::new("generic-top-cards-exile-counted-face-down-rest-bottom"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| {
            input.outcome(part_5::read_generic_top_cards_exile_counted_face_down_rest_bottom(input))
        },
    },
    Reading {
        id: RuleId::new("generic-each-player-exile-top-then-cast-any-number"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
                // A coordinated chain of explicit actions is the and-split composition's.
               && !input.claimed_by("explicit-action-segments")
                // A result-prefixed sentence ("if you do, ...") is the result-prefix composition's.
               && !input.claimed_by("leading-result-prefix")
                // "<player> may ..." is the player-may composition's.
               && !input.claimed_by("leading-player-may")
                // Explicit player-subject clauses ("each opponent ... and you ...") are the player-subject split's.
               && !input.claimed_by("explicit-player-subject-clauses")
                // A for-each-object sentence is the for-each composition's.
               && !input.claimed_by("for-each-object-filter-effect")
                // A quoted grant under "<player> may" is the player-may composition's.
               && !input.claimed_by("quoted-ability-leading-may")
        },
        read: |input| {
            input.outcome(part_5::read_generic_each_player_exile_top_then_cast_any_number(input))
        },
    },
];

/// The composition readers: the general grammar over the chain parser
/// (coordinated "and" segments, ", then" boundaries, "if you do," result
/// prefixes, the conditional families, for-each loops, the top-level
/// subject/verb reading). They read what no specific reading claims, in
/// this order; the overlaps among them are measured toward making the
/// composition itself structural.
const SENTENCE_COMPOSITION: &[Reading] = &[
    Reading {
        id: RuleId::new("explicit-action-segments"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_2::read_explicit_action_segments(input)),
    },
    Reading {
        id: RuleId::new("leading-result-prefix"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_2::read_leading_result_prefix(input)),
    },
    Reading {
        id: RuleId::new("leading-player-may"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_2::read_leading_player_may(input)),
    },
    Reading {
        id: RuleId::new("conditional-sentence-family"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_2::read_conditional_sentence_family(input)),
    },
    Reading {
        id: RuleId::new("explicit-player-subject-clauses"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_3::read_explicit_player_subject_clauses(input)),
    },
    Reading {
        id: RuleId::new("for-each-object-effect"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_4::read_for_each_object_effect(input)),
    },
    Reading {
        id: RuleId::new("for-each-object-filter-effect"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_4::read_for_each_object_filter_effect(input)),
    },
    Reading {
        id: RuleId::new("quoted-ability-leading-may"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_4::read_quoted_ability_leading_may(input)),
    },
    Reading {
        id: RuleId::new("leading-if-conditional"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
        },
        read: |input| input.outcome(part_5::read_leading_if_conditional(input)),
    },
    Reading {
        id: RuleId::new("explicit-comma-then-boundary"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
        },
        read: |input| input.outcome(part_5::read_explicit_comma_then_boundary(input)),
    },
    Reading {
        id: RuleId::new("put-verb-dispatch"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
        },
        read: |input| input.outcome(part_5::read_put_verb_dispatch(input)),
    },
    Reading {
        id: RuleId::new("for-each-target-players"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
        },
        read: |input| input.outcome(part_5::read_for_each_target_players(input)),
    },
    Reading {
        id: RuleId::new("or-action-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
        },
        read: |input| input.outcome(part_5::read_or_action_clause(input)),
    },
    Reading {
        id: RuleId::new("top-level-subject-verb-recognition"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            (super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens))
                .is_none()
                && !(has_unrecognized_leading_effect_label(tokens))
        },
        read: |input| input.outcome(part_5::read_top_level_subject_verb_recognition(input)),
    },
];

/// The sentence's reading: a specific reading if one claims it, else the
/// first composition reader that does.
pub(super) fn read_sentence(input: &Sentence<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    match collect(
        SENTENCE_REGISTRY,
        SENTENCE_READINGS,
        input,
        Resolution::Strict,
    ) {
        ParseOutcome::Match(matched) => ParseOutcome::Match(matched),
        ParseOutcome::NoMatch => collect(
            COMPOSITION_REGISTRY,
            SENTENCE_COMPOSITION,
            input,
            Resolution::Ranked,
        ),
        // A specific reading's committed error stands only when composition
        // has no reading either: the general grammar is the rest of the
        // language, not a competitor the error outranks.
        ParseOutcome::Error(specific) => {
            match collect(
                COMPOSITION_REGISTRY,
                SENTENCE_COMPOSITION,
                input,
                Resolution::Ranked,
            ) {
                ParseOutcome::NoMatch => ParseOutcome::Error(specific),
                outcome => outcome,
            }
        }
    }
}

/// How a tier resolves several readings of one sentence: the specific
/// readings must agree; the composition readers keep their order while the
/// overlaps among them are measured.
#[derive(Clone, Copy)]
enum Resolution {
    Strict,
    Ranked,
}

fn collect(
    registry: RuleId,
    readings: &[Reading],
    input: &Sentence<'_>,
    resolution: Resolution,
) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in readings {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        let outcome = (reading.read)(input).within(reading.id);
        match outcome {
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
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{registry}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = match resolution {
        Resolution::Strict => resolve_registry_candidates(registry, distinct, diagnostics),
        Resolution::Ranked => resolve_ranked_candidates(registry, distinct, diagnostics, || {
            crate::lexer::parser_token_word_refs(input.tokens).join(" ")
        }),
    };
    match &outcome {
        ParseOutcome::Match(matched) => {
            crate::parse_trace::event(format!(
                "{registry}: {} read the sentence",
                matched.value.rule
            ));
        }
        ParseOutcome::Error(diagnostic) => {
            crate::parse_trace::event(format!("{registry}: error: {}", diagnostic.message));
        }
        ParseOutcome::NoMatch => {}
    }
    outcome
}

/// The diagnoses that stood between the readings: a sentence no reading
/// claims fails here before the chain parser sees it, as it did in the ladder.
pub(super) fn diagnose(input: &Sentence<'_>) -> Result<(), CardTextError> {
    let tokens = input.tokens;
    if let Some(diag) =
        super::super::super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens)
    {
        return Err(diag);
    }
    if has_unrecognized_leading_effect_label(tokens) {
        return Err(CardTextError::ParseError(
            "unknown labeled effect prefix".to_string(),
        ));
    }
    Ok(())
}
