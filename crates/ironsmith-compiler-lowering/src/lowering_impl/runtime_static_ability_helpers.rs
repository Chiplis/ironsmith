pub use ironsmith_compiler_semantic::keyword_abilities::static_ability_for_keyword_action;
use crate::ability::{Ability, AbilityKind, PresentationKeyword, PresentationLabel};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, GrantedAbilityAst, KeywordAction,
};
use crate::effect::Effect;
use crate::filter::ObjectFilter;
pub use ironsmith_compiler_semantic::grant::compiler_granted_ability_ast_to_object_ability;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility as RuntimeStaticAbility;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;

use super::lowering_support::{lower_keyword_action_or_err, lower_parsed_ability};

/// Expands gameplay keywords to the complete object-ability set
/// used by a printed instance of the same keyword.
pub fn executable_object_abilities_for_keyword_action(
    action: &KeywordAction,
) -> Option<Vec<Ability>> {
    if !matches!(
        action,
        KeywordAction::Afterlife(_)
            | KeywordAction::Fabricate(_)
            | KeywordAction::Undying
            | KeywordAction::Persist
            | KeywordAction::Prowess
            | KeywordAction::Exalted
            | KeywordAction::Provoke
            | KeywordAction::Storm
            | KeywordAction::Gravestorm
            | KeywordAction::Toxic(_)
            | KeywordAction::Poisonous(_)
            | KeywordAction::BattleCry
            | KeywordAction::Melee
            | KeywordAction::Myriad
            | KeywordAction::Afflict(_)
            | KeywordAction::Dethrone
            | KeywordAction::Evolve
            | KeywordAction::Increment
            | KeywordAction::Ingest
            | KeywordAction::Mentor
            | KeywordAction::Training
            | KeywordAction::Riot
            | KeywordAction::Renown(_)
            | KeywordAction::Modular(_)
            | KeywordAction::Graft(_)
        | KeywordAction::Ripple(_)
            | KeywordAction::Soulbond
            | KeywordAction::Soulshift(_)
            | KeywordAction::SoulshiftValue(_)
        | KeywordAction::Mobilize(_)
        | KeywordAction::MobilizeValue { .. }
            | KeywordAction::Outlast(_)
            | KeywordAction::Unearth(_)
            | KeywordAction::UnearthTotalCost(_)
            | KeywordAction::Encore(_)
            | KeywordAction::EncoreFromSourceCost { .. }
            | KeywordAction::EmbalmFromSourceCost
            | KeywordAction::Eternalize(_)
            | KeywordAction::Ninjutsu(_)
            | KeywordAction::CommanderNinjutsu(_)
            | KeywordAction::Devour(_)
            | KeywordAction::DevourVariant { .. }
            | KeywordAction::Extort
            // "The next artifact creature spell ... has undaunted" (Võx):
            // the printed hand/stack cost reduction (CR 702.125a).
            | KeywordAction::Undaunted
            | KeywordAction::Sunburst
            | KeywordAction::Firebending(_)
            | KeywordAction::FirebendingValue { .. }
            | KeywordAction::Fading(_)
            | KeywordAction::Vanishing(_)
            | KeywordAction::Rampage(_)
            | KeywordAction::Bushido(_)
            | KeywordAction::BushidoValue(_)
            | KeywordAction::Frenzy(_)
            | KeywordAction::Annihilator(_)
            | KeywordAction::AnnihilatorValue(_)
            // "it gains crew 2" (Captain Rex Nebula): crew/saddle are
            // activated abilities, granted through the printed expansion.
            | KeywordAction::Crew { .. }
            | KeywordAction::Saddle { .. }
            // "Each Equipment you control has living weapon" (Ekthi): the
            // printed ETB trigger of each Equipment (CR 702.92a).
            | KeywordAction::LivingWeapon
    ) {
        return None;
    }

    if matches!(action, KeywordAction::Sunburst) {
        let creature_condition = crate::ConditionExpr::SourceMatches(ObjectFilter::creature());
        let noncreature_condition = crate::ConditionExpr::Not(Box::new(creature_condition.clone()));
        return Some(vec![
            Ability::static_ability(RuntimeStaticAbility::keyword_marker("sunburst")),
            Ability::static_ability(
                RuntimeStaticAbility::enters_with_counters_value(
                    crate::object::CounterType::PlusOnePlusOne,
                    crate::effect::Value::ColorsOfManaSpentToCastThisSpell,
                )
                .with_condition(creature_condition),
            ),
            Ability::static_ability(
                RuntimeStaticAbility::enters_with_counters_value(
                    crate::object::CounterType::Charge,
                    crate::effect::Value::ColorsOfManaSpentToCastThisSpell,
                )
                .with_condition(noncreature_condition),
            ),
        ]);
    }

    let builder = CardDefinitionBuilder::new(crate::CardId::new(), "keyword grant template")
        .card_types(vec![crate::types::CardType::Creature])
        .apply_keyword_action(action.clone());
    Some(builder.abilities)
}

