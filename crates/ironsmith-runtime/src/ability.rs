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
            || effects_could_add_mana(
                game,
                source,
                controller,
                self.effects.flattened_default_effects(),
            )
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
        for effect in self.effects.flattened_default_effects() {
            collect_inferred_mana_symbols(game, source, controller, effect, &mut inferred);
        }

        inferred
    }
}

pub fn effects_could_add_mana(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effects: &[crate::effect::Effect],
) -> bool {
    effects
        .iter()
        .any(|effect| effect_could_add_mana(game, source, controller, effect))
}

pub fn effect_could_add_mana(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effect: &crate::effect::Effect,
) -> bool {
    if effect
        .producible_mana_symbols(game, source, controller)
        .is_some_and(|symbols| !symbols.is_empty())
    {
        return true;
    }

    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return effects_could_add_mana(game, source, controller, &sequence.effects);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return effects_could_add_mana(game, source, controller, &may.effects);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return effects_could_add_mana(game, source, controller, &conditional.if_true)
            || effects_could_add_mana(game, source, controller, &conditional.if_false);
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return effects_could_add_mana(game, source, controller, &if_effect.then)
            || effects_could_add_mana(game, source, controller, &if_effect.else_);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_could_add_mana(game, source, controller, &with_id.effect);
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        return choose_mode
            .modes
            .iter()
            .any(|mode| effects_could_add_mana(game, source, controller, &mode.effects));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_could_add_mana(game, source, controller, &tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return effect_could_add_mana(game, source, controller, &tag_all.effect);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        return effects_could_add_mana(game, source, controller, &for_each.effects);
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        return effects_could_add_mana(game, source, controller, &for_players.effects);
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return effects_could_add_mana(game, source, controller, &for_each_tagged.effects);
    }
    if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return effects_could_add_mana(game, source, controller, &for_each_controller.effects);
    }
    if let Some(for_each_tagged_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        return effects_could_add_mana(game, source, controller, &for_each_tagged_player.effects);
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        return effects_could_add_mana(game, source, controller, &unless_action.effects)
            || effects_could_add_mana(game, source, controller, &unless_action.alternative);
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        return effects_could_add_mana(game, source, controller, &unless_pays.effects);
    }

    false
}

fn collect_inferred_mana_symbols(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effect: &crate::effect::Effect,
    inferred: &mut Vec<ManaSymbol>,
) {
    if let Some(symbols) = effect.producible_mana_symbols(game, source, controller) {
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

    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &sequence.effects,
            inferred,
        );
    } else if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &may.effects,
            inferred,
        );
    } else if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &conditional.if_true,
            inferred,
        );
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &conditional.if_false,
            inferred,
        );
    } else if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &if_effect.then,
            inferred,
        );
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &if_effect.else_,
            inferred,
        );
    } else if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        collect_inferred_mana_symbols(game, source, controller, &with_id.effect, inferred);
    } else if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        for mode in &choose_mode.modes {
            collect_inferred_mana_symbols_from_effects(
                game,
                source,
                controller,
                &mode.effects,
                inferred,
            );
        }
    } else if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        collect_inferred_mana_symbols(game, source, controller, &tagged.effect, inferred);
    } else if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        collect_inferred_mana_symbols(game, source, controller, &tag_all.effect, inferred);
    } else if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &for_each.effects,
            inferred,
        );
    } else if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &for_players.effects,
            inferred,
        );
    } else if let Some(for_each_tagged) =
        effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &for_each_tagged.effects,
            inferred,
        );
    } else if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &for_each_controller.effects,
            inferred,
        );
    } else if let Some(for_each_tagged_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &for_each_tagged_player.effects,
            inferred,
        );
    } else if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>()
    {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &unless_action.effects,
            inferred,
        );
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &unless_action.alternative,
            inferred,
        );
    } else if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        collect_inferred_mana_symbols_from_effects(
            game,
            source,
            controller,
            &unless_pays.effects,
            inferred,
        );
    }
}

fn collect_inferred_mana_symbols_from_effects(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effects: &[crate::effect::Effect],
    inferred: &mut Vec<ManaSymbol>,
) {
    for effect in effects {
        collect_inferred_mana_symbols(game, source, controller, effect, inferred);
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
