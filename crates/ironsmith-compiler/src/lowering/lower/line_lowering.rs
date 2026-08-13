//! Mechanical materialization of normalized compiler line chunks.
//!
//! Every semantic choice is complete before this module runs.  Dispatch is
//! therefore exhaustive over typed variants and never inspects source text,
//! tokens, grammar, or previously materialized runtime effects.

use crate::ability::{Ability, AbilityKind};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, GiftTimingAst, KeywordAction, LineInfo, ParseAnnotations,
    PlayerAst, StaticAbilityAst, TriggerSpec,
};
use crate::model::facts::LineSemanticFacts;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

use super::super::effect_pipeline::{
    NormalizedLineChunk, NormalizedParsedAbility, NormalizedPreparedAbility,
    PreparedTriggeredEffectsForLowering,
};
use super::*;

pub(super) fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    chunk: NormalizedLineChunk,
    _info: &LineInfo,
    semantic_facts: &LineSemanticFacts,
    _allow_unsupported: bool,
    _annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    match chunk {
        NormalizedLineChunk::Abilities(actions) => {
            materialize_keyword_actions(builder, state, actions)
        }
        NormalizedLineChunk::StaticAbility(ability) => {
            materialize_static_abilities(builder, vec![ability], semantic_facts)
        }
        NormalizedLineChunk::StaticAbilities(abilities) => {
            materialize_static_abilities(builder, abilities, semantic_facts)
        }
        NormalizedLineChunk::Ability(ability) => materialize_ability(builder, ability),
        NormalizedLineChunk::Triggered {
            trigger,
            prepared,
            max_triggers_per_turn,
        } => materialize_triggered(
            builder,
            trigger,
            prepared,
            max_triggers_per_turn,
            semantic_facts,
        ),
        NormalizedLineChunk::Statement {
            effects_ast,
            prepared,
        } => materialize_statement(builder, state, effects_ast, prepared),
        NormalizedLineChunk::AdditionalCost {
            effects_ast,
            prepared,
        } => materialize_additional_cost(builder, state, effects_ast, prepared),
        NormalizedLineChunk::OptionalCost(cost) => materialize_optional_cost(builder, cost),
        NormalizedLineChunk::GiftKeyword {
            cost,
            prepared,
            followup_text: _,
            timing,
        } => materialize_gift(builder, cost, prepared, timing),
        NormalizedLineChunk::OptionalCostWithCastTrigger {
            cost,
            prepared,
            followup_text: _,
        } => materialize_optional_cost_trigger(builder, cost, prepared),
        NormalizedLineChunk::AdditionalCostChoice { options } => {
            materialize_additional_cost_choice(builder, state, options)
        }
        NormalizedLineChunk::AlternativeCastingMethod(method) => {
            materialize_alternative_cast(builder, method)
        }
    }
}

fn materialize_keyword_actions(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    actions: Vec<KeywordAction>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    for action in actions {
        match action {
            KeywordAction::Backup(amount) => state.pending_backups.push(PendingBackup {
                ability_boundary: builder.abilities.len(),
                amount,
            }),
            KeywordAction::Cipher => state.pending_cipher = true,
            action => builder = builder.apply_keyword_action(action),
        }
    }
    Ok(builder)
}

fn materialize_static_abilities(
    mut builder: CardDefinitionBuilder,
    abilities: Vec<StaticAbilityAst>,
    semantic_facts: &LineSemanticFacts,
) -> Result<CardDefinitionBuilder, CardTextError> {
    for ability in abilities {
        match ability {
            StaticAbilityAst::AttachmentRestriction { filter, .. } => {
                builder.aura_attach_filter = Some(filter);
            }
            StaticAbilityAst::KeywordAction(KeywordAction::Fuse) => {
                builder = builder.has_fuse();
            }
            ability => {
                let ability = rewrite_lower_static_ability_ast(ability)?;
                builder = builder.with_ability(materialize_static_zones(
                    ability,
                    &semantic_facts.static_ability,
                ));
            }
        }
    }
    Ok(builder)
}

