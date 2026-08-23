use crate::ability::{Ability, PresentationKeyword, PresentationLabel};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, GrantedAbilityAst, KeywordAction,
};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;

use super::lowering_support::rewrite_lower_parsed_ability;

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
            Ability::static_ability(StaticAbility::keyword_marker("sunburst")),
            Ability::static_ability(
                StaticAbility::enters_with_counters_value(
                    crate::object::CounterType::PlusOnePlusOne,
                    crate::effect::Value::ColorsOfManaSpentToCastThisSpell,
                )
                .with_condition(creature_condition),
            ),
            Ability::static_ability(
                StaticAbility::enters_with_counters_value(
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

pub fn static_ability_for_keyword_action(action: KeywordAction) -> Option<StaticAbility> {
    if !action.lowers_to_static_ability() {
        return None;
    }

    match action {
        KeywordAction::Flying => Some(StaticAbility::flying()),
        KeywordAction::Menace => Some(StaticAbility::menace()),
        KeywordAction::Banding => Some(StaticAbility::banding()),
        KeywordAction::Hexproof => Some(StaticAbility::hexproof()),
        KeywordAction::HexproofFrom(filter) => Some(StaticAbility::hexproof_from(filter.clone())),
        KeywordAction::Haste => Some(StaticAbility::haste()),
        KeywordAction::Improvise => Some(StaticAbility::improvise()),
        KeywordAction::Convoke => Some(StaticAbility::convoke()),
        KeywordAction::AffinityForArtifacts => Some(StaticAbility::affinity_for_artifacts()),
        KeywordAction::Delve => Some(StaticAbility::delve()),
        KeywordAction::FirstStrike => Some(StaticAbility::first_strike()),
        KeywordAction::DoubleStrike => Some(StaticAbility::double_strike()),
        KeywordAction::Deathtouch => Some(StaticAbility::deathtouch()),
        KeywordAction::Lifelink => Some(StaticAbility::lifelink()),
        KeywordAction::Vigilance => Some(StaticAbility::vigilance()),
        KeywordAction::Trample => Some(StaticAbility::trample()),
        KeywordAction::Reach => Some(StaticAbility::reach()),
        KeywordAction::Defender => Some(StaticAbility::defender()),
        KeywordAction::Decayed => Some(StaticAbility::cant_block()),
        KeywordAction::Flash => Some(StaticAbility::flash()),
        KeywordAction::Phasing => Some(StaticAbility::phasing()),
        KeywordAction::Indestructible => Some(StaticAbility::indestructible()),
        KeywordAction::Shroud => Some(StaticAbility::shroud()),
        KeywordAction::Daybound => Some(StaticAbility::daybound()),
        KeywordAction::Nightbound => Some(StaticAbility::nightbound()),
        KeywordAction::Ward(amount) => u8::try_from(amount).ok().map(|generic| {
            StaticAbility::ward(TotalCost::mana(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(generic),
            ])))
        }),
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Afflict(_) => None,
        KeywordAction::Amplify(_) => None,
        KeywordAction::Afterlife(_) | KeywordAction::Fabricate(_) => None,
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying
        | KeywordAction::Persist
        | KeywordAction::Prowess
        | KeywordAction::Exalted => None,
        KeywordAction::Cascade => Some(StaticAbility::cascade()),
        KeywordAction::Storm
        | KeywordAction::Gravestorm
        | KeywordAction::Toxic(_)
        | KeywordAction::Poisonous(_)
        | KeywordAction::BattleCry
        | KeywordAction::Dethrone
        | KeywordAction::Evolve
        | KeywordAction::Ingest
        | KeywordAction::Mentor => None,
        KeywordAction::Skulk => Some(StaticAbility::skulk()),
        KeywordAction::Training | KeywordAction::Riot => None,
        KeywordAction::Unleash => Some(StaticAbility::unleash()),
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
        KeywordAction::Partner => Some(StaticAbility::partner()),
        KeywordAction::StartYourEngines => Some(StaticAbility::start_your_engines()),
        KeywordAction::Assist => Some(StaticAbility::assist()),
        KeywordAction::SplitSecond => Some(StaticAbility::split_second()),
        KeywordAction::Rebound => Some(StaticAbility::rebound()),
        KeywordAction::Sunburst => None,
        KeywordAction::ReadAhead => Some(StaticAbility::read_ahead()),
        KeywordAction::Firebending(_) | KeywordAction::FirebendingValue { .. } => None,
        KeywordAction::Fading(_) | KeywordAction::Vanishing(_) => None,
        KeywordAction::Fear => Some(StaticAbility::fear()),
        KeywordAction::Intimidate => Some(StaticAbility::intimidate()),
        KeywordAction::Shadow => Some(StaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(StaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(StaticAbility::flanking()),
        KeywordAction::UmbraArmor => Some(StaticAbility::umbra_armor()),
        KeywordAction::Landwalk(kind) => Some(match kind {
            crate::static_abilities::LandwalkKind::Subtype {
                subtype,
                snow: false,
            } => StaticAbility::landwalk(subtype),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype,
                snow: true,
            } => StaticAbility::snow_landwalk(subtype),
            crate::static_abilities::LandwalkKind::AnyLand => StaticAbility::any_landwalk(),
            crate::static_abilities::LandwalkKind::NonbasicLand => {
                StaticAbility::nonbasic_landwalk()
            }
            crate::static_abilities::LandwalkKind::ArtifactLand => {
                StaticAbility::artifact_landwalk()
            }
        }),
        KeywordAction::Bloodthirst(amount) => Some(StaticAbility::bloodthirst(amount)),
        KeywordAction::Tribute(amount) => Some(StaticAbility::tribute(amount)),
        KeywordAction::Rampage(_) | KeywordAction::Bushido(_) | KeywordAction::Frenzy(_) => None,
        KeywordAction::Changeling => Some(StaticAbility::changeling()),
        KeywordAction::ProtectionFrom(colors) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        )),
        KeywordAction::ProtectionFromAllColors => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::AllColors,
        )),
        KeywordAction::ProtectionFromColorless => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Colorless,
        )),
        KeywordAction::ProtectionFromEverything => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Everything,
        )),
        KeywordAction::ProtectionFromChosenPlayer => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::ChosenPlayer,
        )),
        KeywordAction::ProtectionFromChosenColor => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::ChosenColor,
        )),
        KeywordAction::ProtectionFromFilter(filter) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(filter),
        )),
        KeywordAction::ProtectionFromEachManaValueAmong(filter) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::EachManaValueAmong(filter),
        )),
        KeywordAction::ProtectionFromCardType(card_type) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        )),
        KeywordAction::ProtectionFromSubtype(subtype) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(
                ObjectFilter::default().with_subtype(subtype),
            ),
        )),
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::CantBeBlockedByMoreThan(count) => {
            Some(StaticAbility::cant_be_blocked_by_more_than(count as usize))
        }
        KeywordAction::Devoid => Some(StaticAbility::make_colorless(ObjectFilter::source())),
        KeywordAction::Annihilator(_) => None,
        KeywordAction::Dredge(amount) => Some(StaticAbility::dredge(amount)),
        KeywordAction::StaticMarker(name) => Some(StaticAbility::keyword_marker(name)),
        KeywordAction::StaticMarkerText(text) => Some(StaticAbility::keyword_marker(text)),
        KeywordAction::Marker(name) => Some(StaticAbility::keyword_fallback_text(name)),
        KeywordAction::MarkerText(text) => Some(StaticAbility::keyword_fallback_text(text)),
        _ => None,
    }
}

