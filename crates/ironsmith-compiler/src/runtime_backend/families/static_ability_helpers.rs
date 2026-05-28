use crate::ability::Ability;
use crate::cards::builders::{CardTextError, GrantedAbilityAst, KeywordAction};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;

use super::lowering_support::rewrite_lower_parsed_ability;

pub(crate) fn static_ability_for_keyword_action(action: KeywordAction) -> Option<StaticAbility> {
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
        KeywordAction::Ward(amount) => u8::try_from(amount).ok().map(|generic| {
            StaticAbility::ward(TotalCost::mana(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(generic),
            ])))
        }),
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Afflict(_) => None,
        KeywordAction::Amplify(_) => None,
        KeywordAction::Afterlife(amount) => {
            Some(StaticAbility::keyword_marker(format!("afterlife {amount}")))
        }
        KeywordAction::Fabricate(amount) => {
            Some(StaticAbility::keyword_marker(format!("fabricate {amount}")))
        }
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying => Some(StaticAbility::keyword_marker("undying".to_string())),
        KeywordAction::Persist => Some(StaticAbility::keyword_marker("persist".to_string())),
        KeywordAction::Prowess => Some(StaticAbility::keyword_marker("prowess".to_string())),
        KeywordAction::Exalted => Some(StaticAbility::keyword_marker("exalted".to_string())),
        KeywordAction::Cascade => Some(StaticAbility::cascade()),
        KeywordAction::Storm => Some(StaticAbility::keyword_marker("storm".to_string())),
        KeywordAction::Toxic(amount) => {
            Some(StaticAbility::keyword_marker(format!("toxic {amount}")))
        }
        KeywordAction::BattleCry => Some(StaticAbility::keyword_marker("battle cry".to_string())),
        KeywordAction::Dethrone => Some(StaticAbility::keyword_marker("dethrone".to_string())),
        KeywordAction::Evolve => Some(StaticAbility::keyword_marker("evolve".to_string())),
        KeywordAction::Ingest => Some(StaticAbility::keyword_marker("ingest".to_string())),
        KeywordAction::Mentor => Some(StaticAbility::keyword_marker("mentor".to_string())),
        KeywordAction::Skulk => Some(StaticAbility::skulk()),
        KeywordAction::Training => Some(StaticAbility::keyword_marker("training".to_string())),
        KeywordAction::Riot => Some(StaticAbility::keyword_marker("riot".to_string())),
        KeywordAction::Unleash => Some(StaticAbility::unleash()),
        KeywordAction::Renown(amount) => {
            Some(StaticAbility::keyword_marker(format!("renown {amount}")))
        }
        KeywordAction::Modular(amount) => {
            Some(StaticAbility::keyword_marker(format!("modular {amount}")))
        }
        KeywordAction::Graft(amount) => {
            Some(StaticAbility::keyword_marker(format!("graft {amount}")))
        }
        KeywordAction::Soulbond => Some(StaticAbility::keyword_marker("soulbond".to_string())),
        KeywordAction::Soulshift(amount) => {
            Some(StaticAbility::keyword_marker(format!("soulshift {amount}")))
        }
        KeywordAction::Outlast(cost) => Some(StaticAbility::keyword_marker(format!(
            "outlast {}",
            cost.to_oracle()
        ))),
        KeywordAction::Unearth(cost) => Some(StaticAbility::keyword_marker(format!(
            "unearth {}",
            cost.to_oracle()
        ))),
        KeywordAction::Eternalize(cost) => Some(StaticAbility::keyword_marker(format!(
            "eternalize {}",
            cost.to_oracle()
        ))),
        KeywordAction::Ninjutsu(cost) => Some(StaticAbility::keyword_marker(format!(
            "ninjutsu {}",
            cost.to_oracle()
        ))),
        KeywordAction::Extort => Some(StaticAbility::keyword_marker("extort".to_string())),
        KeywordAction::Partner => Some(StaticAbility::partner()),
        KeywordAction::StartYourEngines => Some(StaticAbility::start_your_engines()),
        KeywordAction::Assist => Some(StaticAbility::assist()),
        KeywordAction::SplitSecond => Some(StaticAbility::split_second()),
        KeywordAction::Rebound => Some(StaticAbility::rebound()),
        KeywordAction::Sunburst => Some(StaticAbility::keyword_marker("sunburst".to_string())),
        KeywordAction::ReadAhead => Some(StaticAbility::read_ahead()),
        KeywordAction::Fading(amount) => {
            Some(StaticAbility::keyword_marker(format!("fading {amount}")))
        }
        KeywordAction::Vanishing(amount) => {
            Some(StaticAbility::keyword_marker(format!("vanishing {amount}")))
        }
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
        KeywordAction::Rampage(amount) => {
            Some(StaticAbility::keyword_marker(format!("rampage {amount}")))
        }
        KeywordAction::Bushido(amount) => {
            Some(StaticAbility::keyword_marker(format!("bushido {amount}")))
        }
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
        KeywordAction::Devoid => Some(StaticAbility::make_colorless(ObjectFilter::source())),
        KeywordAction::Annihilator(amount) => Some(StaticAbility::keyword_marker(format!(
            "annihilator {amount}"
        ))),
        KeywordAction::Marker(name) if supported_keyword_marker_text(name) => {
            Some(StaticAbility::keyword_marker(name))
        }
        KeywordAction::MarkerText(text) if supported_keyword_marker_text(&text) => {
            Some(StaticAbility::keyword_marker(text))
        }
        KeywordAction::Marker(name) => Some(StaticAbility::keyword_fallback_text(name)),
        KeywordAction::MarkerText(text) => Some(StaticAbility::keyword_fallback_text(text)),
        _ => None,
    }
}

