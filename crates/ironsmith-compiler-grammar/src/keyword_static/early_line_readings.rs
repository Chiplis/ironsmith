//! The readings of one static ability line before the indexed static-rule
//! registry sees it: the markers, the compound ability-removal shapes, the
//! quoted grants, the fixed rule-text lines (companion, the legend rule, ...).
//! Formerly a first-match ladder in `keyword_static`; every reading runs,
//! resolved by rank while the overlaps are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct EarlyLine<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl EarlyLine<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<Vec<StaticAbilityAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<StaticAbilityAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("early-static-line-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&EarlyLine<'_>) -> bool,
    read: fn(&EarlyLine<'_>) -> ParseOutcome<Vec<StaticAbilityAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("early-static-line-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("companion-ability"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_companion_ability(input)),
    },
    Reading {
        id: RuleId::new("keyword-or-ticket-marker"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_keyword_or_ticket_marker(input)),
    },
    Reading {
        id: RuleId::new("static-effect-continues-until-end-of-turn-surface"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_static_effect_continues_until_end_of_turn_surface(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("source-characteristics-of-last-exiled-creature-card-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_source_characteristics_of_last_exiled_creature_card_line(input))
        },
    },
    Reading {
        id: RuleId::new("enter-as-copy-as-enters-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_enter_as_copy_as_enters_line(input)),
    },
    Reading {
        id: RuleId::new("early-static-marker"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_early_static_marker(input)),
    },
    Reading {
        id: RuleId::new("reveal-first-card-you-draw-each-turn-spec"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_reveal_first_card_you_draw_each_turn_spec(input)),
    },
    Reading {
        id: RuleId::new("can-block-additional-creature-each-combat-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_can_block_additional_creature_each_combat_line(input)),
    },
    Reading {
        id: RuleId::new("count-as-card-named-for-spell-effect-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_count_as_card_named_for_spell_effect_line(input)),
    },
    Reading {
        id: RuleId::new("lose-all-abilities-and-doesnt-untap-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_lose_all_abilities_and_doesnt_untap_line(input)),
    },
    Reading {
        id: RuleId::new("lose-all-abilities-and-transform-base-pt-line"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("lose-all-abilities-and-doesnt-untap-line")
        },
        read: |input| input.outcome(read_lose_all_abilities_and_transform_base_pt_line(input)),
    },
    Reading {
        id: RuleId::new("lose-all-abilities-and-base-pt-line"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("lose-all-abilities-and-doesnt-untap-line")
                && !input.read_by("lose-all-abilities-and-transform-base-pt-line")
        },
        read: |input| input.outcome(read_lose_all_abilities_and_base_pt_line(input)),
    },
    Reading {
        id: RuleId::new("minimum-spell-total-mana-three-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_minimum_spell_total_mana_three_line(input)),
    },
    Reading {
        id: RuleId::new("players-cant-pay-life-or-sacrifice-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_players_cant_pay_life_or_sacrifice_line(input)),
    },
    Reading {
        id: RuleId::new("krrik-black-mana-life-payment-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_krrik_black_mana_life_payment_line(input)),
    },
    Reading {
        id: RuleId::new("cycling-cost-alternative-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cycling_cost_alternative_line(input)),
    },
    Reading {
        id: RuleId::new("quoted-granted-ability-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quoted_granted_ability_line(input)),
    },
    Reading {
        id: RuleId::new("spell-and-player-activated-ability-cost-modifier-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_spell_and_player_activated_ability_cost_modifier_line(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("untap-each-other-players-untap-step-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_untap_each_other_players_untap_step_line(input)),
    },
    Reading {
        id: RuleId::new("activated-abilities-cant-be-activated-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activated_abilities_cant_be_activated_line(input)),
    },
    Reading {
        id: RuleId::new("if-this-spell-costs-less-to-cast-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_if_this_spell_costs_less_to_cast_line(input)),
    },
    Reading {
        id: RuleId::new("legend-rule-doesnt-apply-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_legend_rule_doesnt_apply_line(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &EarlyLine<'_>) -> ParseOutcome<RuleMatch<Vec<StaticAbilityAst>>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
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
    let mut distinct: Vec<RegistryCandidate<Vec<StaticAbilityAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_companion_ability(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_companion_ability(tokens) {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_keyword_or_ticket_marker(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if supported_keyword_marker_tokens(tokens) || is_ticket_sticker_marker_tokens(tokens) {
        return Ok(Some(vec![keyword_static_marker(tokens).into()]));
    }
    Ok(None)
}
fn read_static_effect_continues_until_end_of_turn_surface(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    let marker_text = render_token_slice(tokens);
    if document_grammar::parse_static_effect_continues_until_end_of_turn_surface(tokens).is_some() {
        return Ok(Some(vec![
            StaticAbility::keyword_marker(marker_text).into(),
        ]));
    }
    Ok(None)
}
fn read_source_characteristics_of_last_exiled_creature_card_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_source_characteristics_of_last_exiled_creature_card_line(tokens) {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_enter_as_copy_as_enters_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_enter_as_copy_as_enters_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_early_static_marker(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(marker) = keyword_static_lines::parse_early_static_marker_tokens(tokens) {
        let ability = match marker {
            keyword_static_lines::EarlyStaticMarkerKind::XMaximumPlayerCount => {
                StaticAbility::this_spell_x_maximum(
                    Value::CountPlayers(PlayerFilter::Any),
                    "X can't be greater than the number of players in the game.",
                )
            }
            keyword_static_lines::EarlyStaticMarkerKind::XMinimumOne => {
                StaticAbility::this_spell_x_minimum(Value::Fixed(1), "X can't be 0.")
            }
            keyword_static_lines::EarlyStaticMarkerKind::ExhaustAsUnactivated => {
                StaticAbility::exhaust_abilities_as_though_unactivated_this_turn()
            }
            keyword_static_lines::EarlyStaticMarkerKind::CantAttackWithoutCreatureSpell => {
                StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn()
            }
            keyword_static_lines::EarlyStaticMarkerKind::CantAttackWithoutNoncreatureSpell => {
                StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn()
            }
            keyword_static_lines::EarlyStaticMarkerKind::DayNightStartsDay => {
                StaticAbility::day_night_starts_day_as_enters()
            }
            keyword_static_lines::EarlyStaticMarkerKind::LivingMetal => {
                StaticAbility::living_metal()
            }
            keyword_static_lines::EarlyStaticMarkerKind::VehicleRulesMarker => {
                keyword_static_marker(tokens)
            }
        };
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_reveal_first_card_you_draw_each_turn_spec(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(spec) = parse_reveal_first_card_you_draw_each_turn_spec_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::reveal_first_card_you_draw_each_turn(
                spec.optional,
                spec.your_turns_only,
            )
            .into(),
        ]));
    }
    Ok(None)
}
fn read_can_block_additional_creature_each_combat_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_can_block_additional_creature_each_combat_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_count_as_card_named_for_spell_effect_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_count_as_card_named_for_spell_effect_line(tokens) {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_lose_all_abilities_and_doesnt_untap_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // Route compound ability-removal shapes before the indexed registry can
    // accept their leading "lose all abilities" clause as a complete,
    // narrower removal effect and discard the remaining characteristic
    // changes.
    if let Some(abilities) = parse_lose_all_abilities_and_doesnt_untap_line(tokens)? {
        return Ok(Some(
            abilities.into_iter().map(StaticAbilityAst::from).collect(),
        ));
    }
    Ok(None)
}
fn read_lose_all_abilities_and_transform_base_pt_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_lose_all_abilities_and_transform_base_pt_line(tokens)? {
        return Ok(Some(
            abilities.into_iter().map(StaticAbilityAst::from).collect(),
        ));
    }
    Ok(None)
}
fn read_lose_all_abilities_and_base_pt_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_lose_all_abilities_and_base_pt_line(tokens)? {
        return Ok(Some(
            abilities.into_iter().map(StaticAbilityAst::from).collect(),
        ));
    }
    Ok(None)
}
fn read_minimum_spell_total_mana_three_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if is_minimum_spell_total_mana_three_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::minimum_spell_total_mana(3).into(),
        ]));
    }
    Ok(None)
}
fn read_players_cant_pay_life_or_sacrifice_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if is_players_cant_pay_life_or_sacrifice_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
        ]));
    }
    Ok(None)
}
fn read_krrik_black_mana_life_payment_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if is_krrik_black_mana_life_payment_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::krrik_black_mana_may_be_paid_with_life().into(),
        ]));
    }
    Ok(None)
}
fn read_cycling_cost_alternative_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_cycling_cost_alternative_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_quoted_granted_ability_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // A quoted ability belongs to the filtered subject before the quote. The
    // broad spell/activated-cost parser deliberately scans for a nested
    // "Spells ... cost" clause, so let the typed grant route bind that inner
    // static ability to its affected objects first.
    // A compound animation owns both the leading P/T/type descriptor and
    // its quoted granted trigger. The broad quoted-grant parser treats
    // everything before `has` as a filter; on Bello-style text that
    // misreads `is a 4/4 ... creature` as an unsupported descriptor.
    // Attached combat restrictions can precede the quoted grant. Route
    // that coordinated shape before the broad `has "<ability>"` parser
    // treats the restriction prefix as an anthem subject.
    if contains_token_kind(tokens, TokenKind::Quote) {
        if let Some(abilities) = parse_filter_is_pt_creature_in_addition_and_has_line(tokens)? {
            return Ok(Some(abilities));
        }
        if let Some(abilities) = parse_attached_restriction_and_granted_ability_line(tokens)? {
            return Ok(Some(abilities));
        }
        if let Some(abilities) = parse_filter_has_granted_ability_line(tokens)? {
            return Ok(Some(abilities));
        }
    }
    Ok(None)
}
fn read_spell_and_player_activated_ability_cost_modifier_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(tokens)? {
        return Ok(Some(
            abilities.into_iter().map(StaticAbilityAst::from).collect(),
        ));
    }
    Ok(None)
}
fn read_untap_each_other_players_untap_step_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) {
        let subject_tokens = trim_commas(spec.subject_tokens);
        let filter = parse_object_filter(&subject_tokens, false)?;
        let subject_text = crate::lexer::token_word_refs(&subject_tokens).join(" ");
        let display = if spec.untap_all {
            format!("Untap all {subject_text} during each other player's untap step")
        } else {
            format!("Untap {subject_text} during each other player's untap step")
        };
        return Ok(Some(vec![
            StaticAbility::untap_during_each_other_players_untap_step(filter, display).into(),
        ]));
    }
    Ok(None)
}
fn read_activated_abilities_cant_be_activated_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_activated_abilities_cant_be_activated_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_if_this_spell_costs_less_to_cast_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
fn read_legend_rule_doesnt_apply_line(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = parse_legend_rule_doesnt_apply_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    Ok(None)
}