fn lower_keyword_action_or_err(action: KeywordAction) -> Result<StaticAbility, CardTextError> {
    static_ability_for_keyword_action(action).ok_or_else(|| {
        CardTextError::InvariantViolation(
            "static-ability lowering received a non-static keyword action".to_string(),
        )
    })
}

pub fn lower_granted_ability_ast(
    ability: &GrantedAbilityAst,
) -> Result<StaticAbility, CardTextError> {
    match ability {
        GrantedAbilityAst::KeywordAction(action) => lower_keyword_action_or_err(action.clone()),
        GrantedAbilityAst::StaticAbility(ability) => Ok(ability.clone()),
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot lower as a static ability".to_string(),
        )),
        GrantedAbilityAst::MustAttack => Ok(StaticAbility::must_attack()),
        GrantedAbilityAst::MustBlock => Ok(StaticAbility::must_block()),
        GrantedAbilityAst::CanAttackAsThoughNoDefender => {
            Ok(StaticAbility::can_attack_as_though_no_defender())
        }
        GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional } => Ok(
            StaticAbility::can_block_additional_creature_each_combat(*additional),
        ),
        GrantedAbilityAst::ParsedObjectAbility { ability, display } => {
            let lowered = rewrite_lower_parsed_ability(ability.clone())?.into_runtime();
            Ok(StaticAbility::grant_object_ability_for_filter(
                ObjectFilter::source(),
                lowered,
                display.clone(),
            ))
        }
    }
}

pub fn lower_granted_abilities_ast(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<StaticAbility>, CardTextError> {
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
        kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
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
        Ability::static_ability(StaticAbility::keyword_marker("decayed")),
        Ability::static_ability(StaticAbility::cant_block()),
        decayed_triggered_ability(),
    ]
}

