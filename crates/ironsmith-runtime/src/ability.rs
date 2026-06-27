use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility as NewStaticAbility;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;

pub type Ability =
    ironsmith_core::Ability<NewStaticAbility, Trigger, crate::effect::Effect, crate::costs::Cost>;
pub type AbilityKind = ironsmith_core::AbilityKind<
    NewStaticAbility,
    Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
>;
pub type TriggeredAbility = ironsmith_core::TriggeredAbility<Trigger, crate::effect::Effect>;
pub type ActivatedAbility =
    ironsmith_core::ActivatedAbility<crate::effect::Effect, crate::costs::Cost>;
pub type LevelAbility = ironsmith_core::LevelAbility<NewStaticAbility>;
pub use ironsmith_core::{
    ActivatedPresentationLabel, ActivationTiming, ManaUsageRestriction,
    ManaUsageSubtypeRequirement, PresentationKeyword, PresentationLabel, ProtectionFrom,
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

#[cfg(test)]
pub(crate) fn ability_surface_text_for_tests(ability: &Ability) -> Option<String> {
    Some(crate::compiled_text::ability_surface_text_for_tests(
        ability,
    ))
}

pub trait ActivatedAbilityRuntimeExt {
    fn could_add_mana(
        &self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> bool;

    fn is_runtime_mana_ability(
        &self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> bool;

    fn inferred_mana_symbols(
        &self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Vec<ManaSymbol>;
}

impl ActivatedAbilityRuntimeExt for ActivatedAbility {
    fn could_add_mana(
        &self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> bool {
        self.mana_output.is_some()
            || selected_resolution_effects_for_current_state(
                &self.effects,
                game,
                source,
                controller,
            )
            .into_iter()
            .any(|effect| effect.could_produce_mana(game, source, controller))
    }

    fn is_runtime_mana_ability(
        &self,
        game: &crate::game_state::GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> bool {
        self.could_add_mana(game, source, controller)
            && !self.has_targets()
            && !self.is_loyalty_ability()
    }

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
        for effect in
            selected_resolution_effects_for_current_state(&self.effects, game, source, controller)
        {
            effect.collect_producible_mana_symbols(game, source, controller, &mut inferred);
        }
        canonical_mana_symbols(inferred)
    }
}

pub fn selected_resolution_effects_for_current_state<'a>(
    program: &'a ResolutionProgram,
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
) -> Vec<&'a crate::effect::Effect> {
    let mut selected = Vec::new();

    for segment in &program.segments {
        if segment.self_replacements.is_empty() {
            selected.extend(segment.default_effects.iter());
            continue;
        }

        let mut applicable: Option<&[crate::effect::Effect]> = None;
        for branch in &segment.self_replacements {
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let ctx = crate::effects::ExecutionContext::new(source, controller, &mut dm);
            let condition_matches =
                crate::condition_eval::evaluate_condition_resolution(game, &branch.condition, &ctx)
                    .unwrap_or(false);
            if !condition_matches {
                continue;
            }
            if applicable.is_some() {
                applicable = Some(&[]);
                break;
            }
            applicable = Some(branch.replacement_effects.as_slice());
        }

        selected.extend(
            applicable
                .unwrap_or(segment.default_effects.as_slice())
                .iter(),
        );
    }

    selected
}

pub fn effects_could_add_mana(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effects: &[crate::effect::Effect],
) -> bool {
    effects
        .iter()
        .any(|effect| effect.could_produce_mana(game, source, controller))
}

pub fn effect_could_add_mana(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effect: &crate::effect::Effect,
) -> bool {
    effect.could_produce_mana(game, source, controller)
}

fn canonical_mana_symbols(symbols: Vec<ManaSymbol>) -> Vec<ManaSymbol> {
    let mut inferred = Vec::new();
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
    inferred
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
    Ability::static_ability(NewStaticAbility::flying())
}

pub fn first_strike() -> Ability {
    Ability::static_ability(NewStaticAbility::first_strike())
}

pub fn double_strike() -> Ability {
    Ability::static_ability(NewStaticAbility::double_strike())
}

pub fn deathtouch() -> Ability {
    Ability::static_ability(NewStaticAbility::deathtouch())
}

pub fn lifelink() -> Ability {
    Ability::static_ability(NewStaticAbility::lifelink())
}

pub fn vigilance() -> Ability {
    Ability::static_ability(NewStaticAbility::vigilance())
}

pub fn trample() -> Ability {
    Ability::static_ability(NewStaticAbility::trample())
}

pub fn haste() -> Ability {
    Ability::static_ability(NewStaticAbility::haste())
}

pub fn reach() -> Ability {
    Ability::static_ability(NewStaticAbility::reach())
}

pub fn defender() -> Ability {
    Ability::static_ability(NewStaticAbility::defender())
}

pub fn hexproof() -> Ability {
    Ability::static_ability(NewStaticAbility::hexproof())
}

pub fn indestructible() -> Ability {
    Ability::static_ability(NewStaticAbility::indestructible())
}

pub fn menace() -> Ability {
    Ability::static_ability(NewStaticAbility::menace())
}

pub fn flash() -> Ability {
    Ability::static_ability(NewStaticAbility::flash())
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
        assert_eq!(
            ability_surface_text_for_tests(&ability).as_deref(),
            Some("Flying")
        );
    }

    #[test]
    fn test_mana_ability() {
        let tap_for_green = Ability::mana(crate::cost::TotalCost::free(), vec![ManaSymbol::Green]);
        assert!(tap_for_green.is_mana_ability());
    }

    #[test]
    fn activated_ability_runtime_mana_detection_recurses_into_nested_effects() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(99_001);
        let ability = Ability::activated(
            crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            vec![Effect::for_players(
                PlayerFilter::Any,
                vec![Effect::for_each(
                    crate::filter::ObjectFilter::default(),
                    vec![Effect::add_mana(vec![ManaSymbol::Green])],
                )],
            )],
        );

        let AbilityKind::Activated(activated) = &ability.kind else {
            panic!("expected activated ability");
        };

        assert!(!activated.is_mana_ability());
        assert!(activated.could_add_mana(&game, source, alice));
        assert!(activated.is_runtime_mana_ability(&game, source, alice));
        assert_eq!(
            activated.inferred_mana_symbols(&game, source, alice),
            vec![ManaSymbol::Green]
        );
    }

    #[test]
    fn activated_ability_runtime_mana_detection_uses_generic_child_effect_hook() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(99_002);
        let ability = Ability::activated(
            crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            vec![Effect::repeat_effects(
                crate::effect::Value::Fixed(1),
                vec![Effect::add_mana(vec![ManaSymbol::Green])],
            )],
        );

        let AbilityKind::Activated(activated) = &ability.kind else {
            panic!("expected activated ability");
        };

        assert!(!activated.is_mana_ability());
        assert!(activated.could_add_mana(&game, source, alice));
        assert!(activated.is_runtime_mana_ability(&game, source, alice));
        assert_eq!(
            activated.inferred_mana_symbols(&game, source, alice),
            vec![ManaSymbol::Green]
        );
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
