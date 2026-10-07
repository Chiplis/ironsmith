//! Resolution-time word choices, followed by a real layer-three substitution.
use crate::continuous::{ContinuousEffect, Modification};
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use ironsmith_core::{Color, Subtype, TextChange, TextChangeSelection, TextWord};
pub use ironsmith_core::ChangeTextEffect;

#[derive(Clone, Copy)]
enum Family { Color, BasicLand, Creature }

fn words(family: Family) -> Vec<TextWord> {
    match family {
        Family::Color => Color::ALL.into_iter().map(TextWord::Color).collect(),
        Family::BasicLand => [Subtype::Plains, Subtype::Island, Subtype::Swamp, Subtype::Mountain, Subtype::Forest]
            .into_iter().map(TextWord::BasicLandType).collect(),
        Family::Creature => Subtype::all_creature_types().iter().copied().map(TextWord::CreatureType).collect(),
    }
}

fn word_label(word: TextWord) -> String {
    match word { TextWord::Color(color) => color.name().to_string(),
        TextWord::BasicLandType(subtype) | TextWord::CreatureType(subtype) => subtype.to_string() }
}

fn choose_index(game: &GameState, ctx: &mut ExecutionContext, prompt: &str, labels: Vec<String>)
    -> Result<Option<usize>, ExecutionError>
{
    let length = labels.len();
    let options = labels.into_iter().enumerate().map(|(index, label)| SelectableOption::new(index, label)).collect();
    let choice = SelectOptionsContext::new(ctx.controller, Some(ctx.source), prompt, options, 1, 1);
    let selected = ctx.decision_maker.decide_options(game, &choice);
    if ctx.decision_maker.awaiting_choice() { return Ok(None); }
    let [index] = selected.as_slice() else {
        return Err(ExecutionError::InternalError("text change requires exactly one word choice".into()));
    };
    if *index >= length {
        return Err(ExecutionError::InternalError("text change choice is outside its declared vocabulary".into()));
    }
    Ok(Some(*index))
}

