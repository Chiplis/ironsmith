use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility as NewStaticAbility;
use crate::triggers::Trigger;
use crate::target::PlayerFilter;

pub type Ability =
    ironsmith_core::Ability<NewStaticAbility, Trigger, crate::effect::Effect, crate::costs::Cost>;
pub type AbilityKind = ironsmith_core::AbilityKind<
    NewStaticAbility,
    Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
>;
pub type TriggeredAbility = ironsmith_core::TriggeredAbility<Trigger, crate::effect::Effect>;
pub type ActivatedAbility = ironsmith_core::ActivatedAbility<crate::effect::Effect, crate::costs::Cost>;
pub type LevelAbility = ironsmith_core::LevelAbility<NewStaticAbility>;
pub use ironsmith_core::{
    ActivationTiming, ManaUsageRestriction, ManaUsageSubtypeRequirement, ProtectionFrom,
    RestrictedManaUnit,
};

pub fn extract_static_abilities(abilities: &[Ability]) -> Vec<NewStaticAbility> {
    abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.clone()),
            _ => None,
        })
        .collect()
}

pub trait ActivatedAbilityRuntimeExt {
    fn inferred_mana_symbols(
        &self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Vec<ManaSymbol>;
}

impl ActivatedAbilityRuntimeExt for ActivatedAbility {
    fn inferred_mana_symbols(
        &self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Vec<ManaSymbol> {
        let fixed = self.mana_symbols();
        if !fixed.is_empty() {
            return fixed.to_vec();
        }

        let mut inferred = Vec::new();
        for effect in self.effects.flattened_default_effects() {
            let Some(symbols) = effect.producible_mana_symbols(game, source, controller) else {
                continue;
            };
            for symbol in symbols {
                if !matches!(
                    symbol,
                    ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green
                        | ManaSymbol::Colorless
                ) {
                    continue;
                }
                if !inferred.contains(&symbol) {
                    inferred.push(symbol);
                }
            }
        }

        inferred
    }
}

#[derive(Debug, Clone)]
pub struct AbilityOnStack {
    pub source: ObjectId,
    pub controller: PlayerId,
    pub kind: StackedAbilityKind,
    pub targets: Vec<crate::game_state::Target>,
    pub x_value: Option<u32>,
    pub effects: ResolutionProgram,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StackedAbilityKind {
    Triggered,
    Activated,
}

pub fn flying() -> Ability {
    Ability::static_ability(NewStaticAbility::flying()).with_text("Flying")
}

pub fn first_strike() -> Ability {
    Ability::static_ability(NewStaticAbility::first_strike()).with_text("First strike")
}

pub fn double_strike() -> Ability {
    Ability::static_ability(NewStaticAbility::double_strike()).with_text("Double strike")
}

pub fn deathtouch() -> Ability {
    Ability::static_ability(NewStaticAbility::deathtouch()).with_text("Deathtouch")
}

pub fn lifelink() -> Ability {
    Ability::static_ability(NewStaticAbility::lifelink()).with_text("Lifelink")
}

pub fn vigilance() -> Ability {
    Ability::static_ability(NewStaticAbility::vigilance()).with_text("Vigilance")
}

pub fn trample() -> Ability {
    Ability::static_ability(NewStaticAbility::trample()).with_text("Trample")
}

pub fn haste() -> Ability {
    Ability::static_ability(NewStaticAbility::haste()).with_text("Haste")
}

pub fn reach() -> Ability {
    Ability::static_ability(NewStaticAbility::reach()).with_text("Reach")
}

pub fn defender() -> Ability {
    Ability::static_ability(NewStaticAbility::defender()).with_text("Defender")
}

pub fn hexproof() -> Ability {
    Ability::static_ability(NewStaticAbility::hexproof()).with_text("Hexproof")
}

pub fn indestructible() -> Ability {
    Ability::static_ability(NewStaticAbility::indestructible()).with_text("Indestructible")
}

pub fn menace() -> Ability {
    Ability::static_ability(NewStaticAbility::menace()).with_text("Menace")
}

pub fn flash() -> Ability {
    Ability::static_ability(NewStaticAbility::flash()).with_text("Flash")
}

pub fn etb_trigger(effects: impl Into<ResolutionProgram>) -> Ability {
    Ability::triggered(Trigger::this_enters_battlefield(), effects)
}

pub fn dies_trigger(effects: impl Into<ResolutionProgram>) -> Ability {
    Ability::triggered(Trigger::this_dies(), effects)
}

pub fn upkeep_trigger(effects: impl Into<ResolutionProgram>) -> Ability {
    Ability::triggered(Trigger::beginning_of_upkeep(PlayerFilter::You), effects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_static_ability() {
        let ability = flying();
        if let AbilityKind::Static(s) = &ability.kind {
            assert_eq!(s.id(), StaticAbilityId::Flying);
        } else {
            panic!("Expected static ability");
        }
        assert_eq!(ability.text, Some("Flying".to_string()));
    }

    #[test]
    fn test_mana_ability() {
        let tap_for_green = Ability::mana(crate::cost::TotalCost::free(), vec![ManaSymbol::Green]);
        assert!(tap_for_green.is_mana_ability());
    }

    #[test]
    fn test_triggered_ability() {
        let ability = etb_trigger(vec![Effect::draw(1)]);

        if let AbilityKind::Triggered(triggered) = &ability.kind {
            assert!(
                triggered
                    .trigger
                    .display()
                    .contains("enters the battlefield")
            );
        } else {
            panic!("Expected triggered ability");
        }
    }
}
