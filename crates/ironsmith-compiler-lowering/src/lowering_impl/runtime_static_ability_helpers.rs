use crate::ability::{Ability, AbilityKind, PresentationKeyword, PresentationLabel};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, GrantedAbilityAst, KeywordAction,
};
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::CompilerStaticAbilityCore as CompilerStaticAbility;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility as RuntimeStaticAbility;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;

use super::lowering_support::lower_parsed_ability;

/// Expands marker-backed gameplay keywords to the complete object-ability set
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
            | KeywordAction::Storm
            | KeywordAction::Gravestorm
            | KeywordAction::Toxic(_)
            | KeywordAction::Poisonous(_)
            | KeywordAction::BattleCry
            | KeywordAction::Dethrone
            | KeywordAction::Evolve
            | KeywordAction::Ingest
            | KeywordAction::Mentor
            | KeywordAction::Training
            | KeywordAction::Riot
            | KeywordAction::Renown(_)
            | KeywordAction::Modular(_)
            | KeywordAction::Graft(_)
            | KeywordAction::Soulbond
            | KeywordAction::Soulshift(_)
            | KeywordAction::SoulshiftValue(_)
            | KeywordAction::Outlast(_)
            | KeywordAction::Unearth(_)
            | KeywordAction::Eternalize(_)
            | KeywordAction::Ninjutsu(_)
            | KeywordAction::Extort
            | KeywordAction::Sunburst
            | KeywordAction::Firebending(_)
            | KeywordAction::FirebendingValue { .. }
            | KeywordAction::Fading(_)
            | KeywordAction::Vanishing(_)
            | KeywordAction::Rampage(_)
            | KeywordAction::Bushido(_)
            | KeywordAction::Frenzy(_)
            | KeywordAction::Annihilator(_)
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

