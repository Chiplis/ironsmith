use super::*;
use crate::runtime_backend::grammar::activation_costs::cant_shapes::{
    self, AttackUnlessScope, AttackUnlessSurface, BlockingCantFact, CantFallbackFact,
    DirectCantFact, ManaValueParityCantFact,
};

enum StaticAbilityShapeResolution {
    Ability(StaticAbility),
    Decline,
}

fn direct_cant_static_ability(tokens: &[OwnedLexToken]) -> Option<StaticAbilityShapeResolution> {
    let fact = cant_shapes::parse_direct_cant_fact_tokens(tokens)?;
    let ability = match fact {
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
            let max_speed = crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::Speed(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(4),
            };
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                "This creature can't attack or block".to_string(),
            )
            .with_condition(crate::ConditionExpr::Not(Box::new(max_speed)))
        }
        DirectCantFact::DomainAttackTax => {
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control()
        }
    };
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
            let condition = crate::ConditionExpr::Not(Box::new(condition));
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
        CantFallbackFact::SourceDamageDoubledForManaValueParity(_) => {
            Some(StaticAbility::rule_fallback_text(
                crate::runtime_backend::token_word_refs(tokens).join(" "),
            ))
        }
    }
}

pub(crate) fn parse_cant_clauses(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if cant_shapes::parse_multi_sentence_cant_decline_tokens(tokens).is_some() {
        return Ok(None);
    }

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

    if let Some(resolution) = direct_cant_static_ability(tokens) {
        return Ok(match resolution {
            StaticAbilityShapeResolution::Ability(ability) => Some(vec![ability]),
            StaticAbilityShapeResolution::Decline => None,
        });
    }

    if cant_shapes::parse_direct_temporary_cast_decline_tokens(tokens).is_some() {
        return Ok(None);
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some(restriction) = parse_cant_cast_restriction_words(&normalized_words) {
        return Ok(Some(vec![StaticAbility::restriction(
            restriction,
            format_negated_restriction_display(tokens),
        )]));
    }
    if cant_shapes::parse_iterated_player_who_decline_tokens(tokens).is_some() {
        return Ok(None);
    }
    if cant_shapes::parse_leading_if_cant_decline_tokens(tokens).is_some() {
        return Ok(None);
    }
    if matches!(
        crate::runtime_backend::grammar::activation_restrictions::parse_mana_retention_negated_clause_words(
            &normalized_words,
        ),
        Some(crate::runtime_backend::grammar::activation_restrictions::ManaRetentionNegatedClause {
            tail: crate::runtime_backend::grammar::activation_restrictions::ManaRetentionTailKind::ThisMana,
        })
    ) {
        return Ok(None);
    }
    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && remainder.len() < tokens.len()
    {
        let remainder_words_storage = normalize_cant_words(&remainder);
        let remainder_words = remainder_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if crate::runtime_backend::grammar::activation_restrictions::parse_mana_retention_negated_clause_words(
            &remainder_words,
        )
        .is_some()
        {
            return Ok(None);
        }
    }
    // "Players/You don't lose unspent [color] mana as steps and phases end."
    // Parsed before the and-splitting below tears apart "steps and phases end".
    if let Some(ability) = parse_unspent_mana_retention_static(tokens, &normalized_words) {
        return Ok(Some(vec![ability]));
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
                    crate::runtime_backend::token_word_refs(&segment).join(" ")
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
                    crate::runtime_backend::token_word_refs(&segment).join(" ")
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

pub(crate) fn split_cant_clause_on_or(tokens: &[OwnedLexToken]) -> Option<Vec<Vec<OwnedLexToken>>> {
    crate::runtime_backend::grammar::activation_restrictions::parse_cant_restriction_or_split_tokens(
        tokens,
    )
    .map(|split| vec![split.first, split.second])
}

/// "Players/You don't lose unspent [color] mana as steps and phases end."
/// (Upwelling, Omnath Locus of Mana, Leyline Tyrant.)
fn parse_unspent_mana_retention_static(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<StaticAbility> {
    use crate::runtime_backend::grammar::activation_restrictions::ManaRetentionSubject;

    let parsed = crate::runtime_backend::grammar::activation_restrictions::parse_unspent_mana_retention_static_words(words)?;
    let subject = match parsed.subject {
        ManaRetentionSubject::You => PlayerFilter::You,
        ManaRetentionSubject::AnyPlayer => PlayerFilter::Any,
    };
    Some(StaticAbility::restriction(
        crate::effect::Restriction::lose_unspent_mana(subject, parsed.color),
        format_negated_restriction_display(tokens),
    ))
}

pub(crate) fn parse_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder.as_slice() != tokens
    {
        let Some(ability) = parse_cant_clause(&remainder)? else {
            return Ok(None);
        };
        #[cfg(not(feature = "serialization"))]
        {
            let conditioned = ability.clone().with_condition(condition.clone());
            return Ok(Some(conditioned));
        }
        #[cfg(feature = "serialization")]
        {
            let conditioned = ability.clone().with_condition(condition.clone());
            return Ok(conditioned);
        }
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
        crate::runtime_backend::grammar::activation_restrictions::parse_mana_retention_negated_clause_words(
            &normalized,
        ),
        Some(crate::runtime_backend::grammar::activation_restrictions::ManaRetentionNegatedClause {
            tail: crate::runtime_backend::grammar::activation_restrictions::ManaRetentionTailKind::ThisMana,
        })
    ) {
        return Ok(None);
    }
    if let Some(ability) = parse_unspent_mana_retention_static(tokens, &normalized) {
        return Ok(Some(ability));
    }

    if let Some(fact) = cant_shapes::parse_per_attacker_cant_tax_tokens(tokens) {
        return Ok(Some(
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(fact.amount),
        ));
    }

    if let Some(ability) = blocking_cant_static_ability(tokens) {
        return Ok(Some(ability));
    }

    if let Some(ability) = attack_unless_static_ability(tokens) {
        return Ok(Some(ability));
    }

    if let Some(action) = cant_shapes::parse_generic_negated_cant_action_tokens(tokens) {
        match action {
            cant_shapes::GenericNegatedCantAction::SourceBlocksAttacker {
                attacker_tokens, ..
            } => {
                let attacker_tokens = trim_commas(attacker_tokens);
                let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                    .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported blocker restriction filter (clause: '{}')",
                            crate::runtime_backend::token_word_refs(tokens).join(" ")
                        ))
                    })?;
                return Ok(Some(StaticAbility::restriction(
                    crate::effect::Restriction::block_specific_attacker(
                        ObjectFilter::source(),
                        attacker_filter,
                    ),
                    format!(
                        "this creature can't block {}",
                        crate::runtime_backend::token_word_refs(&attacker_tokens).join(" ")
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
                let subject_text =
                    crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
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
}