pub fn exalted_triggered_ability() -> Ability {
    let attacker_tag = crate::tag::TagKey::from("exalted_attacker");
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
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
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
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
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
        kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
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
        GrantedAbilityAst::KeywordAction(KeywordAction::CumulativeUpkeep {
            total_cost, ..
        }) => Ok(super::keyword_static::cumulative_upkeep_granted_ability(
            total_cost.clone(),
        )),
        GrantedAbilityAst::KeywordAction(action) => {
            let static_ability = lower_keyword_action_or_err(action.clone())?;
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::StaticAbility(static_ability) => {
            Ok(Ability::static_ability(static_ability.clone()))
        }
        GrantedAbilityAst::ThisAbility => Err(CardTextError::InvariantViolation(
            "this ability cannot lower as a granted object ability".to_string(),
        )),
        GrantedAbilityAst::MustAttack => {
            let static_ability = StaticAbility::must_attack();
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::MustBlock => {
            let static_ability = StaticAbility::must_block();
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::CanAttackAsThoughNoDefender => {
            let static_ability = StaticAbility::can_attack_as_though_no_defender();
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional } => {
            let static_ability =
                StaticAbility::can_block_additional_creature_each_combat(*additional);
            Ok(Ability::static_ability(static_ability))
        }
        GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
            let lowered = rewrite_lower_parsed_ability(ability.clone())?.into_runtime();
            Ok(lowered)
        }
    }
}

pub fn lower_granted_abilities_ast_to_object_abilities(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<Ability>, CardTextError> {
    let mut lowered = Vec::new();
    for ability in abilities {
        if let GrantedAbilityAst::KeywordAction(action) = ability
            && let Some(executable) = executable_object_abilities_for_keyword_action(action)
        {
            lowered.extend(executable);
            continue;
        }
        match ability {
            GrantedAbilityAst::KeywordAction(KeywordAction::Afflict(amount)) => {
                lowered.push(afflict_triggered_ability(*amount));
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::Decayed) => {
                lowered.extend(decayed_object_abilities());
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::Persist) => {
                lowered.push(persist_triggered_ability());
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::Undying) => {
                lowered.push(undying_triggered_ability());
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::Exalted) => {
                lowered.push(exalted_triggered_ability());
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::Myriad) => {
                lowered.push(myriad_triggered_ability());
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::Marker("suspend")) => {
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
) -> Result<Vec<StaticAbility>, CardTextError> {
    if abilities
        .iter()
        .all(|ability| matches!(ability.kind, crate::ability::AbilityKind::Static(_)))
    {
        return Ok(abilities
            .into_iter()
            .filter_map(|ability| match ability.kind {
                crate::ability::AbilityKind::Static(static_ability) => Some(static_ability),
                _ => None,
            })
            .collect());
    }

    let mut abilities = abilities.into_iter();
    let first = abilities.next().ok_or_else(|| {
        CardTextError::InvariantViolation("keyword grant produced no abilities".to_string())
    })?;
    Ok(vec![StaticAbility::new(
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

    fn parsed_keyword_action(keyword: &str) -> KeywordAction {
        let tokens =
            crate::lexer::lex_line(keyword, 0).unwrap_or_else(|error| panic!("{keyword}: {error}"));
        let mut actions = super::super::keyword_static::parse_ability_line(&tokens)
            .unwrap_or_else(|| panic!("{keyword} should parse as a keyword action"));
        assert_eq!(actions.len(), 1, "{keyword} should parse as one action");
        actions.pop().expect("one keyword action")
    }

    #[test]
    fn every_marker_backed_gameplay_keyword_grant_lowers_to_printed_object_abilities() {
        for keyword in EXECUTABLE_MARKER_BACKED_KEYWORDS {
            let action = parsed_keyword_action(keyword);
            let expected = executable_object_abilities_for_keyword_action(&action)
                .unwrap_or_else(|| panic!("{keyword} should have an executable expansion"));
            assert!(
                expected.iter().any(|ability| !matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability)
                        if static_ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
                )),
                "{keyword} must not expand to presentation markers only"
            );

            let definition = CardDefinitionBuilder::new(crate::CardId::new(), "Grant Probe")
                .card_types(vec![CardType::Instant])
                .parse_text(format!(
                    "Target creature gains {keyword} until end of turn."
                ))
                .unwrap_or_else(|error| panic!("dynamic {keyword} grant should compile: {error}"));
            let debug = format!("{:#?}", definition.spell_effect);
            let lowered_count =
                debug.matches("AddAbilityGeneric").count() + debug.matches("AddAbility(").count();
            assert!(
                lowered_count >= expected.len(),
                "dynamic {keyword} grant must carry the complete printed ability set: {debug}"
            );
        }
    }

    #[test]
    fn firebending_grants_lower_to_the_executable_attack_trigger() {
        let definition =
            CardDefinitionBuilder::new(crate::CardId::new(), "Firebending Grant Probe")
                .card_types(vec![CardType::Instant])
                .parse_text("Target creature gains firebending 2 until end of turn.")
                .expect("Firebending grants should lower to executable object abilities");
        let debug = format!("{definition:#?}");
        assert!(debug.contains("ThisAttacks"), "{debug}");
        assert!(debug.contains("ManaRetainedEffect"), "{debug}");
        assert!(debug.contains("Firebend"), "{debug}");
    }
}