pub fn static_ability_for_keyword_action(action: KeywordAction) -> Option<CompilerStaticAbility> {
    if !action.lowers_to_static_ability() {
        return None;
    }

    match action {
        KeywordAction::Flying => Some(CompilerStaticAbility::flying()),
        KeywordAction::Menace => Some(CompilerStaticAbility::menace()),
        KeywordAction::Banding => Some(CompilerStaticAbility::banding()),
        KeywordAction::Hexproof => Some(CompilerStaticAbility::hexproof()),
        KeywordAction::HexproofFrom(filter) => {
            Some(CompilerStaticAbility::hexproof_from(filter.clone()))
        }
        KeywordAction::Haste => Some(CompilerStaticAbility::haste()),
        KeywordAction::Improvise => Some(CompilerStaticAbility::improvise()),
        KeywordAction::Convoke => Some(CompilerStaticAbility::convoke()),
        KeywordAction::AffinityForArtifacts => {
            Some(CompilerStaticAbility::affinity_for_artifacts())
        }
        KeywordAction::CantBeCountered => Some(CompilerStaticAbility::cant_be_countered_ability()),
        KeywordAction::Delve => Some(CompilerStaticAbility::delve()),
        KeywordAction::FirstStrike => Some(CompilerStaticAbility::first_strike()),
        KeywordAction::DoubleStrike => Some(CompilerStaticAbility::double_strike()),
        KeywordAction::Deathtouch => Some(CompilerStaticAbility::deathtouch()),
        KeywordAction::Lifelink => Some(CompilerStaticAbility::lifelink()),
        KeywordAction::Vigilance => Some(CompilerStaticAbility::vigilance()),
        KeywordAction::Trample => Some(CompilerStaticAbility::trample()),
        KeywordAction::Reach => Some(CompilerStaticAbility::reach()),
        KeywordAction::Defender => Some(CompilerStaticAbility::defender()),
        KeywordAction::Decayed => Some(CompilerStaticAbility::cant_block()),
        KeywordAction::Flash => Some(CompilerStaticAbility::flash()),
        KeywordAction::Phasing => Some(CompilerStaticAbility::phasing()),
        KeywordAction::Indestructible => Some(CompilerStaticAbility::indestructible()),
        KeywordAction::Shroud => Some(CompilerStaticAbility::shroud()),
        KeywordAction::Daybound => Some(CompilerStaticAbility::daybound()),
        KeywordAction::Nightbound => Some(CompilerStaticAbility::nightbound()),
        KeywordAction::Ward(amount) => u8::try_from(amount).ok().map(|generic| {
            CompilerStaticAbility::ward(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic)]))
        }),
        KeywordAction::Wither => Some(CompilerStaticAbility::wither()),
        KeywordAction::Afflict(_) => None,
        KeywordAction::Amplify(_) => None,
        KeywordAction::Afterlife(_) | KeywordAction::Fabricate(_) => None,
        KeywordAction::Infect => Some(CompilerStaticAbility::infect()),
        KeywordAction::Undying
        | KeywordAction::Persist
        | KeywordAction::Prowess
        | KeywordAction::Exalted => None,
        KeywordAction::Cascade => Some(CompilerStaticAbility::cascade()),
        KeywordAction::Storm
        | KeywordAction::Gravestorm
        | KeywordAction::Toxic(_)
        | KeywordAction::Poisonous(_)
        | KeywordAction::BattleCry
        | KeywordAction::Dethrone
        | KeywordAction::Evolve
        | KeywordAction::Ingest
        | KeywordAction::Mentor => None,
        KeywordAction::Skulk => Some(CompilerStaticAbility::skulk()),
        KeywordAction::Training | KeywordAction::Riot => None,
        KeywordAction::Unleash => Some(CompilerStaticAbility::unleash()),
        KeywordAction::Renown(_)
        | KeywordAction::Modular(_)
        | KeywordAction::Graft(_)
        | KeywordAction::Soulbond
        | KeywordAction::Soulshift(_)
        | KeywordAction::SoulshiftValue(_)
        | KeywordAction::Outlast(_)
        | KeywordAction::Unearth(_)
        | KeywordAction::Eternalize(_)
        | KeywordAction::Ninjutsu(_)
        | KeywordAction::Extort => None,
        KeywordAction::Partner => Some(CompilerStaticAbility::partner()),
        KeywordAction::StartYourEngines => Some(CompilerStaticAbility::start_your_engines()),
        KeywordAction::Assist => Some(CompilerStaticAbility::assist()),
        KeywordAction::SplitSecond => Some(CompilerStaticAbility::split_second()),
        KeywordAction::Rebound => Some(CompilerStaticAbility::rebound()),
        KeywordAction::Sunburst => None,
        KeywordAction::ReadAhead => Some(CompilerStaticAbility::read_ahead()),
        KeywordAction::Firebending(_) | KeywordAction::FirebendingValue { .. } => None,
        KeywordAction::Fading(_) | KeywordAction::Vanishing(_) => None,
        KeywordAction::Fear => Some(CompilerStaticAbility::fear()),
        KeywordAction::Intimidate => Some(CompilerStaticAbility::intimidate()),
        KeywordAction::Shadow => Some(CompilerStaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(CompilerStaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(CompilerStaticAbility::flanking()),
        KeywordAction::UmbraArmor => Some(CompilerStaticAbility::umbra_armor()),
        KeywordAction::Landwalk(kind) => Some(match kind {
            crate::static_abilities::LandwalkKind::Subtype {
                subtype,
                snow: false,
            } => CompilerStaticAbility::landwalk(subtype),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype,
                snow: true,
            } => CompilerStaticAbility::snow_landwalk(subtype),
            crate::static_abilities::LandwalkKind::AnyLand => CompilerStaticAbility::any_landwalk(),
            crate::static_abilities::LandwalkKind::NonbasicLand => {
                CompilerStaticAbility::nonbasic_landwalk()
            }
            crate::static_abilities::LandwalkKind::ArtifactLand => {
                CompilerStaticAbility::artifact_landwalk()
            }
        }),
        KeywordAction::Bloodthirst(amount) => Some(CompilerStaticAbility::bloodthirst(amount)),
        KeywordAction::Tribute(amount) => Some(CompilerStaticAbility::tribute(amount)),
        KeywordAction::Rampage(_) | KeywordAction::Bushido(_) | KeywordAction::Frenzy(_) => None,
        KeywordAction::Changeling => Some(CompilerStaticAbility::changeling()),
        KeywordAction::ProtectionFrom(colors) => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        )),
        KeywordAction::ProtectionFromAllColors => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::AllColors,
        )),
        KeywordAction::ProtectionFromColorless => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::Colorless,
        )),
        KeywordAction::ProtectionFromEverything => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::Everything,
        )),
        KeywordAction::ProtectionFromChosenPlayer => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::ChosenPlayer,
        )),
        KeywordAction::ProtectionFromChosenColor => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::ChosenColor,
        )),
        KeywordAction::ProtectionFromFilter(filter) => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(filter),
        )),
        KeywordAction::ProtectionFromEachManaValueAmong(filter) => {
            Some(CompilerStaticAbility::protection(
                crate::ability::ProtectionFrom::EachManaValueAmong(filter),
            ))
        }
        KeywordAction::ProtectionFromCardType(card_type) => Some(
            CompilerStaticAbility::protection(crate::ability::ProtectionFrom::CardType(card_type)),
        ),
        KeywordAction::ProtectionFromSubtype(subtype) => Some(CompilerStaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(
                ObjectFilter::default().with_subtype(subtype),
            ),
        )),
        KeywordAction::Unblockable => Some(CompilerStaticAbility::unblockable()),
        KeywordAction::CantBeBlockedByMoreThan(count) => Some(
            CompilerStaticAbility::cant_be_blocked_by_more_than(count as usize),
        ),
        KeywordAction::Devoid => {
            Some(CompilerStaticAbility::make_colorless(ObjectFilter::source()))
        }
        KeywordAction::Annihilator(_) => None,
        KeywordAction::Dredge(amount) => Some(CompilerStaticAbility::dredge(amount)),
        KeywordAction::StaticMarker(name) => Some(CompilerStaticAbility::keyword_marker(name)),
        KeywordAction::StaticMarkerText(text) => Some(CompilerStaticAbility::keyword_marker(text)),
        KeywordAction::Marker(name) => Some(CompilerStaticAbility::keyword_fallback_text(name)),
        KeywordAction::MarkerText(text) => Some(CompilerStaticAbility::keyword_fallback_text(text)),
        _ => None,
    }
}

