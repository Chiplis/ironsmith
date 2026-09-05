use super::*;
use crate::grammar::activation_costs::cant_shapes::{
    self, AttackUnlessScope, AttackUnlessSurface, BlockingCantFact, CantFallbackFact,
    DirectCantFact, ManaValueParityCantFact,
};

enum StaticAbilityShapeResolution {
    Ability(StaticAbility),
    Decline,
}

fn direct_cant_static_ability(tokens: &[OwnedLexToken]) -> Option<StaticAbilityShapeResolution> {
    if let Some(fact) = cant_shapes::parse_counter_limit_fact_tokens(tokens) {
        return Some(StaticAbilityShapeResolution::Ability(
            StaticAbility::counter_limit(
                fact.counter_type,
                fact.maximum,
                format_negated_restriction_display(tokens),
            ),
        ));
    }
    let fact = cant_shapes::parse_direct_cant_fact_tokens(tokens)?;
    let authored_self_surface = matches!(
        fact,
        DirectCantFact::SourceCantAttack
            | DirectCantFact::SourceCantBlock
            | DirectCantFact::SourceCantAttackItsOwner
            | DirectCantFact::SourceCantBeBlocked
            | DirectCantFact::SourceCantAttackAlone
            | DirectCantFact::SourceCantAttackOrBlock
            | DirectCantFact::SourceCantAttackOrBlockAlone
            | DirectCantFact::SourceCantAttackOrBlockUnlessMaxSpeed
    )
    .then(|| {
        crate::slice_primitives::select_position(tokens, |token| {
            token.is_word("can't") || token.is_word("cant") || token.is_word("cannot")
        })
        .and_then(|cant| {
            let subject_words = words(&tokens[..cant]);
            source_reference_surface_for_words(&subject_words)
        })
    })
    .flatten();
    let mut ability = match fact {
        DirectCantFact::TemporaryUnblockable => return Some(StaticAbilityShapeResolution::Decline),
        DirectCantFact::PlayerWouldGainNoLifeInstead => StaticAbility::restriction(
            crate::effect::Restriction::gain_life(PlayerFilter::Any),
            "If a player would gain life, that player gains no life instead".to_string(),
        ),
        DirectCantFact::PlayersCantGainLife => StaticAbility::players_cant_gain_life(),
        DirectCantFact::PlayersCantSearchLibraries => StaticAbility::players_cant_search(),
        DirectCantFact::DamageCantBePrevented => StaticAbility::damage_cant_be_prevented(),
        DirectCantFact::YouCantLoseGame => StaticAbility::you_cant_lose_game(),
        DirectCantFact::OpponentsCantWinGame => StaticAbility::opponents_cant_win_game(),
        DirectCantFact::YourLifeTotalCantChange => StaticAbility::your_life_total_cant_change(),
        DirectCantFact::OpponentsCantCastSpells => StaticAbility::opponents_cant_cast_spells(),
        DirectCantFact::OpponentsCantDrawExtraCards => {
            StaticAbility::opponents_cant_draw_extra_cards()
        }
        DirectCantFact::CantHaveCountersPlaced => StaticAbility::cant_have_counters_placed(),
        DirectCantFact::ThisSpellCantBeCountered => StaticAbility::cant_be_countered_ability(),
        DirectCantFact::SourceCantAttack => StaticAbility::cant_attack(),
        DirectCantFact::SourceCantBlock => StaticAbility::cant_block(),
        DirectCantFact::SourceCantAttackItsOwner => StaticAbility::cant_attack_its_owner(),
        DirectCantFact::PermanentsYouControlCantBeSacrificed => {
            StaticAbility::permanents_you_control_cant_be_sacrificed()
        }
        DirectCantFact::SourceCantBeBlocked => StaticAbility::unblockable(),
        DirectCantFact::SourceCantAttackAlone => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            format_negated_restriction_display(tokens),
        ),
        DirectCantFact::SourceCantAttackOrBlock => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            format_negated_restriction_display(tokens),
        ),
        DirectCantFact::SourceCantAttackOrBlockAlone => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
            format_negated_restriction_display(tokens),
        ),
        DirectCantFact::SourceCantAttackOrBlockUnlessMaxSpeed => {
            let max_speed = PredicateAst::ValueComparison {
                left: crate::effect::Value::Speed(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(4),
            };
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                "This creature can't attack or block".to_string(),
            )
            .with_condition(PredicateAst::Not(Box::new(max_speed)))
        }
        DirectCantFact::DomainAttackTax => {
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control()
        }
    };
    if let Some(surface) = authored_self_surface {
        ability = ability.with_self_subject_surface(surface);
    }
    Some(StaticAbilityShapeResolution::Ability(ability))
}