fn materialize_static_zones(
    ability: crate::static_abilities::StaticAbility,
    facts: &crate::model::facts::StaticLineSemanticFacts,
) -> Ability {
    let mut materialized = Ability::static_ability(ability.clone());
    if uses_spell_only_functional_zones(&ability) {
        materialized = materialized.in_zones(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if uses_all_zone_functional_zones(&ability) {
        materialized = materialized.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
            Zone::Ante,
            Zone::OutsideGame,
        ]);
    }
    if uses_referenced_ability_functional_zones(&ability, facts.references_this_ability_cost) {
        materialized = materialized.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let Some(zones) = &facts.explicit_functional_zones {
        materialized = materialized.in_zones(zones.clone());
    }
    materialized
}

fn materialize_ability(
    builder: CardDefinitionBuilder,
    ability: NormalizedParsedAbility,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let ability = rewrite_lower_prepared_ability(ability)?;
    Ok(builder.with_ability(ability.into_runtime()))
}

fn materialize_triggered(
    builder: CardDefinitionBuilder,
    trigger: TriggerSpec,
    prepared: PreparedTriggeredEffectsForLowering,
    max_triggers_per_turn: Option<u32>,
    semantic_facts: &LineSemanticFacts,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let functional_zones = infer_triggered_ability_functional_zones_from_facts(
        &trigger,
        &semantic_facts.triggered_ability.functional_zones,
    );
    let intervening_if = trigger_frequency_condition(
        max_triggers_per_turn,
        &semantic_facts.triggered_ability.frequency,
    );
    let parsed = rewrite_parsed_triggered_ability(
        trigger.clone(),
        prepared.prepared.effects.clone(),
        functional_zones,
        None,
        intervening_if,
        semantic_facts.triggered_ability.presentation_label.as_ref(),
        prepared.prepared.imports.clone(),
    );
    let parsed = rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
    })?;
    Ok(builder.with_ability(parsed.into_runtime()))
}

fn trigger_frequency_condition(
    maximum: Option<u32>,
    facts: &crate::model::facts::TriggerFrequencyFacts,
) -> Option<crate::ConditionExpr> {
    maximum.map(|limit| {
        if limit == 1 && facts.first_time_each_or_this_turn && facts.becomes_crewed {
            crate::ConditionExpr::SourceFirstCrewedThisTurn
        } else if limit == 1 && facts.first_time_each_or_this_turn {
            crate::ConditionExpr::FirstTimeThisTurn
        } else if facts.do_this_limit_each_turn.is_some() {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(limit)
        } else {
            crate::ConditionExpr::MaxTimesEachTurn(limit)
        }
    })
}

fn materialize_statement(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    effects_ast: Vec<crate::cards::builders::EffectAst>,
    prepared: PreparedEffectsForLowering,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if effects_ast.is_empty() {
        return Err(CardTextError::InvariantViolation(
            "normalized statement contains no effects".to_string(),
        ));
    }
    let lowered = rewrite_lower_prepared_statement_effects(&prepared)?;
    rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &lowered,
        false,
        "spell text effects",
    )?;
    state.latest_spell_exports = lowered.exports;
    if let Some(existing) = builder.spell_effect.as_mut() {
        existing.extend(lowered.effects);
    } else {
        builder.spell_effect = Some(lowered.effects);
    }
    Ok(builder)
}

fn materialize_additional_cost(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    effects_ast: Vec<crate::cards::builders::EffectAst>,
    prepared: PreparedEffectsForLowering,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if effects_ast.is_empty() {
        return Err(CardTextError::InvariantViolation(
            "normalized additional cost contains no effects".to_string(),
        ));
    }
    let lowered = rewrite_lower_prepared_statement_effects(&prepared)?;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.extend(runtime_effects_to_costs(lowered.effects.to_vec())?);
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    state.latest_additional_cost_exports = lowered.exports;
    Ok(builder)
}

