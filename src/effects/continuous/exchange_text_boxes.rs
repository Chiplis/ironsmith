//! Exchange text boxes effect implementation.

use crate::continuous::{Modification, TextBoxOverlay, text_box_characteristics_with_effects};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::types::CardType;

/// Effect that exchanges the text boxes of exactly two creatures.
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeTextBoxesEffect {
    pub target: ChooseSpec,
}

impl ExchangeTextBoxesEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

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
    ) -> Result<TextBoxOverlay, ExecutionError> {
        let effects: Vec<_> = game.continuous_effects.effects().to_vec();
        let chars = text_box_characteristics_with_effects(
            object_id,
            game.objects_map(),
            &effects,
            &game.battlefield,
            &game.commanders,
            game,
        )
        .ok_or(ExecutionError::InvalidTarget)?;

        Ok(TextBoxOverlay::new(chars.oracle_text, chars.abilities))
    }
}

impl EffectExecutor for ExchangeTextBoxesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let resolved = resolve_objects_for_effect(game, ctx, &self.target)?;
        if resolved.len() != 2 {
            return Ok(EffectOutcome::target_invalid());
        }

        let first = resolved[0];
        let second = resolved[1];
        if first == second
            || !Self::is_current_creature(game, first)
            || !Self::is_current_creature(game, second)
        {
            return Ok(EffectOutcome::target_invalid());
        }

        let first_overlay = Self::current_text_box_overlay(game, first)?;
        let second_overlay = Self::current_text_box_overlay(game, second)?;

        game.continuous_effects.add_effect(
            crate::continuous::ContinuousEffect::from_resolution(
                ctx.source,
                ctx.controller,
                vec![first],
                Modification::SetTextBox(second_overlay),
            )
            .until(crate::effect::Until::ThisLeavesTheBattlefield),
        );
        game.continuous_effects.add_effect(
            crate::continuous::ContinuousEffect::from_resolution(
                ctx.source,
                ctx.controller,
                vec![second],
                Modification::SetTextBox(first_overlay),
            )
            .until(crate::effect::Until::ThisLeavesTheBattlefield),
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
    use crate::ability::AbilityKind;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::effect::ChoiceCount;
    use crate::events::combat::CreatureAttackedEvent;
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::provenance::ProvNodeId;
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
        let mut object = crate::object::Object::from_card(
            object_id,
            &definition.card,
            controller,
            Zone::Battlefield,
        );
        object.abilities = definition.abilities.clone();
        game.add_object(object);
        game.continuous_effects.record_entry(object_id);
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
        game.continuous_effects.record_entry(object_id);
        object_id
    }

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

        assert_eq!(mana_chars.oracle_text, trigger_def.card.oracle_text);
        assert_eq!(raid_chars.oracle_text, mana_def.card.oracle_text);
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