fn blocking_cant_static_ability(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    let fact = cant_shapes::parse_blocking_cant_fact_tokens(tokens)?;
    let display = format_negated_restriction_display(tokens);
    Some(match fact {
        BlockingCantFact::MaximumBlockers {
            maximum_blockers, ..
        } => StaticAbility::cant_be_blocked_by_more_than(maximum_blockers),
        BlockingCantFact::PowerThreshold { comparison, .. } => match comparison {
            crate::filter::Comparison::LessThanOrEqual(threshold) => {
                StaticAbility::cant_be_blocked_by_power_or_less(threshold)
            }
            crate::filter::Comparison::GreaterThanOrEqual(threshold) => {
                StaticAbility::cant_be_blocked_by_power_or_greater(threshold)
            }
            _ => return None,
        },
        BlockingCantFact::DisallowedBlockers { filter, .. } => StaticAbility::restriction(
            crate::effect::Restriction::block_specific_attacker(filter, ObjectFilter::source()),
            display,
        ),
        BlockingCantFact::MinimumBlockers {
            minimum_blockers, ..
        } => StaticAbility::cant_be_blocked_except_by_n_or_more(minimum_blockers),
    })
}

fn attack_unless_static_ability(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    let fact = cant_shapes::parse_attack_unless_condition_tokens(tokens)?;
    let display = format_negated_restriction_display(fact.display_tokens);
    match fact.scope {
        AttackUnlessScope::Attack => Some(match fact.surface {
            AttackUnlessSurface::ControllerCastCreatureSpellThisTurn => {
                StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn()
            }
            AttackUnlessSurface::ControllerCastNoncreatureSpellThisTurn => {
                StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn()
            }
            _ => StaticAbility::cant_attack_unless_condition(fact.condition, display),
        }),
        AttackUnlessScope::AttackOrBlock => {
            let crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(condition) =
                fact.condition
            else {
                return None;
            };
            let condition = PredicateAst::Not(Box::new(condition));
            Some(
                StaticAbility::restriction(
                    crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                    display,
                )
                .with_condition(condition)
                .unwrap_or_else(|| {
                    StaticAbility::restriction(
                        crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                        format_negated_restriction_display(tokens),
                    )
                }),
            )
        }
    }
}

fn parity_cant_static_ability(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    let fact = cant_shapes::parse_mana_value_parity_cant_fact_tokens(tokens)?;
    let display = format_negated_restriction_display(tokens);
    Some(match fact {
        ManaValueParityCantFact::OpponentsCantCastSpells(parity) => StaticAbility::restriction(
            crate::effect::Restriction::cast_spells_matching(
                PlayerFilter::Opponent,
                ObjectFilter::spell().with_mana_value_parity(parity),
            ),
            display,
        ),
        ManaValueParityCantFact::OpponentsCantBlockWithCreatures(parity) => {
            StaticAbility::restriction(
                crate::effect::Restriction::block(
                    ObjectFilter::creature()
                        .opponent_controls()
                        .with_mana_value_parity(parity),
                ),
                display,
            )
        }
    })
}

fn fallback_cant_static_ability(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    match cant_shapes::parse_cant_fallback_fact_tokens(tokens)? {
        CantFallbackFact::SourceCantAttackOrBlockUnlessEvenCounters => Some(
            StaticAbility::rule_fallback_text(format_negated_restriction_display(tokens)),
        ),
        // Parity-scoped damage doubling lowers through the functional
        // DoubleDamageAmountReplacement rule in the keyword-static family
        // (parse_damage_doubling_mana_value_marker_line tries it first).
        CantFallbackFact::SourceDamageDoubledForManaValueParity(_) => None,
    }
}

fn strip_per_blocking_creature_tail(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    for (index, token) in tokens.iter().enumerate() {
        if !token.is_word("for") {
            continue;
        }
        let tail = crate::lexer::token_word_refs(&tokens[index..]);
        if matches!(
            tail.as_slice(),
            ["for", "each", "of", "those", "creatures"]
                | ["for", "each", "blocking", "creature", "they", "control"]
                | ["for", "each", "blocking", "creature"]
        ) {
            return &tokens[..index];
        }
    }
    tokens
}