fn lower_keyword_action_or_err(
    action: KeywordAction,
) -> Result<RuntimeStaticAbility, CardTextError> {
    // Intrinsic object keywords already have one-to-one runtime abilities.
    // Lower this phase shard directly so temporary grants do not enter the
    // whole keyword/static-ability conversion pipeline just to recover the
    // same leaf ability.
    let intrinsic = match &action {
        KeywordAction::Flying => Some(RuntimeStaticAbility::flying()),
        KeywordAction::Menace => Some(RuntimeStaticAbility::menace()),
        KeywordAction::Banding => Some(RuntimeStaticAbility::banding()),
        KeywordAction::Haste => Some(RuntimeStaticAbility::haste()),
        KeywordAction::Hexproof => Some(RuntimeStaticAbility::hexproof()),
        KeywordAction::FirstStrike => Some(RuntimeStaticAbility::first_strike()),
        KeywordAction::DoubleStrike => Some(RuntimeStaticAbility::double_strike()),
        KeywordAction::Deathtouch => Some(RuntimeStaticAbility::deathtouch()),
        KeywordAction::Lifelink => Some(RuntimeStaticAbility::lifelink()),
        KeywordAction::Vigilance => Some(RuntimeStaticAbility::vigilance()),
        KeywordAction::Trample => Some(RuntimeStaticAbility::trample()),
        KeywordAction::Reach => Some(RuntimeStaticAbility::reach()),
        KeywordAction::Defender => Some(RuntimeStaticAbility::defender()),
        KeywordAction::Indestructible => Some(RuntimeStaticAbility::indestructible()),
        KeywordAction::Shroud => Some(RuntimeStaticAbility::shroud()),
        _ => None,
    };
    if let Some(intrinsic) = intrinsic {
        return Ok(intrinsic);
    }
    let ability = static_ability_for_keyword_action(action).ok_or_else(|| {
        CardTextError::InvariantViolation(
            "static-ability lowering received a non-static keyword action".to_string(),
        )
    })?;
    crate::lowering_support::lower_static_ability_ast(
        crate::cards::builders::StaticAbilityAst::Static(ability),
    )
}