fn materialize_optional_cost(
    mut builder: CardDefinitionBuilder,
    cost: crate::cost::OptionalCost,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let kind = cost.kind.clone();
    let reference = cost.cost_ref();
    builder = builder.optional_cost(cost);
    match kind {
        crate::cost::OptionalCostKind::Squad => {
            builder = builder.with_ability(Ability::triggered(
                crate::triggers::Trigger::this_enters_battlefield(),
                vec![crate::effect::Effect::new(
                    crate::effects::CreateTokenCopyEffect::new(
                        ChooseSpec::Source,
                        crate::effect::Value::TimesPaidLabel(reference),
                        PlayerFilter::You,
                    ),
                )],
            ));
        }
        crate::cost::OptionalCostKind::Offspring => {
            builder = builder.with_ability(Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: crate::triggers::Trigger::this_enters_battlefield(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        crate::effect::Effect::new(
                            crate::effects::CreateTokenCopyEffect::new(
                                ChooseSpec::Source,
                                crate::effect::Value::WasPaidLabel(reference.clone()),
                                PlayerFilter::You,
                            )
                            .set_base_power_toughness(1, 1),
                        ),
                    ]),
                    choices: Vec::new(),
                    intervening_if: Some(crate::effect::Condition::ThisSpellPaidLabel(reference)),
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
        }
        _ => {}
    }
    Ok(builder)
}

fn materialize_gift(
    mut builder: CardDefinitionBuilder,
    cost: crate::cost::OptionalCost,
    prepared: PreparedEffectsForLowering,
    timing: GiftTimingAst,
) -> Result<CardDefinitionBuilder, CardTextError> {
    builder = builder.optional_cost(cost);
    match timing {
        GiftTimingAst::SpellResolution => {
            let lowered = rewrite_lower_prepared_statement_effects(&prepared)?;
            let mut effects = lowered.effects.to_vec();
            effects.push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            let gift = crate::effect::Effect::conditional(
                crate::ConditionExpr::ThisSpellPaidLabel("Gift".into()),
                effects,
                Vec::new(),
            );
            if let Some(existing) = builder.spell_effect.as_mut() {
                existing.push(gift);
            } else {
                builder.spell_effect =
                    Some(crate::resolution::ResolutionProgram::from_effects(vec![
                        gift,
                    ]));
            }
        }
        GiftTimingAst::PermanentEtb => {
            let trigger = TriggerSpec::ThisEntersBattlefield {
                origin_condition: None,
            };
            let parsed = rewrite_parsed_triggered_ability(
                trigger.clone(),
                prepared.effects.clone(),
                vec![Zone::Battlefield],
                None,
                Some(crate::ConditionExpr::ThisSpellPaidLabel("Gift".into())),
                None,
                prepared.imports.clone(),
            );
            let prepared = PreparedTriggeredEffectsForLowering {
                prepared,
                intervening_if: None,
            };
            let mut parsed = rewrite_lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
            })?;
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered
                    .effects
                    .push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            }
            builder = builder.with_ability(parsed.into_runtime());
        }
    }
    Ok(builder)
}

fn materialize_optional_cost_trigger(
    mut builder: CardDefinitionBuilder,
    cost: crate::cost::OptionalCost,
    prepared: PreparedEffectsForLowering,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let reference = cost.cost_ref();
    builder = builder.optional_cost(cost);
    let trigger = TriggerSpec::YouCastThisSpell;
    let parsed = rewrite_parsed_triggered_ability(
        trigger.clone(),
        prepared.effects.clone(),
        vec![Zone::Stack],
        None,
        Some(crate::ConditionExpr::ThisSpellPaidLabel(reference)),
        None,
        prepared.imports.clone(),
    );
    let parsed = rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered {
            trigger,
            prepared: PreparedTriggeredEffectsForLowering {
                prepared,
                intervening_if: None,
            },
        }),
    })?;
    Ok(builder.with_ability(parsed.into_runtime()))
}

fn materialize_additional_cost_choice(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    options: Vec<NormalizedAdditionalCostChoiceOptionAst>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if options.len() < 2 || options.iter().any(|option| option.effects_ast.is_empty()) {
        return Err(CardTextError::InvariantViolation(
            "normalized additional-cost choice requires two nonempty modes".to_string(),
        ));
    }
    let (modes, exports) =
        rewrite_lower_prepared_additional_cost_choice_modes_with_exports(&options)?;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.push(
        crate::costs::payment_effect_to_cost(crate::effect::Effect::choose_one(modes))
            .map_err(CardTextError::InvariantViolation)?,
    );
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    state.latest_additional_cost_exports = exports;
    Ok(builder)
}