/// Parse a CR 509.1d blocking cost before the generic negated-restriction
/// family can incorrectly turn it into an unconditional prohibition.
fn block_cost_static_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(cant_index) = crate::slice_primitives::select_position(tokens, |token| {
        token.is_word("can't") || token.is_word("cant") || token.is_word("cannot")
    }) else {
        return Ok(None);
    };
    let Some(unless_index) =
        crate::slice_primitives::select_position(&tokens[cant_index + 1..], |token| {
            token.is_word("unless")
        })
        .map(|idx| idx + cant_index + 1)
    else {
        return Ok(None);
    };

    let subject_words = crate::lexer::token_word_refs(&tokens[..cant_index]);
    let (blockers, blocker_is_attached_to_source) =
        if crate::word_primitives::parse_any_sequence_complete(
            &subject_words,
            &[&["this"], &["this", "creature"]],
        ) {
            (ObjectFilter::source(), false)
        } else if crate::word_primitives::parse_sequence_complete(&subject_words, &["creatures"]) {
            (ObjectFilter::creature(), false)
        } else if crate::word_primitives::parse_any_sequence_complete(
            &subject_words,
            &[&["enchanted", "creature"], &["equipped", "creature"]],
        ) {
            (ObjectFilter::creature(), true)
        } else {
            return Ok(None);
        };

    let action_tokens = trim_edge_punctuation_tokens(&tokens[cant_index + 1..unless_index]);
    let action_words = crate::lexer::token_word_refs(action_tokens);
    let attackers = if crate::word_primitives::parse_any_sequence_complete(
        &action_words,
        &[&["block"], &["attack", "or", "block"]],
    ) {
        // A number of cards use the combined restriction wording even when
        // the declaration-time cost is the CR 509.1d blocking cost.  Keep the
        // typed BlockCost lowering for that shared wording; the attack-side
        // restriction is handled by the ordinary cant/attack parser when it
        // has its own semantics.
        ObjectFilter::creature()
    } else if crate::word_primitives::parse_sequence_prefix(&action_words, &["block"]) {
        let Some(block_token_index) =
            crate::slice_primitives::select_position(action_tokens, |token| token.is_word("block"))
        else {
            return Ok(None);
        };
        let attacker_tokens = trim_edge_punctuation_tokens(&action_tokens[block_token_index + 1..]);
        parse_subject_object_filter(attacker_tokens)?
            .or_else(|| {
                crate::grammar::primitives::probe_shape(parse_object_filter(attacker_tokens, false))
            })
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported attacker filter for blocking cost (clause: '{}')",
                    crate::lexer::token_word_refs(attacker_tokens).join(" ")
                ))
            })?
    } else {
        return Ok(None);
    };

    let payment_tokens = trim_edge_punctuation_tokens(&tokens[unless_index + 1..]);
    let direct_action_cost = payment_tokens
        .first()
        .is_some_and(|token| token.is_word("you"))
        && crate::slice_primitives::select_position(payment_tokens, |token| {
            token.is_word("pay") || token.is_word("pays")
        })
        .is_none();
    let mut cost_tokens = if direct_action_cost {
        // Some declaration costs state the action directly ("unless you tap
        // ...") rather than introducing it with "pay". Feed the ordinary
        // activation-cost grammar the action after the payer pronoun.
        trim_edge_punctuation_tokens(&payment_tokens[1..])
    } else {
        let Some(pay_index) = crate::slice_primitives::select_position(payment_tokens, |token| {
            token.is_word("pay") || token.is_word("pays")
        }) else {
            return Ok(None);
        };
        let payer_words = crate::lexer::token_word_refs(&payment_tokens[..pay_index]);
        if !crate::word_primitives::parse_any_sequence_complete(
            &payer_words,
            &[&["you"], &["its", "controller"], &["their", "controller"]],
        ) {
            return Ok(None);
        }
        trim_edge_punctuation_tokens(&payment_tokens[pay_index.saturating_add(1)..])
    };
    if crate::word_primitives::parse_sequence_complete(&subject_words, &["creatures"]) {
        cost_tokens = trim_edge_punctuation_tokens(strip_per_blocking_creature_tail(cost_tokens));
    }
    let parsed_cost = if direct_action_cost {
        Some(parse_compiler_activation_cost(cost_tokens)?)
    } else {
        parse_payment_clause_as_total_cost(cost_tokens)?
    };
    let Some(cost) = parsed_cost else {
        return Err(CardTextError::ParseError(format!(
            "unsupported blocking payment cost (clause: '{}')",
            crate::lexer::token_word_refs(cost_tokens).join(" ")
        )));
    };

    let display = format_negated_restriction_display(tokens);
    Ok(Some(if blocker_is_attached_to_source {
        StaticAbility::attached_block_cost(blockers, attackers, cost, display)
    } else {
        StaticAbility::block_cost(blockers, attackers, cost, display)
    }))
}