pub fn lower_granted_ability_ast(
    ability: &GrantedAbilityAst,
) -> Result<RuntimeStaticAbility, CardTextError> {
    match ability {
        GrantedAbilityAst::KeywordAction(action) => lower_keyword_action_or_err((**action).clone()),
        GrantedAbilityAst::StaticAbility(ability) => {
            crate::lowering_support::lower_static_ability_ast((**ability).clone())
        }
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot lower as a static ability".to_string(),
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
    abilities.iter().map(lower_granted_ability_ast).collect()
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

pub fn afflict_triggered_ability(amount: u32) -> Ability {
    Ability {
        kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: Trigger::this_becomes_blocked(),
            effects: ResolutionProgram::from_effects(vec![Effect::lose_life_player(
                amount as i32,
                PlayerFilter::Defending,
            )]),
            choices: vec![],
            intervening_if: None,
            presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Afflict(
                amount,
            ))),
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
    }
}

pub fn decayed_object_abilities() -> Vec<Ability> {
    vec![
        Ability::static_ability(RuntimeStaticAbility::keyword_marker("decayed")),
        Ability::static_ability(RuntimeStaticAbility::cant_block()),
        decayed_triggered_ability(),
    ]
}

pub fn exalted_triggered_ability() -> Ability {
    let attacker_tag = crate::tag::CompilerReferenceTag::ExaltedAttacker.key();
    Ability::triggered(
        Trigger::attacks_alone(ObjectFilter::creature().you_control()),
        vec![
            Effect::tag_triggering_object(attacker_tag.clone()),
            Effect::pump(
                1,
                1,
                crate::target::ChooseSpec::Tagged(attacker_tag),
                crate::effect::Until::EndOfTurn,
            ),
        ],
    )
}

pub fn myriad_triggered_ability() -> Ability {
    let opponent_other_than_defending =
        PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending);
    Ability::triggered(
        Trigger::this_attacks(),
        vec![Effect::for_players(
            opponent_other_than_defending,
            vec![Effect::may(vec![Effect::new(
                crate::effects::CreateTokenCopyEffect::new(
                    crate::target::ChooseSpec::Source,
                    1,
                    PlayerFilter::You,
                )
                .enters_tapped(true)
                .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer)
                .exile_at_eoc(true),
            )])],
        )],
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
            functional_zones: vec![crate::zone::Zone::Exile],
        },
        Ability {
            kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: Trigger::new(crate::triggers::CounterRemovedFromTrigger::new(
                    ObjectFilter::source(),
                )),
                effects: ResolutionProgram::from_effects(vec![Effect::may(vec![Effect::new(
                    crate::effects::CastSourceEffect::new()
                        .without_paying_mana_cost()
                        .require_exile()
                        .cast_as_suspend(),
                )])]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasNoCounter(
                    crate::object::CounterType::Time,
                )),
                presentation_label: Some(PresentationLabel::Keyword(PresentationKeyword::Suspend)),
            }),
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
        .same_stable_id_as_tagged(trigger_tag);

    Ability {
        kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: ResolutionProgram::from_effects(vec![
                Effect::tag_triggering_object(trigger_tag),
                Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                    filter, return_tag,
                )),
                Effect::new(
                    crate::effects::MoveToZoneEffect::new(
                        crate::target::ChooseSpec::Tagged(return_tag.into()),
                        crate::zone::Zone::Battlefield,
                        true,
                    )
                    .under_owner_control(),
                )
                .tag(returned_tag),
                Effect::for_each_tagged(
                    returned_tag,
                    vec![Effect::put_counters(
                        counter_type,
                        1,
                        crate::target::ChooseSpec::Iterated,
                    )],
                ),
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
        GrantedAbilityAst::KeywordAction(action) => {
            if let KeywordAction::CumulativeUpkeep { total_cost, .. } = action.as_ref() {
                return crate::lowering_support::lower_compiler_ability_core(
                    ironsmith_compiler_semantic::keyword_abilities::cumulative_upkeep_granted_ability(total_cost.clone()),
                    None,
                );
            }
            let static_ability = lower_keyword_action_or_err((**action).clone())?;
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::StaticAbility(static_ability) => Ok(Ability::static_ability(
            crate::lowering_support::lower_static_ability_ast((**static_ability).clone())?,
        )),
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot lower as a granted object ability".to_string(),
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
            let lowered = lower_parsed_ability((**ability).clone())?;
            Ok(lowered)
        }
    }
}

