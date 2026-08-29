use crate::cards::builders::{CardTextError, GrantedAbilityAst, KeywordAction};
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::CompilerStaticAbilityCore as CompilerStaticAbility;

/// Convert a recognized keyword into the compiler-owned static-ability
/// algebra. Executable marker-backed keywords are expanded only after the
/// front-end/lowering boundary.
pub fn static_ability_for_keyword_action(action: KeywordAction) -> Option<CompilerStaticAbility> {
    if !action.lowers_to_static_ability() {
        return None;
    }

    match action {
        KeywordAction::Flying => Some(CompilerStaticAbility::flying()),
        KeywordAction::Menace => Some(CompilerStaticAbility::menace()),
        KeywordAction::Banding => Some(CompilerStaticAbility::banding()),
        KeywordAction::Hexproof => Some(CompilerStaticAbility::hexproof()),
        KeywordAction::HexproofFrom(filter) => Some(CompilerStaticAbility::hexproof_from(filter)),
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
        KeywordAction::Afflict(_)
        | KeywordAction::Amplify(_)
        | KeywordAction::Afterlife(_)
        | KeywordAction::Fabricate(_)
        | KeywordAction::Undying
        | KeywordAction::Persist
        | KeywordAction::Prowess
        | KeywordAction::Exalted => None,
        KeywordAction::Infect => Some(CompilerStaticAbility::infect()),
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