fn materialize_alternative_cast(
    mut builder: CardDefinitionBuilder,
    mut method: crate::alternative_cast::AlternativeCastingMethod,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if let crate::alternative_cast::AlternativeCastingMethod::FlashWithAdditionalCost {
        additional_cost,
        ..
    } = &method
    {
        let printed = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut pips = printed.pips().to_vec();
        pips.extend(additional_cost.pips().iter().cloned());
        method = crate::alternative_cast::AlternativeCastingMethod::flash_with_additional_cost(
            additional_cost.clone(),
            crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(pips)),
        );
    }
    if let crate::alternative_cast::AlternativeCastingMethod::Retrace { total_cost } = &method {
        let printed = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut costs = vec![crate::costs::Cost::mana(printed)];
        costs.extend(total_cost.costs().iter().cloned());
        method = crate::alternative_cast::AlternativeCastingMethod::Retrace {
            total_cost: crate::cost::TotalCost::from_costs(costs),
        };
    }
    Ok(builder.alternative_cast(method))
}

#[cfg(test)]
#[test]
fn public_two_line_damage_replacement_reuses_both_announced_targets() {
    let definition = CardDefinitionBuilder::new(crate::CardId::from_raw(1), "Damage Pair Variant")
        .parse_text(
            "This spell deals 1 damage to target player or planeswalker and 1 damage to target creature that player or that planeswalker's controller controls.\nLandfall — If you had a land enter the battlefield under your control this turn, this spell deals 3 damage to that player or planeswalker and 3 damage to that creature instead.",
        )
        .expect("full public document route should lower the damage replacement");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("damage pair should produce a spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one replacement segment: {program:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one typed self-replacement: {segment:#?}");
    };

    fn damage_targets(effect: &crate::effect::Effect) -> Vec<ChooseSpec> {
        let leaf = effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .map_or(effect, |with_id| with_id.effect.as_ref());
        let sequence = leaf
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("coordinated damage pair");
        sequence
            .effects
            .iter()
            .map(|effect| {
                let leaf = effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .map_or(effect, |tagged| tagged.effect.as_ref());
                leaf.downcast_ref::<crate::effects::DealDamageEffect>()
                    .expect("damage leaf")
                    .target
                    .clone()
            })
            .collect()
    }

    let [default] = segment.default_effects.as_slice() else {
        panic!("expected one coordinated default effect: {segment:#?}");
    };
    let [replacement] = branch.replacement_effects.as_slice() else {
        panic!("expected one coordinated replacement effect: {branch:#?}");
    };
    assert_eq!(damage_targets(default), damage_targets(replacement));
    assert!(matches!(
        branch.presentation_label,
        Some(crate::cards::builders::PresentationLabel::AbilityWord(ref label)) if label == "Landfall"
    ));
}

#[cfg(test)]
mod quantified_unless_actor_binding_tests {
    use super::*;