pub fn lower_granted_ability_ast(
    ability: &GrantedAbilityAst,
) -> Result<RuntimeStaticAbility, CardTextError> {
    match ability {
        GrantedAbilityAst::Bundle(_)=>Err(CardTextError::InvariantViolation("a composite granted ability must lower through the complete list owner".into())),
        GrantedAbilityAst::KeywordAction(action) => lower_keyword_action_or_err((**action).clone()),
        GrantedAbilityAst::StaticAbility(ability) => {
            crate::lowering_support::lower_static_ability_ast((**ability).clone())
        }
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot lower as a static ability".to_string(),
        )),
        GrantedAbilityAst::StaticAbilityFamily(_) => Err(CardTextError::InvariantViolation(
            "an ability family can only be lost, not stored as an object ability".to_string(),
        )),
        GrantedAbilityAst::MustAttack => Ok(RuntimeStaticAbility::must_attack()),
        GrantedAbilityAst::MustBlock => Ok(RuntimeStaticAbility::must_block()),
        GrantedAbilityAst::CanAttackAsThoughNoDefender => {
            Ok(RuntimeStaticAbility::can_attack_as_though_no_defender())
        }
        GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional } => {
            Ok(RuntimeStaticAbility::can_block_additional_creature_each_combat(*additional))
        }
        GrantedAbilityAst::ParsedObjectAbility { ability, display } => {
            let lowered = lower_parsed_ability((**ability).clone())?;
            Ok(RuntimeStaticAbility::grant_object_ability_for_filter(
                ObjectFilter::source(),
                lowered,
                display.clone(),
            ))
        }
    }
}

pub fn lower_granted_abilities_ast(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<RuntimeStaticAbility>, CardTextError> {
    let mut result=Vec::new();
    for ability in abilities {match ability {
        GrantedAbilityAst::Bundle(members)=>result.extend(lower_granted_abilities_ast(members)?),
        ability=>result.push(lower_granted_ability_ast(ability)?),
    }}
    Ok(result)
}

pub fn decayed_triggered_ability() -> Ability {
    Ability::triggered(
        Trigger::this_attacks(),
        ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::EndOfCombat,
                vec![Effect::sacrifice_source()],
                true,
                Vec::new(),
                PlayerFilter::You,
            ),
        )]),
    )
}

pub fn decayed_object_abilities() -> Vec<Ability> {
    vec![
        Ability::static_ability(RuntimeStaticAbility::keyword_marker("decayed")),
        Ability::static_ability(RuntimeStaticAbility::cant_block()),
        decayed_triggered_ability(),
    ]
}

pub fn exalted_triggered_ability() -> Ability {
    let attacker_tag = crate::tag::CompilerReferenceTag::ExaltedAttacker.bind();
    Ability::triggered(
        Trigger::attacks_alone(ObjectFilter::creature().you_control()),
        vec![
            Effect::tag_triggering_object(attacker_tag.clone()),
            Effect::pump(
                1,
                1,
                crate::target::ChooseSpec::Tagged(attacker_tag.key.clone()),
                crate::effect::Until::EndOfTurn,
            ),
        ],
    )
}

pub fn suspend_exile_triggered_abilities() -> Vec<Ability> {
    vec![
        Ability {
            kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
                effects: ResolutionProgram::from_effects(vec![Effect::remove_counters(
                    crate::object::CounterType::Time,
                    1,
                    crate::target::ChooseSpec::Source,
                )]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasCounterAtLeast {
                    counter_type: crate::object::CounterType::Time,
                    count: 1,
                    surface: crate::SourceCounterThresholdSurface::SourceHas,
                }),
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Suspend)),
            }),
            designations: Default::default(),
            functional_zones: vec![crate::zone::Zone::Exile],
        },
        Ability {
            kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                // CR 702.62a: "When the last time counter is removed from
                // this card, if it's exiled, you may play it ..."
                trigger: Trigger::new(
                    crate::triggers::CounterRemovedFromTrigger::new(ObjectFilter::source())
                        .counter_type(crate::object::CounterType::Time)
                        .last(),
                ),
                effects: ResolutionProgram::from_effects(vec![Effect::may(vec![Effect::new(
                    crate::effects::CastSourceEffect::new()
                        .without_paying_mana_cost()
                        .require_exile()
                        .cast_as_suspend(),
                )])]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceIsInZone(
                    crate::zone::Zone::Exile,
                )),
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Suspend)),
            }),
            designations: Default::default(),
            functional_zones: vec![crate::zone::Zone::Exile],
        },
    ]
}

