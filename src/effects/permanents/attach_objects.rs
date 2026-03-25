//! Attach arbitrary objects to a target object or player.

use super::attach_battlefield_object_to_target;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_single_target_from_spec};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::AttachmentTarget;
use crate::target::ChooseSpec;

/// Effect that attaches one or more objects to a destination object.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachObjectsEffect {
    /// Objects to attach.
    pub objects: ChooseSpec,
    /// Destination to attach objects to.
    pub target: ChooseSpec,
}

impl AttachObjectsEffect {
    /// Create a new attach-objects effect.
    pub fn new(objects: ChooseSpec, target: ChooseSpec) -> Self {
        Self { objects, target }
    }
}

impl EffectExecutor for AttachObjectsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target = match resolve_single_target_from_spec(game, &self.target, ctx) {
            Ok(crate::executor::ResolvedTarget::Object(id)) => AttachmentTarget::Object(id),
            Ok(crate::executor::ResolvedTarget::Player(id)) => AttachmentTarget::Player(id),
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };
        if !game.attachment_target_exists_on_battlefield(target) {
            return Ok(EffectOutcome::target_invalid());
        }

        let object_ids = resolve_objects_for_effect(game, ctx, &self.objects)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut attached_count = 0i32;
        for object_id in object_ids {
            if attach_battlefield_object_to_target(game, object_id, target) {
                attached_count += 1;
            }
        }

        Ok(EffectOutcome::count(attached_count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to attach to"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_land(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_equipment(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_marker_artifact(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_aura(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let mut object = Object::from_card(id, &card, controller, Zone::Battlefield);
        object.aura_attach_filter = Some(ObjectFilter::creature().into());
        game.add_object(object);
        id
    }

    #[test]
    fn test_attach_objects_illegal_equipment_target_does_not_move() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let equipment = create_equipment(&mut game, "Test Equipment", alice);
        let creature = create_creature(&mut game, "Bear", alice);
        let land = create_land(&mut game, "Forest", alice);

        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            equipment,
            AttachmentTarget::Object(creature),
        );
        let original_timestamp = game
            .continuous_effects
            .get_attachment_timestamp(equipment)
            .expect("equipment should gain a timestamp when first attached");

        let mut ctx = ExecutionContext::new_default(equipment, alice)
            .with_targets(vec![ResolvedTarget::Object(land)]);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(equipment),
            ChooseSpec::target_permanent(),
        );

        let result = effect.execute(&mut game, &mut ctx).expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 0);
        assert_eq!(
            game.object(equipment).and_then(|object| object.attached_to),
            Some(crate::object::AttachmentTarget::Object(creature)),
            "illegal reattach should leave the equipment on its original creature"
        );
        assert_eq!(
            game.continuous_effects.get_attachment_timestamp(equipment),
            Some(original_timestamp),
            "illegal attach attempts should not create a new timestamp"
        );
    }

    #[test]
    fn test_attach_objects_non_attachment_object_does_not_move() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let marker = create_marker_artifact(&mut game, "Marker", alice);
        let creature = create_creature(&mut game, "Bear", alice);

        let mut ctx = ExecutionContext::new_default(marker, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(marker),
            ChooseSpec::target_creature(),
        );

        let result = effect.execute(&mut game, &mut ctx).expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 0);
        assert_eq!(
            game.object(marker).and_then(|object| object.attached_to),
            None,
            "objects that are not Auras, Equipment, or Fortifications should remain unattached"
        );
        assert!(
            !game.object(creature)
                .expect("creature should exist")
                .attachments
                .contains(&marker),
            "the target should not gain a fake attachment link"
        );
    }

    #[test]
    fn test_attach_objects_to_same_target_is_no_op() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let aura = create_aura(&mut game, "Pacifism", alice);
        let creature = create_creature(&mut game, "Bear", alice);

        assert!(crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            aura,
            AttachmentTarget::Object(creature),
        ));
        let original_timestamp = game
            .continuous_effects
            .get_attachment_timestamp(aura)
            .expect("aura should gain a timestamp when first attached");

        let mut ctx = ExecutionContext::new_default(aura, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(aura),
            ChooseSpec::target_creature(),
        );

        let result = effect.execute(&mut game, &mut ctx).expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 0);
        assert_eq!(
            game.continuous_effects.get_attachment_timestamp(aura),
            Some(original_timestamp),
            "reattaching to the same object should not create a new timestamp"
        );
        assert_eq!(
            game.object(aura).and_then(|object| object.attached_to),
            Some(crate::object::AttachmentTarget::Object(creature))
        );
    }

    #[test]
    fn test_attach_objects_aura_to_player_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let aura = create_aura(&mut game, "Curse", alice);
        game.object_mut(aura)
            .expect("aura should exist")
            .aura_attach_filter = Some(crate::target::PlayerFilter::Any.into());

        let mut ctx =
            ExecutionContext::new_default(aura, alice).with_targets(vec![ResolvedTarget::Player(
                bob,
            )]);
        let effect =
            AttachObjectsEffect::new(ChooseSpec::SpecificObject(aura), ChooseSpec::target_player());

        let result = effect.execute(&mut game, &mut ctx).expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 1);
        assert_eq!(
            game.object(aura).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Player(bob))
        );
        assert!(
            game.player(bob)
                .expect("bob should exist")
                .attachments
                .contains(&aura),
            "the enchanted player should record the Aura attachment"
        );
    }
}