    fn inverted_program(explicit_you: bool) -> crate::resolution::ResolutionProgram {
        let token_definition =
            CardDefinitionBuilder::new(crate::ids::CardId::new(), "Quantified Zombie")
                .card_types(vec![crate::types::CardType::Creature])
                .subtypes(vec![crate::types::Subtype::Zombie])
                .build();
        let mut token = crate::effects::CreateTokenEffect::new(
            token_definition,
            crate::effect::Value::Fixed(1),
            PlayerFilter::Opponent,
        );
        if explicit_you {
            token = token.with_explicit_actor_surface();
        }
        let unless = crate::effects::UnlessPaysEffect {
            player: PlayerFilter::You,
            effects: vec![crate::effect::Effect::new(token)],
            cost: crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
            )),
            leading_surface: false,
            before_delayed_step: false,
        };
        let for_players = crate::effects::ForPlayersEffect {
            filter: PlayerFilter::Opponent,
            effects: vec![crate::effect::Effect::new(unless)],
            starting_with_controller: false,
            stop_after_first_happened: false,
        };
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![crate::effect::Effect::new(
                for_players,
            )]),
        ])
    }

    fn sacrifice_cost(
        program: &crate::resolution::ResolutionProgram,
    ) -> &crate::effects::SacrificeEffect {
        let for_players = program.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .expect("quantified player loop");
        let unless = for_players.effects[0]
            .downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
            .expect("unless payment");
        let [cost] = unless.cost.as_all().expect("all-cost branch") else {
            panic!("expected one sacrifice cost: {unless:#?}");
        };
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
            .expect("typed sacrifice cost")
    }

    #[test]
    fn explicit_you_token_and_that_opponent_sacrifice_recover_distinct_actors() {
        let mut program = inverted_program(true);
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(&mut program, None);
        let debug = format!("{program:#?}");
        assert!(debug.contains("filter: Opponent"), "{debug}");
        assert!(debug.contains("player: IteratedPlayer"), "{debug}");
        assert!(debug.contains("controller: You"), "{debug}");
        assert_eq!(
            sacrifice_cost(&program).filter.controller,
            Some(PlayerFilter::You),
            "the sacrifice cost must stay payer-relative after the outer loop chooses the opponent"
        );

        let mut participant_created = inverted_program(false);
        let before = format!("{participant_created:#?}");
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
            &mut participant_created,
            None,
        );
        assert_eq!(
            format!("{participant_created:#?}"),
            before,
            "a participant-created token must not inherit the controller-action correction"
        );

        let authored = crate::runtime_backend::lex_line(
            "For each opponent, you create a Zombie token unless that player sacrifices a creature of their choice.",
            0,
        )
        .expect("authored quantified token sentence should lex");
        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(
            &mut participant_created,
            Some(&authored),
        );
        let debug = format!("{participant_created:#?}");
        assert!(debug.contains("player: IteratedPlayer"), "{debug}");
        assert!(debug.contains("controller: You"), "{debug}");
    }

    #[test]
    fn authoritative_iterated_player_and_tagged_sacrifice_cost_are_normalized() {
        let mut program = inverted_program(true);
        let root = &mut program.segments[0].default_effects[0];
        let mut for_players = root
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .expect("quantified player loop")
            .clone();
        let mut unless = for_players.effects[0]
            .downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
            .expect("unless payment")
            .clone();
        unless.player = PlayerFilter::IteratedPlayer;
        unless.cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(
            crate::effect::Effect::sacrifice(
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                1,
            )
            .tag("sacrifice_cost_0"),
        ));
        for_players.effects = vec![crate::effect::Effect::new(unless)];
        *root = crate::effect::Effect::new(for_players);

        bind_each_opponent_explicit_you_token_unless_iterated_sacrifice(&mut program, None);
        let debug = format!("{program:#?}");
        assert!(debug.contains("player: IteratedPlayer"), "{debug}");
        assert!(debug.contains("controller: You"), "{debug}");
        assert_eq!(
            sacrifice_cost(&program).filter.controller,
            Some(PlayerFilter::You),
            "the wrapped cost must normalize to payer-relative You"
        );
        assert!(
            !debug.contains("sacrifice_cost_0"),
            "the transparent provenance wrapper should normalize to an executable cost: {debug}"
        );
    }
}

#[cfg(test)]
mod dynamic_power_owner_exile_permission_tests {
    use super::*;