fn except_for_cant_attack_static_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("except"))
        || !tokens.get(1).is_some_and(|token| token.is_word("for"))
    {
        return Ok(None);
    }
    let Some(comma_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Comma)
    else {
        return Ok(None);
    };
    let exception_tokens = trim_edge_punctuation_tokens(&tokens[2..comma_idx]);
    let restriction_tokens = trim_edge_punctuation_tokens(&tokens[comma_idx + 1..]);
    let Some(parsed) = parse_negated_object_restriction_clause(restriction_tokens)? else {
        return Ok(None);
    };
    if parsed.target.is_some() {
        return Ok(None);
    }
    let crate::effect::Restriction::Attack(mut affected) = parsed.restriction else {
        return Ok(None);
    };

    let mut exception_filters = None;
    // Prefer the final conjunction so a card name that itself contains
    // "and" remains one named exception.
    for (and_idx, token) in exception_tokens.iter().enumerate().rev() {
        if !token.is_word("and") {
            continue;
        }
        let left = trim_edge_punctuation_tokens(&exception_tokens[..and_idx]);
        let right = trim_edge_punctuation_tokens(&exception_tokens[and_idx + 1..]);
        if left.is_empty() || right.is_empty() {
            continue;
        }
        let (Ok(left), Ok(right)) = (
            parse_object_filter(left, false),
            parse_object_filter(right, false),
        ) else {
            continue;
        };
        exception_filters = Some([left, right]);
        break;
    }
    let Some(exception_filters) = exception_filters else {
        return Ok(None);
    };

    let affected_types = affected
        .card_types
        .iter()
        .chain(affected.all_card_types.iter())
        .copied()
        .collect::<Vec<_>>();
    if affected_types.is_empty() {
        return Ok(None);
    }
    for exception in exception_filters {
        let exception_types = exception
            .card_types
            .iter()
            .chain(exception.all_card_types.iter())
            .copied()
            .collect::<Vec<_>>();
        let mut unsupported_exception_constraints = exception.clone();
        unsupported_exception_constraints.zone = None;
        unsupported_exception_constraints.card_types.clear();
        unsupported_exception_constraints.all_card_types.clear();
        unsupported_exception_constraints.name = None;
        if unsupported_exception_constraints != ObjectFilter::default() {
            return Ok(None);
        }
        if !affected_types
            .iter()
            .all(|card_type| crate::slice_primitives::contains(&exception_types, card_type))
        {
            return Ok(None);
        }
        let additional_types = exception_types
            .iter()
            .copied()
            .filter(|card_type| !crate::slice_primitives::contains(&affected_types, card_type))
            .collect::<Vec<_>>();
        if let Some(name) = exception.name {
            if !additional_types.is_empty()
                || affected.excluded_name.is_some()
                || !exception.subtypes.is_empty()
                || exception.colors.is_some()
            {
                return Ok(None);
            }
            affected.excluded_name = Some(name);
        } else {
            if additional_types.is_empty()
                || !exception.subtypes.is_empty()
                || exception.colors.is_some()
            {
                return Ok(None);
            }
            for card_type in additional_types {
                if !crate::slice_primitives::contains(&affected.excluded_card_types, &card_type) {
                    affected.excluded_card_types.push(card_type);
                }
            }
        }
    }

    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::attack(affected),
        crate::lexer::render_token_slice(tokens)
            .trim()
            .trim_end_matches('.')
            .to_string(),
    )))
}

/// Whether the clause after a leading restriction duration is a mana-retention
/// clause ("until end of turn, you don't lose this mana ..."), which is not a
/// static restriction.
fn restriction_duration_remainder_retains_mana(tokens: &[OwnedLexToken]) -> bool {
    let Ok(Some((_, remainder))) = parse_restriction_duration(tokens) else {
        return false;
    };
    if remainder.len() >= tokens.len() {
        return false;
    }
    let storage = normalize_cant_words(&remainder);
    let words = storage.iter().map(String::as_str).collect::<Vec<_>>();
    crate::grammar::activation_restrictions::parse_mana_retention_negated_clause_words(&words)
        .is_some()
}

use crate::recognition::ParseOutcome;
#[path = "activation_costs/cant_clause_readings.rs"]
mod cant_clause_readings;