fn graveyard_return_counter_ability(
    counter_type: crate::object::CounterType,
    trigger_tag: &'static str,
    return_tag: &'static str,
    returned_tag: &'static str,
) -> Ability {
    let filter = crate::target::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Graveyard)
        .same_stable_id_as_tagged(ironsmith_compiler_semantic::tag::declared_key(trigger_tag));

    Ability {
        kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: ResolutionProgram::from_effects(vec![
                Effect::tag_triggering_object(trigger_tag),
                Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                    filter, return_tag,
                )),
                // CR 702.79a / 702.93a: it returns "with a counter on it", an
                // entry counter placed as it enters (CR 122.6), not afterwards.
                Effect::new(
                    crate::effects::MoveToZoneEffect::new(
                        crate::target::ChooseSpec::Tagged(
                            ironsmith_compiler_semantic::tag::declared_key(return_tag).into(),
                        ),
                        crate::zone::Zone::Battlefield,
                        true,
                    )
                    .under_owner_control()
                    .with_entry_counter(ironsmith_core::BattlefieldEntryCounterSpec::new(
                        counter_type,
                        1,
                        ironsmith_core::BattlefieldEntryCounterSurface::Inline,
                    )),
                )
                .tag(returned_tag),
            ]),
            choices: vec![],
            intervening_if: Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::TriggeringObjectHadCounters {
                    counter_type,
                    min_count: 1,
                },
            ))),
            presentation_label: None,
        }),
        designations: Default::default(),
        functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
    }
}

pub fn persist_triggered_ability() -> Ability {
    graveyard_return_counter_ability(
        crate::object::CounterType::MinusOneMinusOne,
        "persist_trigger",
        "persist_return",
        "persist_returned",
    )
}

pub fn undying_triggered_ability() -> Ability {
    graveyard_return_counter_ability(
        crate::object::CounterType::PlusOnePlusOne,
        "undying_trigger",
        "undying_return",
        "undying_returned",
    )
}

pub fn lower_granted_ability_ast_to_object_ability(
    ability: &GrantedAbilityAst,
) -> Result<Ability, CardTextError> {
    match ability {
        GrantedAbilityAst::Bundle(_)=>Err(CardTextError::InvariantViolation("a composite granted ability must lower through the complete list owner".into())),
        GrantedAbilityAst::KeywordAction(action) => {
            if let KeywordAction::CumulativeUpkeep { total_cost, .. } = action.as_ref() {
                return crate::lowering_support::lower_compiler_ability_core(
                    ironsmith_compiler_semantic::keyword_abilities::cumulative_upkeep_granted_ability(total_cost.clone()),
                    None,
                );
            }
            let static_ability = lower_keyword_action_or_err((**action).clone())?;
            // Same rule as the static grant path: the granted keyword functions
            // where the printed keyword functions.
            Ok(crate::lowering_support::with_printed_keyword_zones(
                action,
                Ability::static_ability(static_ability),
            ))
        }
        GrantedAbilityAst::StaticAbility(static_ability) => Ok(Ability::static_ability(
            crate::lowering_support::lower_static_ability_ast((**static_ability).clone())?,
        )),
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot lower as a granted object ability".to_string(),
        )),
        GrantedAbilityAst::StaticAbilityFamily(_) => Err(CardTextError::InvariantViolation(
            "an ability family can only be lost, not stored as an object ability".to_string(),
        )),
        GrantedAbilityAst::MustAttack => {
            let static_ability = RuntimeStaticAbility::must_attack();
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::MustBlock => {
            let static_ability = RuntimeStaticAbility::must_block();
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::CanAttackAsThoughNoDefender => {
            let static_ability = RuntimeStaticAbility::can_attack_as_though_no_defender();
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional } => {
            let static_ability =
                RuntimeStaticAbility::can_block_additional_creature_each_combat(*additional);
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
            let reference_env=crate::model::reference_state::ReferenceEnv::from_imports(&ability.reference_imports,false,false,false,None);
            let imports=ability.effects_ast.as_ref().map(|effects|
                ironsmith_compiler_resolve::tag_support::granted_cast_object_reference_imports(effects)).unwrap_or_default()
                .into_iter().map(|tag|crate::reference_helpers::resolve_it_tag_key(&tag,&reference_env)).collect::<Result<Vec<_>,_>>()?;
            let mut lowered = lower_parsed_ability((**ability).clone())?;
            match &mut lowered.kind {
                AbilityKind::Triggered(triggered)=>triggered.effects.object_reference_imports=imports,
                AbilityKind::Activated(activated)=>activated.effects.object_reference_imports=imports,
                _=>{},
            }
            Ok(lowered)
        }
    }
}