    fn placeholder_program() -> crate::resolution::ResolutionProgram {
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![crate::effect::Effect::new(
                crate::effects::DrawCardsEffect::new(
                    crate::effect::Value::Fixed(1),
                    PlayerFilter::You,
                ),
            )]),
        ])
    }

    #[test]
    fn exact_authored_bundle_reconciles_to_dynamic_owner_linked_permission() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "When enchanted creature dies, exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.",
            0,
        )
        .expect("linked dynamic permission should lex");
        let mut program = placeholder_program();
        bind_dynamic_power_owner_exile_permission(
            &mut program,
            &tokens,
            "When enchanted creature dies, exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.",
        );
        let debug = format!("{program:#?}");
        for required in [
            "TagTriggeringObjectEffect",
            "PowerOf",
            "OwnerOf",
            "ExileTopOfLibraryEffect",
            "GrantPlayTaggedEffect",
            "ForAsLongAsExiled",
            "AnyType",
            "cast_pool_is_plural: true",
        ] {
            assert!(debug.contains(required), "missing {required}: {debug}");
        }

        let lossy_tokens = crate::runtime_backend::lexer::lex_line(
            "exile the top card of your library. You may cast that card for as long as it remains exiled.",
            0,
        )
        .expect("prepared lossy effect slice should lex");
        let mut recovered_from_raw_line = placeholder_program();
        bind_dynamic_power_owner_exile_permission(
            &mut recovered_from_raw_line,
            &lossy_tokens,
            "When enchanted creature dies, exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.",
        );
        let recovered_debug = format!("{recovered_from_raw_line:#?}");
        assert!(recovered_debug.contains("PowerOf"), "{recovered_debug}");
        assert!(recovered_debug.contains("OwnerOf"), "{recovered_debug}");
        assert!(
            recovered_debug.contains("cast_pool_is_plural: true"),
            "{recovered_debug}"
        );

        let near_miss = crate::runtime_backend::lexer::lex_line(
            "When enchanted creature dies, exile the top card of its owner's library. You may cast that card for as long as it remains exiled.",
            0,
        )
        .expect("fixed-card near miss should lex");
        let mut unchanged = placeholder_program();
        let before = format!("{unchanged:#?}");
        bind_dynamic_power_owner_exile_permission(
            &mut unchanged,
            &near_miss,
            "When enchanted creature dies, exile the top card of its owner's library. You may cast that card for as long as it remains exiled.",
        );
        assert_eq!(format!("{unchanged:#?}"), before);
    }
}

#[cfg(test)]
mod graveyard_card_copy_cast_program_normalization_tests {
    use super::*;
    use crate::CardType;

    fn program(exile_tag: &str, cast_tag: &str) -> crate::resolution::ResolutionProgram {
        let exile =
            crate::effect::Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::target(
                ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Graveyard)),
            )))
            .tag(exile_tag);
        let copy = crate::effect::Effect::with_id(
            7,
            crate::effect::Effect::new(
                crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(crate::TagKey::from(
                    exile_tag,
                )))
                .with_target_reference_pronoun(true),
            ),
        )
        .tag(crate::cards::builders::COPIED_STACK_OBJECT_TAG);
        let producer =
            crate::effect::Effect::new(crate::effects::SequenceEffect::coordinated(vec![
                exile, copy,
            ]));
        let cast = crate::effect::Effect::new(
            crate::effects::CastTaggedEffect::new(cast_tag, PlayerFilter::You).as_copy(),
        );
        let may = crate::effect::Effect::may(vec![cast]);
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![producer]),
            crate::resolution::ResolutionSegment::from_effects(vec![may]),
        ])
    }

    #[test]
    fn exact_shared_tag_replaces_the_invalid_stack_copy_with_the_card_copy_cast() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let mut normalized = program(tag, tag);
        normalize_graveyard_card_copy_cast_program(&mut normalized);
        let debug = format!("{normalized:#?}");
        assert!(!debug.contains("CopySpellEffect"), "{debug}");
        assert!(debug.contains("CastTaggedEffect"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");

        let mut wrong_tag = program(tag, "__sentence_helper_exiled_l0_s9_e9");
        normalize_graveyard_card_copy_cast_program(&mut wrong_tag);
        assert!(
            format!("{wrong_tag:#?}").contains("CopySpellEffect"),
            "an unrelated copied-card cast must not consume the producer marker"
        );
    }

    #[test]
    fn conditional_union_graveyard_domain_uses_the_copied_card_cast() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let mut union = ObjectFilter::default();
        union.any_of = vec![
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .with_type(CardType::Creature),
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .with_ability_marker("freerunning"),
        ];
        let producer = crate::effect::Effect::with_id(
            11,
            crate::effect::Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::target(ChooseSpec::Object(union)),
                Zone::Exile,
                true,
            ))
            .tag(tag),
        );
        let obsolete_copy = crate::effect::Effect::with_id(
            11,
            crate::effect::Effect::new(
                crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from(tag)))
                    .with_target_reference_pronoun(true),
            ),
        )
        .tag(crate::cards::builders::COPIED_STACK_OBJECT_TAG);
        let gate = crate::effect::Effect::new(crate::effects::IfEffect::if_then(
            crate::effect::EffectId(11),
            crate::effect::EffectPredicate::Happened,
            vec![obsolete_copy],
        ));
        let may_cast = crate::effect::Effect::may(vec![crate::effect::Effect::new(
            crate::effects::CastTaggedEffect::new(tag, PlayerFilter::You).as_copy(),
        )]);
        let mut program = crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![producer]),
            crate::resolution::ResolutionSegment::from_effects(vec![gate]),
            crate::resolution::ResolutionSegment::from_effects(vec![may_cast]),
        ]);

        normalize_graveyard_card_copy_cast_program(&mut program);
        let debug = format!("{program:#?}");
        assert!(!debug.contains("CopySpellEffect"), "{debug}");
        assert!(debug.contains("IfEffect"), "{debug}");
        assert!(debug.contains("CastTaggedEffect"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");
        assert_eq!(program.segments.len(), 2, "{debug}");
    }
}