pub fn parse_cant_clauses(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    // These complete shapes lower through dedicated static-ability
    // productions. The generic negated-restriction grammar owns neither the
    // source-owner marker nor a quantified per-creature granted restriction.
    if crate::grammar::attached_object_static_lines::parse_attached_combat_restriction_tokens(
        tokens,
    )
    .is_some()
        || crate::grammar::abilities::is_this_creature_cant_attack_its_owner_line_lexed(tokens)
        || crate::grammar::abilities::parse_flying_block_restriction_line_lexed(tokens).is_some()
        || crate::grammar::anthem_grants::parse_keywords_and_cant_be_blocked_by_more_than_clause(
            tokens,
        )
        .is_some()
        || crate::grammar::anthem_grants::parse_cant_be_blocked_by_more_than_clause(tokens)
            .is_some_and(|clause| {
                crate::grammar::anthem_grants::parse_each_creature_subject(clause.subject_tokens)
                    .is_some()
            })
        || crate::grammar::keyword_static_lines::parse_dont_untap_during_controllers_step_tokens(
            tokens,
        )
        .is_some()
        || crate::keyword_static::parse_static_text_marker_line(tokens).is_some()
        || crate::grammar::anthem_grants::parse_cant_be_blocked_and_has_keywords_clause(tokens)
            .is_some()
        // The mirrored order ("<subject> has <keywords> and can't be blocked")
        // is one grant-plus-restriction production. Reading only its negated
        // half drops the granted keywords.
        || matches!(
            crate::keyword_static::parse_subject_has_keywords_and_cant_be_blocked_line(tokens),
            Ok(Some(_))
        )
        // A subject-scoped evasion line ("Blue creatures you control can't be
        // blocked") has its own canonical static ability. Competing here makes
        // the whole line ambiguous, and the statement fallback then renders it
        // as a pronoun grant that has lost the subject filter.
        || matches!(
            crate::keyword_static::parse_subject_cant_be_blocked_line(tokens),
            Ok(Some(_))
        )
    {
        // Flying-only evasion has a dedicated canonical static ability. The
        // generic negated-restriction route can recognize the same words but
        // does not lower to the same runtime model, so it must not compete for
        // this grammar-proven complete line.
        return Ok(None);
    }
    // A leading duration belongs to the effect sentence parser.  If this
    // rule claims it as a static restriction, the duration is lost and
    // temporary spell effects such as "Until end of turn, players can't gain
    // life" compile without a spell effect at all.
    if crate::token_primitives::parse_simple_restriction_duration_prefix(tokens).is_some() {
        return Ok(None);
    }

    if cant_shapes::parse_multi_sentence_cant_decline_tokens(tokens).is_some() {
        return Ok(None);
    }

    // NOTE(equip-grant threshold, 2026-07-25): a guard declining
    // "equipped/enchanted creature has ..." lines here was tried and
    // REVERTED — with parse_cant_clauses out of the way the line reaches
    // parse_equipped_creature_has_line (whose parse_ability_line now handles
    // the by-more-than tail via KeywordAction::CantBeBlockedByMoreThan), but
    // compilation then fails downstream with "unsupported subject target
    // phrase (clause: 'trample')" from parse_subject_object_filter — a later
    // stage re-parses the grant subject. Root-cause that error before
    // re-adding the guard.

    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder.as_slice() != tokens
    {
        let Some(abilities) = parse_cant_clauses(&remainder)? else {
            return Ok(None);
        };
        let conditioned = abilities
            .into_iter()
            .map(|ability| {
                ability
                    .clone()
                    .with_condition(condition.clone())
                    .unwrap_or(ability)
            })
            .collect::<Vec<_>>();
        return Ok(Some(conditioned));
    }

    // The combined attack-or-block wording is also superficially a blocking
    // payment clause. Claim its typed attack-unless meaning first so the
    // blocking-cost parser does not reinterpret the condition as a payment
    // segment (for example, `control seven or more lands`).
    let input = cant_clause_readings::CantClause {
        tokens,
        read_by_cache: Default::default(),
    };
    match cant_clause_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    // The declines the ladder made before its fallback still gate the fallback.
    if cant_clause_readings::declines(&input) {
        return Ok(None);
    }
    if cant_shapes::parse_stat_modifier_conjunction_decline_tokens(tokens).is_some() {
        return Ok(None);
    }

    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if let Some(segments) = split_cant_clause_on_or(tokens) {
        let mut abilities = Vec::new();
        for segment in segments {
            let Some(ability) = parse_cant_clause(&segment)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::lexer::token_word_refs(&segment).join(" ")
                )));
            };
            abilities.push(ability);
        }
        if !abilities.is_empty() {
            return Ok(Some(abilities));
        }
    }

    if let Some(expansion) = cant_shapes::parse_cant_conjunction_expansion_tokens(tokens) {
        let mut abilities = Vec::new();
        for segment in expansion.segments {
            let Some(ability) = parse_cant_clause(&segment)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::lexer::token_word_refs(&segment).join(" ")
                )));
            };
            abilities.push(ability);
        }
        if !abilities.is_empty() {
            return Ok(Some(abilities));
        }
    }

    parse_cant_clause(tokens).map(|ability| ability.map(|ability| vec![ability]))
}

pub fn split_cant_clause_on_or(tokens: &[OwnedLexToken]) -> Option<Vec<Vec<OwnedLexToken>>> {
    crate::grammar::activation_restrictions::parse_cant_restriction_or_split_tokens(tokens)
        .map(|split| vec![split.first, split.second])
}

