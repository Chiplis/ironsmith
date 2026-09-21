//! Exchange text boxes effect implementation.

use crate::continuous::{Modification, TextBoxOverlay, text_box_characteristics_with_effects};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::types::CardType;
pub use ironsmith_core::ExchangeTextBoxesEffect;

/// Effect that exchanges the text boxes of exactly two creatures.
fn is_current_creature(game: &GameState, object_id: crate::ids::ObjectId) -> bool {
    let Some(object) = game.object(object_id) else {
        return false;
    };
    game.calculated_characteristics(object_id)
        .map(|chars| chars.card_types.contains(&CardType::Creature))
        .unwrap_or_else(|| object.card_types.contains(&CardType::Creature))
}

fn current_text_box_overlay(
    game: &GameState,
    object_id: crate::ids::ObjectId,
    entry: Option<&crate::events::EnterBattlefieldEvent>,
) -> Result<TextBoxOverlay, ExecutionError> {
    let preview = entry.filter(|entry| entry.object == object_id)
        .and_then(|entry| entry.prospective_game_state(game));
    let game = preview.as_ref().unwrap_or(game);
    let effects: Vec<_> = game.effect_store.continuous_effects.effects().to_vec();
    let chars = text_box_characteristics_with_effects(
        object_id,
        game.objects_map(),
        &effects,
        &game.battlefield,
        game.commander_objects(),
        game,
    )
    .ok_or(ExecutionError::InvalidTarget)?;

    Ok(
        TextBoxOverlay::new(chars.compiled_card_text, chars.abilities)
            .with_ability_labels(chars.ability_labels),
    )
}

