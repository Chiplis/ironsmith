use crate::cards::builders::StatChangeActionAst;
use super::*;

#[cfg(test)]
#[test]
pub(super) fn generic_triggered_source_pump_unblockable_keeps_both_effects() {
    let full = "Whenever you cast a noncreature spell, this creature gets +1/+0 until end of turn and can't be blocked this turn.";
    let effects = "this creature gets +1/+0 until end of turn and can't be blocked this turn.";
    let parsed = parse_triggered_text_for_test(full, "you cast a noncreature spell", effects)
        .expect("source pump and unblockable trigger should parse");
    let effects = match &parsed {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_deref()
            .expect("runtime-backed trigger should retain its effect AST"),
        _ => panic!("expected one triggered line: {parsed:#?}"),
    };
    assert!(
        matches!(
            effects,
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                        power: Value::Fixed(1),
                        toughness: Value::Fixed(0),
                        target: TargetAst::Source(_),
                        duration: Until::EndOfTurn,
                        ..
                    }),
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Cant {
                        duration: Until::EndOfTurn,
                        ..
                    },
                    ..
                })
            ]
        ),
        "{effects:#?}"
    );

    let near_miss = parse_triggered_text_for_test(
        "Whenever you cast a noncreature spell, this creature can't be blocked this turn.",
        "you cast a noncreature spell",
        "this creature can't be blocked this turn.",
    )
    .expect("ordinary unblockable trigger should stay parseable");
    let effects = match &near_miss {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_deref()
            .expect("runtime-backed trigger should retain its effect AST"),
        _ => panic!("expected one triggered near miss: {near_miss:#?}"),
    };
    assert_eq!(effects.len(), 1, "{effects:#?}");
}

#[test]
pub(super) fn protected_battle_surface_binds_the_pre_lowering_damage_target_inside_opponent_loop() {
    fn battle_damage() -> EffectAst {
        let mut battle = ObjectFilter::default();
        battle.zone = Some(Zone::Battlefield);
        battle.card_types.push(CardType::Battle);
        EffectAst::subject_verb_damage(Value::Fixed(1), TargetAst::Object(battle, None, None))
    }

    fn damage_filter(effect: &EffectAst) -> &ObjectFilter {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                    target: TargetAst::Object(filter, None, _),
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected a typed non-targeted Battle damage action: {effect:#?}");
        };
        filter
    }

    let mut effects = vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
        effects: vec![battle_damage()],
    })];
    bind_protected_battle_iteration_in_effects(&mut effects, false);
    let [EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects: nested })] = effects.as_slice() else {
        panic!("expected the opponent loop to remain intact: {effects:#?}");
    };
    assert_eq!(
        damage_filter(&nested[0]).protected_by,
        Some(PlayerFilter::IteratedPlayer)
    );

    let mut outside_loop = vec![battle_damage()];
    bind_protected_battle_iteration_in_effects(&mut outside_loop, false);
    assert_eq!(
        damage_filter(&outside_loop[0]).protected_by,
        None,
        "ordinary each-Battle damage must not acquire an iterated opponent"
    );
}

pub fn parse_exert_attack_keyword_line(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let Some(head_tokens) = sentence_tokens.first().copied() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse exert attack line '{}'",
            line.info.raw_line
        )));
    };
    let semantic_grammar::ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref: _,
        source_tokens,
    } = semantic_grammar::parse_exert_attack_head_tokens(head_tokens).map_err(|message| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering {message} '{}'",
            line.info.raw_line
        ))
    })?;

    let followup = sentence_tokens
        .get(1)
        .and_then(|tokens| semantic_grammar::parse_exert_reflexive_followup_tokens(tokens));
    let linked_trigger = if let Some(followup) = followup {
        let normalized_followup_tokens = normalize_exert_followup_source_reference_tokens(
            &source_tokens,
            followup.effect_tokens,
        );
        let effects_ast = parse_effect_sentences_lexed(&normalized_followup_tokens)?;
        Some(
            crate::model::compiler_semantic::CompilerTriggeredAbilityCore {
                trigger: TriggerSpec::StateBased {
                    condition: PredicateAst::ValueComparison {
                        left: Value::Fixed(1),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: Value::Fixed(1),
                    },
                    display: "When you do".to_string(),
                },
                effects: ironsmith_core::ResolutionProgram::from_effects(effects_ast),
                choices: Vec::new(),
                intervening_if: None,
                presentation_label: None,
            },
        )
    } else if sentence_tokens
        .get(1)
        .is_some_and(|tokens| semantic_grammar::parse_when_followup_intro_tokens(tokens))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering expected exert reflexive followup '{}'",
            line.info.raw_line
        )));
    } else {
        None
    };

    Ok(LineAst::StaticAbility(
        StaticAbility::exert_attack(
            only_if_not_exerted_this_turn,
            linked_trigger,
            line.info.raw_line.clone(),
        )
        .into(),
    ))
}
