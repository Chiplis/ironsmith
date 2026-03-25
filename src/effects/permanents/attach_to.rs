//! Attach to effect implementation.

use super::attach_battlefield_object_to_target;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_target_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::AttachmentTarget;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that attaches the source permanent to a target permanent.
///
/// Used primarily by Auras that grant control or Equipment that auto-attach.
/// The source becomes attached to the target, and the target gains
/// the source in its attachments list.
///
/// # Fields
///
/// * `target` - The target specification for what to attach to
///
/// # Example
///
/// ```ignore
/// // Create an attach effect for an aura
/// let effect = AttachToEffect::new(ChooseSpec::target_creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AttachToEffect {
    /// The target to attach to.
    pub target: ChooseSpec,
}

impl AttachToEffect {
    /// Create a new attach to effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for AttachToEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target = resolve_single_target_from_spec(game, &self.target, ctx)?;

        // If this is a spell on the stack (Aura resolving), defer attachment
        if let Some(source) = game.object(ctx.source)
            && source.zone == Zone::Stack
        {
            return Ok(EffectOutcome::resolved());
        }

        match target {
            crate::executor::ResolvedTarget::Object(id) => {
                attach_battlefield_object_to_target(game, ctx.source, AttachmentTarget::Object(id));
            }
            crate::executor::ResolvedTarget::Player(id) => {
                attach_battlefield_object_to_target(game, ctx.source, AttachmentTarget::Player(id));
            }
        }

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to attach to"
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
    use crate::snapshot::ObjectSnapshot;
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn make_aura_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_aura(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_aura_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.aura_attach_filter = Some(crate::target::ObjectFilter::creature().into());
        game.add_object(obj);
        id
    }

    #[test]
    fn test_attach_to_target_from_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_aura(&mut game, "Pacifism", alice);
        let target = create_creature(&mut game, "Bear", alice);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let effect = AttachToEffect::new(ChooseSpec::target_creature());
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.object(source).unwrap().attached_to,
            Some(crate::object::AttachmentTarget::Object(target))
        );
        assert!(game.object(target).unwrap().attachments.contains(&source));
    }

    #[test]
    fn test_attach_to_tagged_target_without_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_aura(&mut game, "Pacifism", alice);
        let target = create_creature(&mut game, "Bear", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let snapshot = ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        ctx.tag_object("attach_target", snapshot);

        let effect = AttachToEffect::new(ChooseSpec::Tagged("attach_target".into()));
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.object(source).unwrap().attached_to,
            Some(crate::object::AttachmentTarget::Object(target))
        );
        assert!(game.object(target).unwrap().attachments.contains(&source));
    }

    #[test]
    fn test_attach_to_player_target_from_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_aura(&mut game, "Curse", alice);
        game.object_mut(source)
            .expect("source Aura should exist")
            .aura_attach_filter = Some(crate::target::PlayerFilter::Any.into());
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Player(
                bob,
            )]);

        let effect = AttachToEffect::new(ChooseSpec::target_player());
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.object(source).unwrap().attached_to,
            Some(crate::object::AttachmentTarget::Player(bob))
        );
        assert!(
            game.player(bob)
                .expect("bob should exist")
                .attachments
                .contains(&source)
        );
    }
}