/// "Players/You don't lose unspent [color] mana as steps and phases end."
/// (Upwelling, Omnath Locus of Mana, Leyline Tyrant.)
fn parse_unspent_mana_retention_static(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<StaticAbility> {
    use crate::grammar::activation_restrictions::ManaRetentionSubject;

    let parsed =
        crate::grammar::activation_restrictions::parse_unspent_mana_retention_static_words(words)?;
    let subject = match parsed.subject {
        ManaRetentionSubject::You => PlayerFilter::You,
        ManaRetentionSubject::AnyPlayer => PlayerFilter::Any,
    };
    Some(StaticAbility::restriction(
        crate::effect::Restriction::lose_unspent_mana(subject, parsed.color),
        format_negated_restriction_display(tokens),
    ))
}

pub fn parse_cant_clause(tokens: &[OwnedLexToken]) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder.as_slice() != tokens
    {
        let Some(ability) = parse_cant_clause(&remainder)? else {
            return Ok(None);
        };
        let conditioned = ability.clone().with_condition(condition.clone());
        return Ok(Some(conditioned));
    }
    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && !remainder.is_empty()
        && remainder.len() < tokens.len()
        && cant_shapes::parse_negated_untap_remainder_tokens(&remainder).is_some()
    {
        return Ok(None);
    }
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if matches!(
        crate::grammar::activation_restrictions::parse_mana_retention_negated_clause_words(
            &normalized,
        ),
        Some(
            crate::grammar::activation_restrictions::ManaRetentionNegatedClause {
                tail: crate::grammar::activation_restrictions::ManaRetentionTailKind::ThisMana,
            }
        )
    ) {
        return Ok(None);
    }
    if let Some(ability) = parse_unspent_mana_retention_static(tokens, &normalized) {
        return Ok(Some(ability));
    }

    if let Some(fact) = cant_shapes::parse_per_attacker_cant_tax_tokens(tokens) {
        return Ok(Some(if fact.covers_planeswalkers {
            StaticAbility::cant_attack_you_or_planeswalkers_unless_controller_pays_per_attacker(
                fact.amount,
            )
        } else {
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(fact.amount)
        }));
    }

    if let Some(ability) = attack_unless_static_ability(tokens) {
        return Ok(Some(ability));
    }

    if let Some(ability) = blocking_cant_static_ability(tokens) {
        return Ok(Some(ability));
    }

    if let Some(action) = cant_shapes::parse_generic_negated_cant_action_tokens(tokens) {
        match action {
            cant_shapes::GenericNegatedCantAction::SourceBlocksAttacker {
                attacker_tokens, ..
            } => {
                let attacker_tokens = trim_commas(attacker_tokens);
                let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                    .or_else(|| {
                        crate::grammar::primitives::probe_shape(parse_object_filter(
                            &attacker_tokens,
                            false,
                        ))
                    })
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported blocker restriction filter (clause: '{}')",
                            crate::lexer::token_word_refs(tokens).join(" ")
                        ))
                    })?;
                return Ok(Some(StaticAbility::restriction(
                    crate::effect::Restriction::block_specific_attacker(
                        ObjectFilter::source(),
                        attacker_filter,
                    ),
                    format!(
                        "this creature can't block {}",
                        crate::lexer::token_word_refs(&attacker_tokens).join(" ")
                    ),
                )));
            }
            cant_shapes::GenericNegatedCantAction::SubjectCantTransform {
                subject_tokens, ..
            } => {
                let subject_tokens = trim_commas(subject_tokens);
                let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                    return Ok(None);
                };
                let subject_text = crate::lexer::token_word_refs(&subject_tokens).join(" ");
                if subject_text.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(StaticAbility::restriction(
                    crate::effect::Restriction::transform(filter),
                    format!("{subject_text} can't transform"),
                )));
            }
        }
    }

    if let Some(ability) = parity_cant_static_ability(tokens) {
        return Ok(Some(ability));
    }

    if let Some(ability) = fallback_cant_static_ability(tokens) {
        return Ok(Some(ability));
    }

    if let Some(parsed) = parse_cant_restriction_clause(tokens)?
        && parsed.target.is_none()
        && matches!(
            parsed.restriction,
            crate::effect::Restriction::GainLife(_)
                | crate::effect::Restriction::SearchLibraries(_)
                | crate::effect::Restriction::CastSpellsMatching(_, _)
                | crate::effect::Restriction::ActivateNonManaAbilities(_)
                | crate::effect::Restriction::ActivateAbilitiesOf(_)
                | crate::effect::Restriction::ActivateTapAbilitiesOf(_)
                | crate::effect::Restriction::ActivateNonManaAbilitiesOf(_)
                | crate::effect::Restriction::CastMoreThanOneSpellEachTurn(_, _)
                | crate::effect::Restriction::DrawCards(_)
                | crate::effect::Restriction::DrawExtraCards(_)
                | crate::effect::Restriction::LoseLife(_)
                | crate::effect::Restriction::ChangeLifeTotal(_)
                | crate::effect::Restriction::LoseGame(_)
                | crate::effect::Restriction::WinGame(_)
                | crate::effect::Restriction::PreventDamage
        )
    {
        let ability = StaticAbility::restriction(
            parsed.restriction,
            format_negated_restriction_display(tokens),
        );
        return Ok(Some(ability));
    }

    if let Some(resolution) = direct_cant_static_ability(tokens) {
        match resolution {
            StaticAbilityShapeResolution::Ability(ability) => return Ok(Some(ability)),
            StaticAbilityShapeResolution::Decline => return Ok(None),
        }
    }

    if let Some(parsed) = parse_negated_object_restriction_clause(tokens)?
        && parsed.target.is_none()
    {
        return Ok(Some(StaticAbility::restriction(
            parsed.restriction,
            format_negated_restriction_display(tokens),
        )));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::super::super::util::tokenize_line;
    use super::*;

    #[test]
    fn extra_turn_attack_restriction_is_a_typed_condition() {
        let tokens = tokenize_line("This can't attack during extra turns.", 0);
        let ability = parse_cant_clause(&tokens)
            .expect("extra-turn attack restriction should parse")
            .expect("extra-turn attack restriction should be claimed");
        let debug = format!("{ability:#?}");
        assert!(debug.contains("CantAttack"), "{debug}");
        assert!(debug.contains("CurrentTurnIsExtra"), "{debug}");
    }

    #[test]
    fn permanents_cant_phase_in_is_a_typed_static_restriction() {
        let tokens = crate::lexer::lex_line("Permanents can't phase in.", 0)
            .expect("phase-in restriction should lex");
        let abilities = parse_cant_clauses(&tokens)
            .expect("phase-in restriction should parse")
            .expect("phase-in restriction should be claimed as static");
        assert_eq!(abilities.len(), 1);
        assert!(
            format!("{:#?}", abilities[0]).contains("PhaseIn"),
            "{abilities:#?}"
        );
        let repeated = parse_cant_clauses(&tokens)
            .expect("repeated phase-in restriction parse should not error")
            .expect("repeated phase-in restriction parse should remain static");
        assert!(
            format!("{:#?}", repeated[0]).contains("PhaseIn"),
            "{repeated:#?}"
        );

        let contextual = parse_cant_clauses(&tokens)
            .expect("phase-in restriction should parse in a card source context")
            .expect("phase-in restriction should remain static in a card source context");
        assert!(
            format!("{:#?}", contextual[0]).contains("PhaseIn"),
            "{contextual:#?}"
        );
    }

    #[test]
    fn except_for_named_and_type_filters_stay_one_attack_restriction() {
        let tokens = crate::lexer::lex_line(
            "Except for creatures named Akron Legionnaire and artifact creatures, creatures you control can't attack.",
            0,
        )
        .expect("exception restriction should lex");

        let abilities = parse_cant_clauses(&tokens)
            .expect("except-for attack restriction should parse")
            .expect("expected one static restriction");

        assert_eq!(abilities.len(), 1);
        assert_eq!(
            abilities[0].display(),
            "Except for creatures named Akron Legionnaire and artifact creatures, creatures you control can't attack"
        );
        let ironsmith_core::StaticAbilityPayload::RuleRestriction {
            restriction: crate::effect::Restriction::Attack(filter),
            ..
        } = &abilities[0].payload
        else {
            panic!("expected one typed attack restriction: {:#?}", abilities[0]);
        };
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.excluded_name.as_deref(), Some("akron legionnaire"));
        assert!(
            filter.excluded_card_types.contains(&CardType::Artifact),
            "{filter:#?}"
        );
    }

    #[test]
    fn pronoun_conjunction_inherits_the_typed_restriction_subject() {
        let tokens = tokenize_line(
            "Clerics your opponents control can't block, and they can't attack you or planeswalkers you control.",
            0,
        );

        let abilities = parse_cant_clauses(&tokens)
            .expect("shared-subject restrictions should parse")
            .expect("expected two typed restrictions");
        assert_eq!(abilities.len(), 2, "{abilities:#?}");

        let ironsmith_core::StaticAbilityPayload::RuleRestriction {
            restriction: crate::effect::Restriction::Block(blockers),
            ..
        } = &abilities[0].payload
        else {
            panic!("expected a block restriction: {:#?}", abilities[0]);
        };
        let ironsmith_core::StaticAbilityPayload::RuleRestriction {
            restriction:
                crate::effect::Restriction::AttackPlayerOrPlaneswalkersControlledBy {
                    attackers,
                    player: PlayerFilter::You,
                },
            ..
        } = &abilities[1].payload
        else {
            panic!("expected an attack-target restriction: {:#?}", abilities[1]);
        };

        assert_eq!(blockers, attackers);
        assert_eq!(blockers.controller, Some(PlayerFilter::Opponent));
        assert!(blockers.subtypes.contains(&crate::Subtype::Cleric));
        assert_eq!(
            abilities[1].display(),
            "clerics your opponents control can't attack you or planeswalkers you control"
        );
    }

    #[test]
    fn parse_cant_attack_or_block_unless_cards_in_exile_condition() {
        let tokens = tokenize_line(
            "This creature can't attack or block unless there are seven or more cards in exile.",
            0,
        );

        let abilities = parse_cant_clauses(&tokens)
            .expect("cant-attack-or-block-unless-exile-count should parse")
            .expect("expected a static restriction");

        assert_eq!(abilities.len(), 1);
        let debug = format!("{:?}", abilities[0]);
        assert!(debug.contains("AttackOrBlock"), "{debug}");
        assert!(debug.contains("ValueComparison"), "{debug}");
        assert!(debug.contains("GreaterThanOrEqual"), "{debug}");
        assert!(debug.contains("Fixed(7)"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");

        let display = abilities[0].display().to_ascii_lowercase();
        assert!(
            display.contains("can't attack or block unless there are seven or more cards in exile")
                || display
                    .contains("cant attack or block unless there are seven or more cards in exile"),
            "expected original conditional attack/block restriction text, got {display}"
        );
    }

    #[test]
    fn parse_direct_attack_or_block_tap_cost_with_combatant_exclusion() {
        let tokens = tokenize_line(
            "This creature can't attack or block unless you tap an untapped creature you control not declared as an attacking or blocking creature this combat.",
            0,
        );
        let abilities = parse_cant_clauses(&tokens)
            .expect("direct attack-or-block tap cost should not error")
            .expect("direct attack-or-block tap cost should parse");
        assert_eq!(abilities.len(), 1, "{abilities:#?}");
        assert!(matches!(
            abilities[0].payload,
            ironsmith_core::StaticAbilityPayload::BlockCost { .. }
        ));
    }

    #[test]
    fn u035_lowers_source_counter_limit_to_typed_static_payload() {
        let tokens = tokenize_line(
            "This creature can't have more than seven dream counters on it.",
            0,
        );
        let abilities = parse_cant_clauses(&tokens)
            .expect("counter limit should parse")
            .expect("counter limit should lower to a static ability");
        assert_eq!(abilities.len(), 1);
        assert_eq!(
            abilities[0].id(),
            crate::static_abilities::StaticAbilityId::CounterLimit
        );
        assert!(matches!(
            &abilities[0].payload,
            ironsmith_core::StaticAbilityPayload::CounterLimit {
                counter_type: CounterType::Dream,
                maximum: 7,
                ..
            }
        ));
    }

    #[test]
    fn parse_cant_attack_unless_routes_control_tail_through_capture_shape() {
        let cases = [
            (
                "This creature can't attack unless you control another artifact.",
                "other: true",
            ),
            (
                "This creature can't attack unless you control seven or more lands.",
                "PlayerHasAtLeast",
            ),
        ];

        for (text, expected_debug) in cases {
            let tokens = tokenize_line(text, 0);
            let abilities = parse_cant_clauses(&tokens)
                .expect("cant-attack-unless-control condition should parse")
                .expect("expected a static restriction");

            assert_eq!(abilities.len(), 1, "{text}");
            let debug = format!("{:?}", abilities[0]);
            assert!(debug.contains("CantAttackUnlessCondition"), "{debug}");
            assert!(debug.contains(expected_debug), "{debug}");
        }
    }

    #[test]
    fn parse_cant_attack_unless_routes_defending_player_control_tail_through_capture_shape() {
        let cases = [
            (
                "This creature can't attack unless defending player controls an Island.",
                "Island",
            ),
            (
                "This creature can't attack unless defending player controls a snow land.",
                "Snow",
            ),
            (
                "This creature can't attack unless defending player controls a creature with flying.",
                "Flying",
            ),
            (
                "This creature can't attack unless defending player controls a blue permanent.",
                "colors: Some",
            ),
        ];

        for (text, expected_debug) in cases {
            let tokens = tokenize_line(text, 0);
            let abilities = parse_cant_clauses(&tokens)
                .expect("cant-attack-unless-defending-player-controls condition should parse")
                .expect("expected a static restriction");

            assert_eq!(abilities.len(), 1, "{text}");
            let debug = format!("{:?}", abilities[0]);
            assert!(debug.contains("DefendingPlayerCondition"), "{debug}");
            assert!(debug.contains(expected_debug), "{debug}");
        }
    }

    #[test]
    fn parse_this_token_cant_be_blocked_clause() {
        let tokens = tokenize_line("This token can't be blocked.", 0);

        let abilities = parse_cant_clauses(&tokens)
            .expect("this-token-cant-be-blocked clause should parse")
            .expect("expected unblockable static ability");

        assert_eq!(abilities.len(), 1);
        let display = abilities[0].display().to_ascii_lowercase();
        let debug = format!("{:?}", abilities[0]).to_ascii_lowercase();
        assert!(
            display.contains("can't be blocked")
                || display.contains("cant be blocked")
                || display.contains("unblockable")
                || debug.contains("unblockable"),
            "expected unblockable static ability, display={display}, debug={debug}"
        );
    }

    #[test]
    fn compound_keyword_maximum_blocker_line_is_owned_by_the_grant_grammar() {
        let tokens = tokenize_line(
            "Equipped creature has trample and can't be blocked by more than one creature.",
            0,
        );

        assert!(
            parse_cant_clauses(&tokens)
                .expect("generic restriction probe should not error")
                .is_none()
        );
    }
}
