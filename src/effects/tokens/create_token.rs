//! Create token effect implementation.

use crate::cards::CardDefinition;
use crate::effect::{Effect, EffectOutcome, EffectResult, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{
    EffectExecutor, EnterAttackingEffect, SacrificeTargetEffect, ScheduleDelayedTriggerEffect,
};
use crate::events::EnterBattlefieldEvent;
use crate::events::zones::ZoneChangeEvent;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::object::Object;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::{Trigger, TriggerEvent};
use crate::zone::Zone;

/// Effect that creates token creatures or other token permanents.
///
/// # Fields
///
/// * `token` - The token definition (use CardDefinitionBuilder with .token())
/// * `count` - How many tokens to create
/// * `controller` - Who controls the tokens
/// * `enters_tapped` - Whether the tokens enter tapped
/// * `enters_attacking` - Whether the tokens enter attacking
/// * `exile_at_end_of_combat` - Whether to exile the tokens at end of combat
/// * `sacrifice_at_next_end_step` - Whether to sacrifice the tokens at the
///   beginning of the next end step.
/// * `exile_at_next_end_step` - Whether to exile the tokens at the beginning
///   of the next end step.
///
/// # Example
///
/// ```ignore
/// // Create two 1/1 white Soldier tokens
/// let soldier = CardDefinitionBuilder::new(CardId::new(), "Soldier")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Soldier])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(1, 1))
///     .build();
/// let effect = CreateTokenEffect::new(soldier, 2, PlayerFilter::You);
///
/// // Create a 4/4 Angel token that enters tapped and attacking, exiled at EOC
/// let angel = CardDefinitionBuilder::new(CardId::new(), "Angel")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Angel])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(4, 4))
///     .flying()
///     .build();
/// let effect = CreateTokenEffect::one(angel)
///     .tapped()
///     .attacking()
///     .exile_at_end_of_combat();
/// ```
#[derive(Debug, Clone)]
pub struct CreateTokenEffect {
    /// The token definition.
    pub token: CardDefinition,
    /// How many tokens to create.
    pub count: Value,
    /// Who controls the tokens.
    pub controller: PlayerFilter,
    /// Whether the tokens enter tapped.
    pub enters_tapped: bool,
    /// Whether the tokens enter attacking.
    pub enters_attacking: bool,
    /// Whether to exile the tokens at end of combat.
    pub exile_at_end_of_combat: bool,
    /// Whether to sacrifice the tokens at the beginning of the next end step.
    pub sacrifice_at_next_end_step: bool,
    /// Whether to exile the tokens at the beginning of the next end step.
    pub exile_at_next_end_step: bool,
}

impl PartialEq for CreateTokenEffect {
    fn eq(&self, other: &Self) -> bool {
        // Compare by token name and count - CardDefinition doesn't impl PartialEq
        self.token.card.name == other.token.card.name
            && self.count == other.count
            && self.controller == other.controller
            && self.enters_tapped == other.enters_tapped
            && self.enters_attacking == other.enters_attacking
            && self.exile_at_end_of_combat == other.exile_at_end_of_combat
            && self.sacrifice_at_next_end_step == other.sacrifice_at_next_end_step
            && self.exile_at_next_end_step == other.exile_at_next_end_step
    }
}

impl CreateTokenEffect {
    /// Create a new create token effect.
    pub fn new(token: CardDefinition, count: impl Into<Value>, controller: PlayerFilter) -> Self {
        Self {
            token,
            count: count.into(),
            controller,
            enters_tapped: false,
            enters_attacking: false,
            exile_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
        }
    }

    /// Create tokens under your control.
    pub fn you(token: CardDefinition, count: impl Into<Value>) -> Self {
        Self::new(token, count, PlayerFilter::You)
    }

    /// Create a single token under your control.
    pub fn one(token: CardDefinition) -> Self {
        Self::you(token, 1)
    }

    /// Set whether the tokens enter tapped.
    pub fn tapped(mut self) -> Self {
        self.enters_tapped = true;
        self
    }

    /// Set whether the tokens enter attacking.
    pub fn attacking(mut self) -> Self {
        self.enters_attacking = true;
        self
    }

    /// Set whether to exile the tokens at end of combat.
    pub fn exile_at_end_of_combat(mut self) -> Self {
        self.exile_at_end_of_combat = true;
        self
    }

    /// Set whether to sacrifice the tokens at the beginning of the next end step.
    pub fn sacrifice_at_next_end_step(mut self) -> Self {
        self.sacrifice_at_next_end_step = true;
        self
    }

    /// Set whether to exile the tokens at the beginning of the next end step.
    pub fn exile_at_next_end_step(mut self) -> Self {
        self.exile_at_next_end_step = true;
        self
    }
}

impl EffectExecutor for CreateTokenEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id =
            crate::effects::helpers::resolve_player_filter(game, &self.controller, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        let mut created_ids = Vec::with_capacity(count);
        let mut events = Vec::with_capacity(count);

        let original_targets = ctx.targets.clone();

        for _ in 0..count {
            let id = game.new_object_id();
            let token_obj = Object::from_token_definition(id, &self.token, controller_id);

            // Track creature ETBs for trap conditions
            if token_obj.is_creature() {
                *game
                    .creatures_entered_this_turn
                    .entry(controller_id)
                    .or_insert(0) += 1;
            }

            game.add_object(token_obj);

            // Apply entry modifications (must be after add_object so object exists for extension maps)
            if self.enters_tapped {
                game.tap(id);
            }
            // Tokens always have summoning sickness
            game.set_summoning_sick(id);

            created_ids.push(id);

            // Emit primitive zone-change ETB event plus ETB-tapped event.
            events.push(TriggerEvent::new(ZoneChangeEvent::new(
                id,
                Zone::Stack,
                Zone::Battlefield,
                None,
            )));
            let etb_event = if self.enters_tapped {
                TriggerEvent::new(EnterBattlefieldEvent::tapped(id, Zone::Stack))
            } else {
                TriggerEvent::new(EnterBattlefieldEvent::new(id, Zone::Stack))
            };
            events.push(etb_event);

            // Handle enters_attacking - add directly to combat if in combat
            if self.enters_attacking {
                ctx.targets = vec![ResolvedTarget::Object(id)];
                let enter_attacking = EnterAttackingEffect::new(ChooseSpec::AnyTarget);
                let _ = execute_effect(game, &Effect::new(enter_attacking), ctx)?;
            }

            // Handle exile at end of combat
            if self.exile_at_end_of_combat {
                let schedule = ScheduleDelayedTriggerEffect::new(
                    Trigger::end_of_combat(),
                    vec![Effect::exile(ChooseSpec::SpecificObject(id))],
                    true,
                    vec![id],
                    PlayerFilter::Specific(controller_id),
                );
                let _ = execute_effect(game, &Effect::new(schedule), ctx)?;
            }

            if self.sacrifice_at_next_end_step {
                let schedule = ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(PlayerFilter::Any),
                    vec![Effect::new(SacrificeTargetEffect::new(ChooseSpec::SpecificObject(
                        id,
                    )))],
                    true,
                    vec![id],
                    PlayerFilter::Specific(controller_id),
                );
                let _ = execute_effect(game, &Effect::new(schedule), ctx)?;
            }
            if self.exile_at_next_end_step {
                let schedule = ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(PlayerFilter::Any),
                    vec![Effect::exile(ChooseSpec::SpecificObject(id))],
                    true,
                    vec![id],
                    PlayerFilter::Specific(controller_id),
                );
                let _ = execute_effect(game, &Effect::new(schedule), ctx)?;
            }
        }

        ctx.targets = original_targets;

        Ok(EffectOutcome::from_result(EffectResult::Objects(created_ids)).with_events(events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::color::{Color, ColorSet};
    use crate::ids::{CardId, PlayerId};
    use crate::object::ObjectKind;
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn soldier_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Soldier")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Soldier])
            .color_indicator(ColorSet::from(Color::White))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn goblin_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Goblin")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Goblin])
            .color_indicator(ColorSet::from(Color::Red))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn zombie_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn beast_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Beast")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Beast])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    fn spirit_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Spirit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    #[test]
    fn test_create_single_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::one(soldier_token());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.name, "Soldier");
            assert_eq!(token.kind, ObjectKind::Token);
            assert!(token.is_creature());
            assert_eq!(token.power(), Some(1));
            assert_eq!(token.toughness(), Some(1));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_multiple_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(goblin_token(), 3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 3);
            for id in ids {
                let token = game.object(id).unwrap();
                assert_eq!(token.name, "Goblin");
                assert_eq!(token.kind, ObjectKind::Token);
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_zero_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(zombie_token(), 0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert!(ids.is_empty());
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_tracks_creature_etb() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(beast_token(), 2);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have tracked 2 creatures entering
        assert_eq!(game.creatures_entered_this_turn.get(&alice), Some(&2));
    }

    #[test]
    fn test_create_token_for_other_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        // Use Specific instead of Opponent since Opponent requires targeting context
        let effect = CreateTokenEffect::new(spirit_token(), 1, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.controller, bob);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_clone_box() {
        let effect = CreateTokenEffect::one(soldier_token());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateTokenEffect"));
    }
}