fn supported_keyword_marker_text(text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    text == "compleated"
        || text.starts_with("prototype ")
        || text.starts_with("more than meets the eye ")
        || text.starts_with("splice onto ")
        || is_ticket_sticker_marker_line(&text)
        || text.starts_with("dredge ")
}

fn is_ticket_sticker_marker_line(text: &str) -> bool {
    let Some((cost, body_text)) = text.split_once('—') else {
        return false;
    };

    let mut saw_ticket_symbol = false;
    let mut remainder = cost.trim();
    while let Some(next) = remainder.strip_prefix("{tk}") {
        saw_ticket_symbol = true;
        remainder = next.trim_start();
    }
    if !saw_ticket_symbol || !remainder.is_empty() {
        return false;
    }

    !body_text.trim().is_empty()
}

fn lower_keyword_action_or_err(action: KeywordAction) -> Result<StaticAbility, CardTextError> {
    static_ability_for_keyword_action(action).ok_or_else(|| {
        CardTextError::InvariantViolation(
            "static-ability lowering received a non-static keyword action".to_string(),
        )
    })
}

pub(crate) fn lower_granted_ability_ast(
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

pub(crate) fn lower_granted_abilities_ast(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<StaticAbility>, CardTextError> {
    abilities.iter().map(lower_granted_ability_ast).collect()
}

pub(crate) fn decayed_triggered_ability() -> Ability {
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

pub(crate) fn decayed_object_abilities() -> Vec<Ability> {
    vec![
        Ability::static_ability(StaticAbility::cant_block()),
        decayed_triggered_ability(),
    ]
}

pub(crate) fn exalted_triggered_ability() -> Ability {
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

pub(crate) fn myriad_triggered_ability() -> Ability {
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

pub(crate) fn persist_triggered_ability() -> Ability {
    graveyard_return_counter_ability(
        crate::object::CounterType::MinusOneMinusOne,
        "persist_trigger",
        "persist_return",
        "persist_returned",
    )
}

pub(crate) fn undying_triggered_ability() -> Ability {
    graveyard_return_counter_ability(
        crate::object::CounterType::PlusOnePlusOne,
        "undying_trigger",
        "undying_return",
        "undying_returned",
    )
}

pub(crate) fn lower_granted_ability_ast_to_object_ability(
    ability: &GrantedAbilityAst,
) -> Result<Ability, CardTextError> {
    match ability {
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

pub(crate) fn lower_granted_abilities_ast_to_object_abilities(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<Ability>, CardTextError> {
    let mut lowered = Vec::new();
    for ability in abilities {
        match ability {
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
            _ => lowered.push(lower_granted_ability_ast_to_object_ability(ability)?),
        }
    }
    Ok(lowered)
}
