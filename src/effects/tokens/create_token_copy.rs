//! Create token copy effect implementation.

use crate::ability::Ability;
use crate::card::PtValue;
use crate::effect::{Effect, EffectOutcome, EffectResult, Value};
use crate::effects::helpers::{resolve_objects_from_spec, resolve_value};
use crate::effects::{
    EffectExecutor, EnterAttackingEffect, GrantObjectAbilityEffect, ScheduleDelayedTriggerEffect,
};
use crate::events::EnterBattlefieldEvent;
use crate::events::zones::ZoneChangeEvent;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::object::Object;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::{Trigger, TriggerEvent};
use crate::zone::Zone;

/// Effect that creates a token copy of a permanent.
///
/// # Fields
///
/// * `target` - Which permanent to copy
/// * `count` - How many copies to create
/// * `controller` - Who controls the tokens
/// * `enters_tapped` - Whether the copy enters tapped
/// * `has_haste` - Whether the copy has haste
/// * `enters_attacking` - Whether the copy enters attacking
/// * `exile_at_end_of_combat` - Whether to exile at end of combat
///
/// # Example
///
/// ```ignore
/// // Create a token copy of target creature
/// let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
///
/// // Create a copy with haste that's exiled at end of combat (Kiki-Jiki style)
/// let effect = CreateTokenCopyEffect::kiki_jiki_style(ChooseSpec::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTokenCopyEffect {
    /// Which permanent to copy.
    pub target: ChooseSpec,
    /// How many copies to create.
    pub count: Value,
    /// Who controls the tokens.
    pub controller: PlayerFilter,
    /// Whether the copy enters tapped.
    pub enters_tapped: bool,
    /// Whether the copy has haste.
    pub has_haste: bool,
    /// Whether the copy enters attacking.
    pub enters_attacking: bool,
    /// Whether to exile at end of combat.
    pub exile_at_end_of_combat: bool,
    /// Optional power/toughness adjustment for the created tokens.
    pub pt_adjustment: Option<CopyPtAdjustment>,
}

/// Optional power/toughness adjustment for copied tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyPtAdjustment {
    /// Set base power/toughness to half (rounded up) of the original.
    HalfRoundUp,
}

impl CreateTokenCopyEffect {
    /// Create a new create token copy effect.
    pub fn new(target: ChooseSpec, count: impl Into<Value>, controller: PlayerFilter) -> Self {
        Self {
            target,
            count: count.into(),
            controller,
            enters_tapped: false,
            has_haste: false,
            enters_attacking: false,
            exile_at_end_of_combat: false,
            pt_adjustment: None,
        }
    }

    /// Create a single token copy under your control.
    pub fn one(target: ChooseSpec) -> Self {
        Self::new(target, 1, PlayerFilter::You)
    }

    /// Create a token copy with haste.
    pub fn with_haste(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.has_haste = true;
        effect
    }

    /// Create a token copy that enters tapped.
    pub fn tapped(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.enters_tapped = true;
        effect
    }

    /// Create a Kiki-Jiki style copy: has haste and is exiled at end of combat.
    pub fn kiki_jiki_style(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.has_haste = true;
        effect.exile_at_end_of_combat = true;
        effect
    }

    /// Set whether the copy enters tapped.
    pub fn enters_tapped(mut self, value: bool) -> Self {
        self.enters_tapped = value;
        self
    }

    /// Set whether the copy has haste.
    pub fn haste(mut self, value: bool) -> Self {
        self.has_haste = value;
        self
    }

    /// Set whether the copy enters attacking.
    pub fn attacking(mut self, value: bool) -> Self {
        self.enters_attacking = value;
        self
    }

    /// Set whether to exile at end of combat.
    pub fn exile_at_eoc(mut self, value: bool) -> Self {
        self.exile_at_end_of_combat = value;
        self
    }

    /// Set base power/toughness to half (rounded up) of the original.
    pub fn half_power_toughness_round_up(mut self) -> Self {
        self.pt_adjustment = Some(CopyPtAdjustment::HalfRoundUp);
        self
    }
}