impl EffectExecutor for ExchangeTextBoxesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut resolved = resolve_objects_for_effect(game, ctx, &self.target)?;
        if self.include_source {
            resolved.insert(0, ctx.source);
        }
        if resolved.len() != 2 {
            return Ok(EffectOutcome::target_invalid());
        }

        let first = resolved[0];
        let second = resolved[1];
        let fallback_entry = ctx.replacement.entry_counter_source.and_then(|source| {
            let from = game.object(source)?.zone;
            Some(crate::events::EnterBattlefieldEvent::new(source, from))
        });
        let entry = ctx.replacement.entry_event.as_deref().or(fallback_entry.as_ref());
        let preview = entry.and_then(|entry| entry.prospective_game_state(game));
        let creature_game = |id| {
            if entry.is_some_and(|entry| entry.object == id) {
                preview.as_ref().unwrap_or(game)
            } else { &*game }
        };
        if first == second
            || !is_current_creature(creature_game(first), first)
            || !is_current_creature(creature_game(second), second)
        {
            return Ok(EffectOutcome::target_invalid());
        }

        let first_overlay = current_text_box_overlay(game, first, entry)?;
        let second_overlay = current_text_box_overlay(game, second, entry)?;

        game.effect_store.continuous_effects.add_effect(
            crate::continuous::ContinuousEffect::from_resolution(
                ctx.source,
                ctx.controller,
                vec![first],
                Modification::SetTextBox(second_overlay),
            )
            .until(self.duration.clone()),
        );
        game.effect_store.continuous_effects.add_effect(
            crate::continuous::ContinuousEffect::from_resolution(
                ctx.source,
                ctx.controller,
                vec![second],
                Modification::SetTextBox(first_overlay),
            )
            .until(self.duration.clone()),
        );

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target.is_target().then_some(&self.target)
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        Some(self.target.count())
    }

    fn target_description(&self) -> &'static str {
        "creatures whose text boxes are exchanged"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::ability::AbilityKind;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effect::ChoiceCount;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effects::ResolvedTarget;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::events::combat::CreatureAttackedEvent;
    use crate::ids::{CardId, ObjectId, PlayerId};
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::provenance::ProvNodeId;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::triggers::{AttackEventTarget, TriggerEvent, check_triggers};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature_from_definition(
        game: &mut GameState,
        definition: &crate::cards::CardDefinition,
        controller: PlayerId,
    ) -> ObjectId {
        let object_id = game.new_object_id();
        let object = crate::object::Object::from_card_definition(
            object_id,
            definition,
            controller,
            Zone::Battlefield,
        );
        game.add_object(object);
        game.effect_store.continuous_effects.record_entry(object_id);
        object_id
    }

    fn vanilla_creature_definition(name: &str, id: u32) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_exchange_source(game: &mut GameState, controller: PlayerId) -> ObjectId {
        let definition = CardDefinitionBuilder::new(CardId::from_raw(700_099), "Exchange Source")
            .card_types(vec![CardType::Enchantment])
            .build();
        let object_id = game.new_object_id();
        let object = crate::object::Object::from_card(
            object_id,
            &definition.card,
            controller,
            Zone::Battlefield,
        );
        game.add_object(object);
        game.effect_store.continuous_effects.record_entry(object_id);
        object_id
    }

    #[test]
    fn ordinary_two_object_exchange_preserves_source_bound_duration() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let flying = CardDefinitionBuilder::new(CardId::new(), "Flying creature")
            .card_types(vec![CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::flying()))
            .build();
        let plain = vanilla_creature_definition("Plain creature", 700_200);
        let first = create_creature_from_definition(&mut game, &flying, alice);
        let second = create_creature_from_definition(&mut game, &plain, alice);
        let source = create_exchange_source(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(first),
            crate::effects::ResolvedTarget::Object(second),
        ]);
        ExchangeTextBoxesEffect::new(
            ChooseSpec::target(ChooseSpec::creature()).with_count(crate::effect::ChoiceCount::exactly(2)),
        ).execute(&mut game, &mut ctx).unwrap();
        assert!(!game.object_has_static_ability_id(first, crate::static_abilities::StaticAbilityId::Flying));
        assert!(game.object_has_static_ability_id(second, crate::static_abilities::StaticAbilityId::Flying));
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert!(game.object_has_static_ability_id(first, crate::static_abilities::StaticAbilityId::Flying));
        assert!(!game.object_has_static_ability_id(second, crate::static_abilities::StaticAbilityId::Flying));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn exchange_text_boxes_swaps_oracle_text_and_current_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mana_def = CardDefinitionBuilder::new(CardId::from_raw(700_001), "Mana Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("{T}: Add {G}.")
            .expect("mana creature should parse");
        let trigger_def = CardDefinitionBuilder::new(CardId::from_raw(700_002), "Raid Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("Whenever this creature attacks, draw a card.")
            .expect("trigger creature should parse");

        let mana_bear = create_creature_from_definition(&mut game, &mana_def, alice);
        let raid_bear = create_creature_from_definition(&mut game, &trigger_def, bob);

        let source = create_exchange_source(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(mana_bear),
            ResolvedTarget::Object(raid_bear),
        ]);
        let outcome = ExchangeTextBoxesEffect::new(
            ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2)),
        )
        .execute(&mut game, &mut ctx)
        .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);

        let mana_chars = game
            .calculated_characteristics(mana_bear)
            .expect("mana creature should have characteristics");
        let raid_chars = game
            .calculated_characteristics(raid_bear)
            .expect("trigger creature should have characteristics");

        assert_eq!(
            mana_chars.compiled_card_text.as_ref(),
            crate::runtime_display::debug_compiled_lines(&trigger_def).join("\n")
        );
        assert_eq!(
            raid_chars.compiled_card_text.as_ref(),
            crate::runtime_display::debug_compiled_lines(&mana_def).join("\n")
        );
        assert!(
            mana_chars
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
            "mana creature should gain the attack trigger text box"
        );
        assert!(
            raid_chars
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
            "trigger creature should gain the mana ability text box"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn exchange_text_boxes_enables_swapped_attack_trigger() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mana_def = CardDefinitionBuilder::new(CardId::from_raw(700_003), "Mana Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("{T}: Add {G}.")
            .expect("mana creature should parse");
        let trigger_def = CardDefinitionBuilder::new(CardId::from_raw(700_004), "Raid Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("Whenever this creature attacks, draw a card.")
            .expect("trigger creature should parse");

        let mana_bear = create_creature_from_definition(&mut game, &mana_def, alice);
        let raid_bear = create_creature_from_definition(&mut game, &trigger_def, alice);

        let source = create_exchange_source(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(mana_bear),
            ResolvedTarget::Object(raid_bear),
        ]);
        ExchangeTextBoxesEffect::new(
            ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2)),
        )
        .execute(&mut game, &mut ctx)
        .expect("exchange should resolve");

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(mana_bear, AttackEventTarget::Player(bob)),
            ProvNodeId::default(),
        );
        let triggers = check_triggers(&game, &event);
        assert!(
            triggers.iter().any(|entry| entry.source == mana_bear),
            "mana creature should gain the swapped attack trigger"
        );
        assert!(
            !triggers.iter().any(|entry| entry.source == raid_bear),
            "the original trigger source should lose that trigger after the exchange"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn exchange_text_boxes_swapped_static_abilities_generate_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let lord_def = CardDefinitionBuilder::new(CardId::from_raw(700_005), "Lord Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("Other creatures you control get +1/+1.")
            .expect("lord creature should parse");
        let vanilla_def = vanilla_creature_definition("Vanilla Bear", 700_006);
        let buddy_def = vanilla_creature_definition("Buddy Bear", 700_007);

        let lord = create_creature_from_definition(&mut game, &lord_def, alice);
        let vanilla = create_creature_from_definition(&mut game, &vanilla_def, alice);
        let buddy = create_creature_from_definition(&mut game, &buddy_def, alice);

        let source = create_exchange_source(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(lord),
            ResolvedTarget::Object(vanilla),
        ]);
        ExchangeTextBoxesEffect::new(
            ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2)),
        )
        .execute(&mut game, &mut ctx)
        .expect("exchange should resolve");
        game.refresh_continuous_state();

        assert_eq!(game.calculated_power(vanilla), Some(2));
        assert_eq!(game.calculated_power(buddy), Some(3));
        assert_eq!(game.calculated_power(lord), Some(3));
    }
}