pub fn compiler_granted_abilities_ast_to_object_abilities(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<crate::model::compiler_semantic::CompilerAbilityCore>, CardTextError> {
    let mut result=Vec::new();
    for ability in abilities {match ability {
        GrantedAbilityAst::Bundle(members)=>result.extend(compiler_granted_abilities_ast_to_object_abilities(members)?),
        ability=>result.push(compiler_granted_ability_ast_to_object_ability(ability)?),
    }}
    Ok(result)
}

pub fn lower_granted_abilities_ast_to_object_abilities(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<Ability>, CardTextError> {
    let mut lowered = Vec::new();
    for ability in abilities {
        if let GrantedAbilityAst::Bundle(members)=ability {
            lowered.extend(lower_granted_abilities_ast_to_object_abilities(members)?);continue;
        }
        if let GrantedAbilityAst::KeywordAction(action) = ability
            && let Some(executable) =
                executable_object_abilities_for_keyword_action(action.as_ref())
        {
            lowered.extend(executable);
            continue;
        }
        match ability {
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Decayed) =>
            {
                lowered.extend(decayed_object_abilities());
            }
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Persist) =>
            {
                lowered.push(persist_triggered_ability());
            }
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Undying) =>
            {
                lowered.push(undying_triggered_ability());
            }
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Exalted) =>
            {
                lowered.push(exalted_triggered_ability());
            }
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Marker("suspend")) =>
            {
                lowered.extend(suspend_exile_triggered_abilities());
            }
            _ => lowered.push(lower_granted_ability_ast_to_object_ability(ability)?),
        }
    }
    Ok(lowered)
}

/// Stores a complete object-ability set in a context whose schema only accepts
/// static abilities. Purely static sets stay direct; sets containing triggered
/// or activated abilities use a source-filtered executable grant carrier.
pub fn object_abilities_to_static_carriers(
    abilities: Vec<Ability>,
    display: String,
) -> Result<Vec<RuntimeStaticAbility>, CardTextError> {
    if abilities
        .iter()
        .all(|ability| matches!(ability.kind, AbilityKind::Static(_)))
    {
        return Ok(abilities
            .into_iter()
            .filter_map(|ability| match ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability),
                _ => None,
            })
            .collect());
    }

    let mut abilities = abilities.into_iter();
    let first = abilities.next().ok_or_else(|| {
        CardTextError::InvariantViolation("keyword grant produced no abilities".to_string())
    })?;
    Ok(vec![RuntimeStaticAbility::new(
        crate::static_abilities::GrantObjectAbilityForFilter::new(
            ObjectFilter::source(),
            first,
            display,
        )
        .with_additional_abilities(abilities.collect()),
    )])
}

#[cfg(test)]
mod dynamic_keyword_grant_tests {
    use super::*;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::types::CardType;

    #[test]
    fn blocker_limit_keeps_its_count_on_printed_and_granted_abilities() {
        let action = KeywordAction::CantBeBlockedByMoreThan(2);
        let printed = CardDefinitionBuilder::new(crate::CardId::new(), "blocker limit")
            .apply_keyword_action(action.clone());
        let continuous = super::super::lowering_support::lower_keyword_action_to_object_abilities(
            action.clone(),
        )
        .expect("continuous grant should lower");
        let temporary = lower_granted_abilities_ast_to_object_abilities(&[
            GrantedAbilityAst::KeywordAction(Box::new(action)),
        ])
        .expect("temporary grant should lower");

        for abilities in [printed.abilities, continuous, temporary] {
            let [ability] = abilities.as_slice() else {
                panic!("expected one blocker restriction, got {abilities:?}");
            };
            assert!(matches!(&ability.kind, AbilityKind::Static(restriction)
                if matches!(restriction.payload,
                    crate::static_abilities::StaticAbilityPayload::CantBeBlockedByMoreThan(2))));
        }
    }

    #[test]
    fn keyword_grant_materialization_matches_printed_triggered_abilities() {
        for action in [
            KeywordAction::Melee,
            KeywordAction::Myriad,
            KeywordAction::Afflict(3),
        ] {
            let printed = CardDefinitionBuilder::new(crate::CardId::new(), "printed keyword")
                .card_types(vec![CardType::Creature])
                .apply_keyword_action(action.clone());
            let grant = super::super::lowering_support::lower_keyword_action_to_object_abilities(
                action.clone(),
            )
            .expect("continuous keyword grant must have an executable expansion");
            let temporary = lower_granted_abilities_ast_to_object_abilities(&[
                GrantedAbilityAst::KeywordAction(Box::new(action)),
            ])
            .expect("temporary grants must use the same expansion");
            assert_eq!(format!("{grant:?}"), format!("{:?}", printed.abilities));
            assert_eq!(format!("{temporary:?}"), format!("{grant:?}"));
            assert_eq!(grant.len(), 1);
            assert!(matches!(grant[0].kind, AbilityKind::Triggered(_)));
            assert_eq!(
                grant[0].functional_zones,
                vec![crate::zone::Zone::Battlefield]
            );
        }
    }
}