impl EffectExecutor for CreateTokenCopyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id =
            crate::effects::helpers::resolve_player_filter(game, &self.controller, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Resolve target from spec (supports tagged/spec-specific references)
        let target_ids = resolve_objects_from_spec(game, &self.target, ctx)?;
        let target_id = *target_ids.first().ok_or(ExecutionError::InvalidTarget)?;

        // Resolve target object (supports tagged LKI with stable_id lookup)
        let mut tagged_snapshot = None;
        let mut resolved_target_id = target_id;
        let mut target_object = game.object(resolved_target_id);

        if target_object.is_none() {
            if let ChooseSpec::Tagged(tag) = &self.target {
                if let Some(snapshot) = ctx.get_tagged(tag.as_str()) {
                    tagged_snapshot = Some(snapshot.clone());
                    if let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id) {
                        resolved_target_id = current_id;
                        target_object = game.object(resolved_target_id);
                    }
                }
            }
        }

        let Some(target_object) = target_object else {
            return Err(ExecutionError::ObjectNotFound(target_id));
        };

        let mut created_ids = Vec::with_capacity(count);
        let mut events = Vec::with_capacity(count);
        let original_targets = ctx.targets.clone();

        let target_for_stats = &target_object;
        let (half_power, half_toughness) = match self.pt_adjustment {
            Some(CopyPtAdjustment::HalfRoundUp) => {
                let (power, toughness) = if let Some(snapshot) = &tagged_snapshot {
                    (snapshot.power.unwrap_or(0), snapshot.toughness.unwrap_or(0))
                } else {
                    (
                        target_for_stats.power().unwrap_or(0),
                        target_for_stats.toughness().unwrap_or(0),
                    )
                };
                ((power + 1) / 2, (toughness + 1) / 2)
            }
            None => (0, 0),
        };

        for _ in 0..count {
            let id = game.new_object_id();
            // Get fresh reference to target each iteration
            let target = game
                .object(resolved_target_id)
                .ok_or(ExecutionError::ObjectNotFound(resolved_target_id))?;
            let mut token = Object::token_copy_of(target, id, controller_id);
            token.zone = Zone::Battlefield;

            if let Some(CopyPtAdjustment::HalfRoundUp) = self.pt_adjustment {
                token.base_power = Some(PtValue::Fixed(half_power));
                token.base_toughness = Some(PtValue::Fixed(half_toughness));
            }

            // Track creature ETBs for trap conditions
            if token.is_creature() {
                *game
                    .creatures_entered_this_turn
                    .entry(controller_id)
                    .or_insert(0) += 1;
            }

            game.add_object(token);

            // Apply modifications (must be after add_object for extension maps)
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

            // Apply haste via the shared GrantObjectAbilityEffect primitive.
            if self.has_haste {
                ctx.targets = vec![ResolvedTarget::Object(id)];
                let grant_effect = GrantObjectAbilityEffect::new(
                    Ability::static_ability(StaticAbility::haste()),
                    ChooseSpec::AnyTarget,
                );
                let _ = execute_effect(game, &Effect::new(grant_effect), ctx)?;
            }
        }

        ctx.targets = original_targets;

        Ok(EffectOutcome::from_result(EffectResult::Objects(created_ids)).with_events(events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to copy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::ObjectKind;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_create_token_copy() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.name, "Grizzly Bears");
            assert_eq!(token.kind, ObjectKind::Token);
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_with_haste() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Baneslayer Angel", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::with_haste(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token = game.object(ids[0]).unwrap();
            // Token should have haste ability
            let has_haste = token.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            assert!(has_haste, "Token should have haste");
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Serra Angel", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::tapped(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert!(game.is_tapped(ids[0]), "Token should enter tapped");
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_multiple_token_copies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Llanowar Elves", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::new(ChooseSpec::creature(), 3, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 3);
            for id in ids {
                let token = game.object(id).unwrap();
                assert_eq!(token.name, "Llanowar Elves");
                assert_eq!(token.kind, ObjectKind::Token);
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err(), "Should fail without target");
    }

    #[test]
    fn test_create_token_copy_clone_box() {
        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateTokenCopyEffect"));
    }

    #[test]
    fn test_create_token_copy_kiki_jiki_style() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Pestermite", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::kiki_jiki_style(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token_id = ids[0];
            let token = game.object(token_id).unwrap();

            // Token should have haste
            let has_haste = token.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            assert!(has_haste, "Token should have haste");

            // Should have delayed trigger to exile at end of combat
            assert_eq!(game.delayed_triggers.len(), 1);
            let delayed = &game.delayed_triggers[0];
            assert!(delayed.trigger.display().contains("end of combat"));
            assert!(delayed.one_shot);
            assert_eq!(delayed.target_objects, vec![token_id]);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_enters_attacking() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature(&mut game, "Goblin Guide", alice);
        let source = create_creature(&mut game, "Source Attacker", alice);

        // Set up combat with source attacking Bob
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature()).attacking(true);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token_id = ids[0];
            // Token should be added to combat attackers
            let combat = game.combat.as_ref().expect("Combat should still be active");
            assert!(
                combat
                    .attackers
                    .iter()
                    .any(|info| info.creature == token_id),
                "Token should be in combat attackers"
            );
            // Token should be attacking the same target as source
            let token_attacker = combat
                .attackers
                .iter()
                .find(|info| info.creature == token_id)
                .expect("Token should be attacking");
            assert_eq!(
                token_attacker.target,
                AttackTarget::Player(bob),
                "Token should attack the same target as source"
            );
        } else {
            panic!("Expected Objects result");
        }
    }
}