impl EffectExecutor for ChangeTextEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError>
    {
        self.selection.validate().map_err(|error| ExecutionError::InternalError(error.to_string()))?;
        let objects = crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?;
        if objects.is_empty() { return Ok(EffectOutcome::target_invalid()); }
        let family = match &self.selection {
            TextChangeSelection::Color => Family::Color,
            TextChangeSelection::BasicLand => Family::BasicLand,
            TextChangeSelection::Creature { .. } | TextChangeSelection::CreatureTo(_) => Family::Creature,
            TextChangeSelection::ColorOrBasicLand => {
                let Some(choice) = choose_index(game, ctx, "Choose which kind of word to change",
                    vec!["Color word".into(), "Basic land type".into()])? else { return Ok(EffectOutcome::count(0)); };
                [Family::Color, Family::BasicLand][choice]
            }
        };
        let choices = words(family);
        let Some(from_index) = choose_index(game, ctx, "Choose the word to replace",
            choices.iter().copied().map(word_label).collect())? else { return Ok(EffectOutcome::count(0)); };
        let from = choices[from_index];
        let to = if let TextChangeSelection::CreatureTo(subtype) = &self.selection {
            TextWord::CreatureType(*subtype)
        } else {
            let destinations: Vec<_> = choices.into_iter().filter(|word| {
                *word != from && !matches!((&self.selection, word),
                    (TextChangeSelection::Creature { excluded_new }, TextWord::CreatureType(subtype))
                        if excluded_new.contains(subtype))
            }).collect();
            let Some(to_index) = choose_index(game, ctx, "Choose the replacement word",
                destinations.iter().copied().map(word_label).collect())? else { return Ok(EffectOutcome::count(0)); };
            destinations[to_index]
        };
        // A fixed replacement word does not say “another.” Identity succeeds
        // without registering an effect; a nonidentity change is retained even
        // when its source word is absent because a later copy may introduce it.
        if from == to { return Ok(EffectOutcome::with_objects(objects)); }
        let change = TextChange::new(from, to).map_err(|error| ExecutionError::InternalError(error.to_string()))?;
        let id = game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
            ctx.source, ctx.controller, objects.clone(), Modification::RewriteText(change)).until(self.duration.clone()));
        ctx.created_continuous_effects.push(id);
        Ok(EffectOutcome::with_objects(objects))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> { self.target.is_target().then_some(&self.target) }
    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> { Some(self.target.count()) }
    fn target_description(&self) -> &'static str { "object whose text changes" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Ability, ProtectionFrom};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::effects::{EffectContext, execute_effect};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use ironsmith_core::{CardType, ColorSet, Until, Zone};
    use std::collections::VecDeque;

    const A: PlayerId = PlayerId::from_index(0);
    struct Choices { steps: VecDeque<Vec<usize>>, pending: bool }
    impl Choices {
        fn exact(steps: Vec<usize>) -> Self {
            Self { steps: steps.into_iter().map(|index| vec![index]).collect(), pending: false }
        }
    }
    impl DecisionMaker for Choices {
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
            match self.steps.pop_front() { Some(step) => step, None => { self.pending = true; Vec::new() } }
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn game_target() -> (GameState, ObjectId) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Red Human")
            .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Human])
            .color_indicator(ColorSet::RED).power_toughness(PowerToughness::fixed(2, 3)).build();
        let target = game.create_object_from_card(&card, A, Zone::Battlefield);
        (game, target)
    }
    fn color_index(color: Color) -> usize { Color::ALL.iter().position(|value| *value == color).unwrap() }
    fn destination_index(from: Color, to: Color) -> usize {
        Color::ALL.iter().filter(|value| **value != from).position(|value| *value == to).unwrap()
    }
    fn execute(game: &mut GameState, target: ObjectId, selection: TextChangeSelection, duration: Until, choices: &mut Choices)
        -> Result<EffectOutcome, ExecutionError>
    {
        execute_effect(game, &crate::effect::Effect::new(ChangeTextEffect::new(
            ChooseSpec::SpecificObject(target), selection, duration)), &mut EffectContext::new(target, A, choices))
    }

    #[test]
    fn color_choices_apply_real_text_layer_and_expire_without_changing_identity_or_color() {
        let (mut game, target) = game_target();
        let protection = StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED));
        let identity = protection.instance_id();
        game.object_mut(target).unwrap().abilities_mut().push(Ability::static_ability(protection));
        let mut choices = Choices::exact(vec![color_index(Color::Red), destination_index(Color::Red, Color::Blue)]);
        let outcome = execute(&mut game, target, TextChangeSelection::Color, Until::EndOfTurn, &mut choices).unwrap();
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Objects(vec![target]));
        let chars = game.calculated_characteristics(target).unwrap();
        assert_eq!(chars.name.as_ref(), "Red Human");
        assert_eq!(chars.colors, ColorSet::RED);
        assert_eq!(chars.static_abilities[0].instance_id(), identity);
        assert_eq!(chars.static_abilities[0].protection_from(), Some(&ProtectionFrom::Color(ColorSet::BLUE)));
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.calculated_characteristics(target).unwrap().static_abilities[0].protection_from(),
            Some(&ProtectionFrom::Color(ColorSet::RED)));
    }

    #[test]
    fn pending_or_invalid_word_choices_leave_no_partial_registration() {
        for invalid in [false, true] {
            let (mut game, target) = game_target();
            let before = game.effect_store.continuous_effects.effects().len();
            let mut choices = Choices::exact(vec![color_index(Color::Red)]);
            if invalid { choices.steps.push_back(vec![0, 1]); }
            let result = execute(&mut game, target, TextChangeSelection::Color, Until::Forever, &mut choices);
            if invalid { assert!(result.is_err()); } else { assert!(result.is_ok()); assert!(choices.pending); }
            assert_eq!(game.effect_store.continuous_effects.effects().len(), before);
            assert_eq!(game.object(target).unwrap().subtypes.to_vec(), vec![Subtype::Human]);
        }
    }

    #[test]
    fn source_word_may_be_absent_but_nonidentity_effect_stays_for_later_copy_values() {
        let (mut game, target) = game_target();
        let mut choices = Choices::exact(vec![color_index(Color::Blue), destination_index(Color::Blue, Color::Green)]);
        execute(&mut game, target, TextChangeSelection::Color, Until::Forever, &mut choices).unwrap();
        let effects = game.effect_store.continuous_effects.effects();
        assert!(effects.iter().any(|effect| matches!(effect.modification,
            Modification::RewriteText(change) if change == TextChange::color(Color::Blue, Color::Green).unwrap())));
        let donor = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Later copied definition")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 2)).build(), A, Zone::Battlefield);
        game.object_mut(donor).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::protection(ProtectionFrom::Color(ColorSet::BLUE))));
        let values = crate::snapshot::CopiableValues::from_object(game.object(donor).unwrap());
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(donor, A, vec![target],
            Modification::CopyOf { target_id: donor, copiable_values: Box::new(values),
                preserve_source_abilities: false, name_override: None, name_override_surface: None,
                add_supertypes: Vec::new() }));
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.calculated_characteristics(target).unwrap().static_abilities[0].protection_from(),
            Some(&ProtectionFrom::Color(ColorSet::GREEN)), "the earlier absent-word effect applies after layer-one copying");
    }

    #[test]
    fn fixed_vampire_destination_accepts_vampire_source_as_successful_identity_choice() {
        let (mut game, target) = game_target();
        let before = game.effect_store.continuous_effects.effects().len();
        let vampire = Subtype::all_creature_types().iter().position(|subtype| *subtype == Subtype::Vampire).unwrap();
        let mut choices = Choices::exact(vec![vampire]);
        let outcome = execute(&mut game, target, TextChangeSelection::CreatureTo(Subtype::Vampire), Until::Forever, &mut choices).unwrap();
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Objects(vec![target]));
        assert_eq!(game.effect_store.continuous_effects.effects().len(), before);
        assert_eq!(game.object(target).unwrap().subtypes.to_vec(), vec![Subtype::Human]);
        assert!(!choices.pending);
    }

    #[test]
    fn artificial_destination_exclusion_does_not_exclude_wall_as_the_source() {
        let (mut game, target) = game_target();
        game.object_mut(target).unwrap().subtypes = vec![Subtype::Wall].into();
        let source = Subtype::all_creature_types().iter().position(|subtype| *subtype == Subtype::Wall).unwrap();
        let destination = Subtype::all_creature_types().iter().filter(|subtype| **subtype != Subtype::Wall)
            .position(|subtype| *subtype == Subtype::Human).unwrap();
        let mut choices = Choices::exact(vec![source, destination]);
        execute(&mut game, target, TextChangeSelection::Creature { excluded_new: vec![Subtype::Wall] },
            Until::Forever, &mut choices).unwrap();
        assert_eq!(game.calculated_characteristics(target).unwrap().subtypes.to_vec(), vec![Subtype::Human]);
    }
    #[test]
    fn pending_new_blood_choice_rolls_back_control_but_keeps_previously_paid_tap_cost() {
        let (mut game, target) = game_target();
        let b = PlayerId::from_index(1);
        game.object_mut(target).unwrap().initial_controller = b;
        let payer = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Vampire payer")
            .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Vampire])
            .power_toughness(PowerToughness::fixed(1, 1)).build(), A, Zone::Battlefield);
        game.tap(payer);
        let control = crate::effects::ApplyContinuousEffect::with_spec_runtime(
            ChooseSpec::SpecificObject(target), crate::effects::RuntimeModification::ChangeControllerToEffectController, Until::Forever);
        let body = crate::resolution::ResolutionProgram::from_effects(vec![
            crate::effect::Effect::new(control),
            crate::effect::Effect::new(ChangeTextEffect::new(ChooseSpec::SpecificObject(target),
                TextChangeSelection::CreatureTo(Subtype::Vampire), Until::Forever)),
        ]);
        let mut pending = Choices::exact(Vec::new());
        let before = game.effect_store.continuous_effects.effects().len();
        crate::game_loop::execute_resolution_program_typed(&mut game,
            &mut EffectContext::new(payer, A, &mut pending), A, payer, &body, None, &[]).unwrap();
        assert!(pending.pending);
        assert_eq!(game.calculated_characteristics(target).unwrap().controller, b);
        assert_eq!(game.effect_store.continuous_effects.effects().len(), before);
        assert!(game.is_tapped(payer));
        let human = Subtype::all_creature_types().iter().position(|subtype| *subtype == Subtype::Human).unwrap();
        let mut ready = Choices::exact(vec![human]);
        crate::game_loop::execute_resolution_program_typed(&mut game,
            &mut EffectContext::new(payer, A, &mut ready), A, payer, &body, None, &[]).unwrap();
        let current = game.calculated_characteristics(target).unwrap();
        assert_eq!(current.controller, A);
        assert_eq!(current.subtypes.to_vec(), vec![Subtype::Vampire]);
        assert!(game.is_tapped(payer));
    }

    #[test]
    fn cumulative_upkeep_presence_is_typed_and_rechecks_the_current_ability_set() {
        use crate::filter::ObjectFilterExt as _;
        let (mut game, target) = game_target();
        let mut filter = ironsmith_core::ObjectFilter::creature();
        filter.has_cumulative_upkeep = Some(false);
        let context = crate::filter::FilterContext::new(A);
        assert!(filter.matches(game.object(target).unwrap(), &context, &game));
        let echo = crate::effects::CumulativeUpkeepEffect::echo(crate::target::PlayerFilter::You, Vec::new(), Vec::new());
        game.object_mut(target).unwrap().abilities_mut().push(Ability::triggered(
            crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
            vec![crate::effect::Effect::new(echo)]));
        assert!(filter.matches(game.object(target).unwrap(), &context, &game), "echo is a distinct typed mechanic");
        let upkeep = Ability::triggered(crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
            vec![crate::effect::Effect::new(crate::effects::CumulativeUpkeepEffect::new(
                crate::target::PlayerFilter::You, Vec::new(), Vec::new()))]);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(target, A,
            vec![target], Modification::AddAbilityGeneric(upkeep)));
        game.refresh_continuous_state().unwrap();
        assert!(!filter.matches(game.object(target).unwrap(), &context, &game), "a newly granted upkeep invalidates the original target");
    }

}
