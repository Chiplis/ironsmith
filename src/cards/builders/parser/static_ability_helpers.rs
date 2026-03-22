use crate::cards::builders::{CardTextError, GrantedAbilityAst, KeywordAction};
use crate::cost::TotalCost;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;

use super::lowering_support::rewrite_lower_parsed_ability;

pub(crate) fn static_ability_for_keyword_action(action: KeywordAction) -> Option<StaticAbility> {
    if !action.lowers_to_static_ability() {
        return None;
    }

    match action {
        KeywordAction::Flying => Some(StaticAbility::flying()),
        KeywordAction::Menace => Some(StaticAbility::menace()),
        KeywordAction::Hexproof => Some(StaticAbility::hexproof()),
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
        KeywordAction::Ninjutsu(cost) => Some(StaticAbility::keyword_marker(format!(
            "ninjutsu {}",
            cost.to_oracle()
        ))),
        KeywordAction::Extort => Some(StaticAbility::keyword_marker("extort".to_string())),
        KeywordAction::Partner => Some(StaticAbility::partner()),
        KeywordAction::Assist => Some(StaticAbility::assist()),
        KeywordAction::SplitSecond => Some(StaticAbility::split_second()),
        KeywordAction::Rebound => Some(StaticAbility::rebound()),
        KeywordAction::Sunburst => Some(StaticAbility::keyword_marker("sunburst".to_string())),
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
        KeywordAction::Landwalk(subtype) => Some(StaticAbility::landwalk(subtype)),
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
        KeywordAction::ProtectionFromCardType(card_type) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        )),
        KeywordAction::ProtectionFromSubtype(subtype) => {
            Some(StaticAbility::keyword_marker(format!(
                "protection from {}",
                subtype.to_string().to_ascii_lowercase()
            )))
        }
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::Devoid => Some(StaticAbility::make_colorless(ObjectFilter::source())),
        KeywordAction::Annihilator(amount) => Some(StaticAbility::keyword_marker(format!(
            "annihilator {amount}"
        ))),
        KeywordAction::Marker(name) => Some(StaticAbility::keyword_marker(name)),
        KeywordAction::MarkerText(text) => Some(StaticAbility::keyword_marker(text)),
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

pub(crate) fn lower_granted_ability_ast(
    ability: &GrantedAbilityAst,
) -> Result<StaticAbility, CardTextError> {
    match ability {
        GrantedAbilityAst::KeywordAction(action) => lower_keyword_action_or_err(action.clone()),
        GrantedAbilityAst::MustAttack => Ok(StaticAbility::must_attack()),
        GrantedAbilityAst::MustBlock => Ok(StaticAbility::must_block()),
        GrantedAbilityAst::CanAttackAsThoughNoDefender => {
            Ok(StaticAbility::can_attack_as_though_no_defender())
        }
        GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat { additional } => Ok(
            StaticAbility::can_block_additional_creature_each_combat(*additional),
        ),
        GrantedAbilityAst::ParsedObjectAbility { ability, display } => {
            let mut lowered = rewrite_lower_parsed_ability(ability.clone())?.ability;
            lowered.text = Some(display.clone());
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