#[cfg(test)]
mod delayed_copy_retarget_transport_tests {
    use super::*;

    fn copy_schedule(
        copied_tag: &str,
        reference_kind: crate::filter::StackObjectKind,
    ) -> crate::effect::Effect {
        let triggering_source_tag = crate::TagKey::from("triggering_source");
        let copy = crate::effect::Effect::with_id(
            0,
            crate::effect::Effect::new(
                crate::effects::CopySpellEffect::new(
                    ChooseSpec::Tagged(triggering_source_tag.clone()),
                    crate::effect::Value::Fixed(1),
                )
                .with_target_reference_kind(reference_kind),
            ),
        )
        .tag(crate::TagKey::from(copied_tag));
        crate::effect::Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
            crate::effect::DelayedTriggerSpec::BeginningOfUpkeep(PlayerFilter::You),
            vec![
                crate::effect::Effect::new(crate::effects::TagTriggeringSourceEffect::new(
                    triggering_source_tag,
                )),
                copy,
            ],
            false,
            Vec::new(),
            PlayerFilter::You,
        ))
    }

    fn plural_copy_retarget() -> crate::effect::Effect {
        crate::effect::Effect::may_player(
            PlayerFilter::You,
            vec![crate::effect::Effect::new(
                crate::effects::RetargetStackObjectEffect::new(ChooseSpec::Tagged(
                    crate::TagKey::from(crate::cards::builders::COPIED_STACK_OBJECT_TAG),
                ))
                .with_plural_copy_reference(),
            )],
        )
    }

    fn two_segment_program(
        schedule: crate::effect::Effect,
    ) -> crate::resolution::ResolutionProgram {
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![schedule]),
            crate::resolution::ResolutionSegment::from_effects(vec![plural_copy_retarget()]),
        ])
    }

    #[test]
    fn tagged_with_id_ability_copy_owns_its_plural_retarget() {
        let mut program = two_segment_program(copy_schedule(
            crate::cards::builders::COPIED_STACK_OBJECT_TAG,
            crate::filter::StackObjectKind::Ability,
        ));
        transport_plural_copy_retarget_into_delayed_trigger(&mut program);

        let [segment] = program.segments.as_slice() else {
            panic!("retarget sibling should be absorbed into its schedule: {program:#?}");
        };
        let [schedule] = segment.default_effects.as_slice() else {
            panic!("outer program should retain only the schedule: {segment:#?}");
        };
        let schedule = schedule
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .expect("outer effect should remain a delayed trigger schedule");
        assert_eq!(schedule.effects.len(), 3, "{schedule:#?}");
        assert!(
            schedule.effects[2]
                .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
                .is_some(),
            "the plural retarget May must execute after the delayed copy: {schedule:#?}"
        );
    }

    #[test]
    fn wrong_copy_tag_or_stack_kind_does_not_transport_retarget() {
        for schedule in [
            copy_schedule("ordinary_result", crate::filter::StackObjectKind::Ability),
            copy_schedule(
                crate::cards::builders::COPIED_STACK_OBJECT_TAG,
                crate::filter::StackObjectKind::Spell,
            ),
        ] {
            let mut program = two_segment_program(schedule);
            transport_plural_copy_retarget_into_delayed_trigger(&mut program);
            assert_eq!(
                program.segments.len(),
                2,
                "near miss must keep its outer retarget sibling: {program:#?}"
            );
        }
    }
}