/// Preserve a parsed granted ability in the compiler-owned generic ability
/// algebra. Runtime effects, triggers, and costs are materialized only by the
/// lowering module.
pub fn compiler_granted_ability_ast_to_object_ability(
    ability: &GrantedAbilityAst,
) -> Result<crate::model::compiler_semantic::CompilerAbilityCore, CardTextError> {
    use crate::model::compiler_semantic::CompilerAbilityCore;

    let static_ability = |ability| Ok(CompilerAbilityCore::static_ability(ability));
    match ability {
        GrantedAbilityAst::KeywordAction(action) => static_ability(
            static_ability_for_keyword_action((**action).clone()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "keyword grant requires executable lowering: {action:?}"
                ))
            })?,
        ),
        GrantedAbilityAst::StaticAbility(ability) => match ability.as_ref() {
            crate::cards::builders::StaticAbilityAst::Static(ability) => {
                static_ability(ability.clone())
            }
            other => Err(CardTextError::ParseError(format!(
                "nested static grant requires structural lowering: {other:?}"
            ))),
        },
        GrantedAbilityAst::MustAttack => static_ability(CompilerStaticAbility::must_attack()),
        GrantedAbilityAst::MustBlock => static_ability(CompilerStaticAbility::must_block()),
        GrantedAbilityAst::CanAttackAsThoughNoDefender => {
            static_ability(CompilerStaticAbility::can_attack_as_though_no_defender())
        }
        GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional } => static_ability(
            CompilerStaticAbility::can_block_additional_creature_each_combat(*additional),
        ),
        GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
            let mut compiler_ability = (*ability.ability).clone();
            if let Some(effects) = &ability.effects_ast {
                match &mut compiler_ability.kind {
                    crate::model::CompilerAbilityKindCore::Triggered(triggered) => {
                        triggered.effects =
                            ironsmith_core::ResolutionProgram::from_effects(effects.clone());
                    }
                    crate::model::CompilerAbilityKindCore::Activated(activated) => {
                        activated.effects =
                            ironsmith_core::ResolutionProgram::from_effects(effects.clone());
                    }
                    _ => {}
                }
            }
            if let Some(trigger) = &ability.trigger_spec
                && let crate::model::CompilerAbilityKindCore::Triggered(triggered) =
                    &mut compiler_ability.kind
            {
                triggered.trigger = (**trigger).clone();
            }
            Ok(compiler_ability)
        }
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot be stored as an independent object ability".to_string(),
        )),
    }
}

pub fn compiler_granted_abilities_ast_to_object_abilities(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<crate::model::compiler_semantic::CompilerAbilityCore>, CardTextError> {
    abilities
        .iter()
        .map(compiler_granted_ability_ast_to_object_ability)
        .collect()
}

pub fn lower_granted_abilities_ast_to_object_abilities(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<Ability>, CardTextError> {
    let mut lowered = Vec::new();
    for ability in abilities {
        if let GrantedAbilityAst::KeywordAction(action) = ability
            && let Some(executable) =
                executable_object_abilities_for_keyword_action(action.as_ref())
        {
            lowered.extend(executable);
            continue;
        }
        match ability {
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Afflict(_)) =>
            {
                let KeywordAction::Afflict(amount) = action.as_ref() else {
                    unreachable!();
                };
                lowered.push(afflict_triggered_ability(*amount));
            }
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
                if matches!(action.as_ref(), KeywordAction::Myriad) =>
            {
                lowered.push(myriad_triggered_ability());
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

    const EXECUTABLE_MARKER_BACKED_KEYWORDS: &[&str] = &[
        "afterlife 2",
        "fabricate 2",
        "prowess",
        "storm",
        "toxic 2",
        "battle cry",
        "dethrone",
        "evolve",
        "ingest",
        "mentor",
        "training",
        "riot",
        "renown 2",
        "modular 2",
        "graft 2",
        "soulbond",
        "soulshift 2",
        "outlast {1}{W}",
        "unearth {1}{B}",
        "eternalize {2}{B}",
        "ninjutsu {1}{U}",
        "extort",
        "sunburst",
        "fading 2",
        "vanishing 2",
        "rampage 2",
        "bushido 2",
        "frenzy 2",
        "poisonous 2",
        "annihilator 2",
    ];
}
