//! Transform effect implementation.

use crate::card::LinkedFaceLayout;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_tagged_object_id};
use crate::events::other::{ConvertedEvent, TransformedEvent};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::types::CardType;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransformLikeAction {
    Transform,
    Convert,
}

impl TransformLikeAction {
    fn event(
        self,
        target_id: crate::ids::ObjectId,
        provenance: crate::provenance::ProvNodeId,
    ) -> TriggerEvent {
        match self {
            Self::Transform => {
                TriggerEvent::new_with_provenance(TransformedEvent::new(target_id), provenance)
            }
            Self::Convert => {
                TriggerEvent::new_with_provenance(ConvertedEvent::new(target_id), provenance)
            }
        }
    }

    fn target_description(self) -> &'static str {
        match self {
            Self::Transform => "permanent to transform",
            Self::Convert => "permanent to convert",
        }
    }
}

/// Effect that transforms a double-faced permanent.
///
/// Toggles the face state of a DFC (double-faced card).
/// When face_down is false, the card shows its front face.
/// When face_down is true, the card shows its back face.
///
/// # Fields
///
/// * `target` - The permanent to transform
///
/// # Example
///
/// ```ignore
/// // Transform target permanent
/// let effect = TransformEffect::new(ChooseSpec::permanent());
///
/// // Transform this permanent (the source)
/// let effect = TransformEffect::source();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TransformEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
}

impl TransformEffect {
    /// Create a new transform effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that transforms the source permanent.
    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    /// Create an effect that transforms target permanent.
    pub fn target_permanent() -> Self {
        Self::new(ChooseSpec::permanent())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvertEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
}

impl ConvertEffect {
    /// Create a new convert effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that converts the source permanent.
    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    /// Create an effect that converts target permanent.
    pub fn target_permanent() -> Self {
        Self::new(ChooseSpec::permanent())
    }
}

fn source_transform_like_action_is_stale(
    game: &GameState,
    ctx: &ExecutionContext,
    target_id: crate::ids::ObjectId,
) -> bool {
    target_id == ctx.source
        && ctx
            .source_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.transform_count != game.transform_count(target_id))
}

fn execute_transform_like_action(
    action: TransformLikeAction,
    target: &ChooseSpec,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let target_id = if let ChooseSpec::Tagged(tag) = target {
        ctx.get_tagged_all(tag)
            .and_then(|snapshots| {
                snapshots
                    .iter()
                    .find_map(|snapshot| resolve_tagged_object_id(game, snapshot))
            })
            .ok_or(ExecutionError::InvalidTarget)?
    } else {
        resolve_single_object_for_effect(game, ctx, target)?
    };

    if !game.can_transform(target_id) {
        return Ok(EffectOutcome::resolved());
    }

    if source_transform_like_action_is_stale(game, ctx, target_id) {
        return Ok(EffectOutcome::resolved());
    }

    let Some(target) = game.object(target_id) else {
        return Ok(EffectOutcome::resolved());
    };
    if target.zone != Zone::Battlefield
        || target.linked_face_layout != LinkedFaceLayout::TransformLike
    {
        return Ok(EffectOutcome::resolved());
    }

    let Some(other_def) = game
        .linked_face_definition_by_name_or_id(target.other_face_name.as_deref(), target.other_face)
    else {
        return Ok(EffectOutcome::resolved());
    };
    if other_def.card.card_types.contains(&CardType::Instant)
        || other_def.card.card_types.contains(&CardType::Sorcery)
    {
        return Ok(EffectOutcome::resolved());
    }

    let was_face_down = game.is_face_down(target_id);
    if let Some(obj) = game.object_mut(target_id) {
        obj.apply_definition_face(&other_def);
    }

    // The engine uses `face_down` to represent a transform-like permanent's back face.
    if was_face_down {
        game.set_face_up(target_id);
    } else {
        game.set_face_down(target_id);
    }
    game.mark_transformed(target_id);

    Ok(EffectOutcome::resolved().with_event(action.event(target_id, ctx.provenance)))
}

impl EffectExecutor for TransformEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        execute_transform_like_action(TransformLikeAction::Transform, &self.target, game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        TransformLikeAction::Transform.target_description()
    }
}

impl EffectExecutor for ConvertEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        execute_transform_like_action(TransformLikeAction::Convert, &self.target, game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        TransformLikeAction::Convert.target_description()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{LinkedFaceLayout, PowerToughness};
    use crate::cards::{CardDefinition, CardDefinitionBuilder};
    use crate::events::EventKind;
    use crate::events::combat::CreatureAttackedEvent;
    use crate::events::phase::EndOfCombatEvent;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::{
        AttackEventTarget, TransformsTrigger, TriggerContext, TriggerEvent, TriggerMatcher,
        TriggerQueue, check_triggers,
    };
    use crate::types::{CardType, Subtype};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn runtime_custom_registry_test_guard() -> MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("transform registry tests should acquire the runtime custom-card test mutex")
    }

    fn register_transform_pair(
        front_id: CardId,
        front_name: &str,
        back_id: CardId,
        back_name: &str,
        back_types: Vec<CardType>,
        back_text: &str,
    ) -> CardDefinition {
        let mut front = CardDefinitionBuilder::new(front_id, front_name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human, Subtype::Scout])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("Vigilance")
            .expect("front face should parse");
        front.card.other_face = Some(back_id);
        front.card.other_face_name = Some(back_name.to_string());
        front.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        let mut back_builder = CardDefinitionBuilder::new(back_id, back_name)
            .card_types(back_types)
            .oracle_text(back_text);
        if back_text == "Trample" {
            back_builder = back_builder
                .subtypes(vec![Subtype::Werewolf])
                .power_toughness(PowerToughness::fixed(4, 4));
        }
        let mut back = back_builder
            .parse_text(back_text)
            .expect("back face should parse");
        back.card.other_face = Some(front_id);
        back.card.other_face_name = Some(front_name.to_string());
        back.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        crate::cards::register_runtime_custom_card(front.clone());
        crate::cards::register_runtime_custom_card(back);
        front
    }

    fn register_conquerors_galleon_pair(front_id: CardId, back_id: CardId) -> CardDefinition {
        let mut front = CardDefinitionBuilder::new(front_id, "Conqueror's Galleon")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle])
            .power_toughness(PowerToughness::fixed(2, 10))
            .parse_text(
                "When this Vehicle attacks, exile it at end of combat, then return it to the battlefield transformed under your control.\nCrew 4 (Tap any number of creatures you control with total power 4 or more: This Vehicle becomes an artifact creature until end of turn.)",
            )
            .expect("front face should parse");
        front.card.other_face = Some(back_id);
        front.card.other_face_name = Some("Conqueror's Foothold".to_string());
        front.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        let mut back = CardDefinitionBuilder::new(back_id, "Conqueror's Foothold")
            .card_types(vec![CardType::Land])
            .build();
        back.card.other_face = Some(front_id);
        back.card.other_face_name = Some("Conqueror's Galleon".to_string());
        back.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        crate::cards::register_runtime_custom_card(front.clone());
        crate::cards::register_runtime_custom_card(back);
        front
    }

    #[test]
    fn transform_swaps_faces_and_refreshes_timestamp() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_100),
            "Trail Scout",
            CardId::from_raw(79_101),
            "Moonlit Howler",
            vec![CardType::Creature],
            "Trample",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let before_ts = game
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("battlefield permanent should have an entry timestamp");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("transform should execute");

        assert_eq!(outcome.events.len(), 1);
        assert!(game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        let after_ts = game
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("transformed permanent should keep an entry timestamp");
        assert!(
            after_ts > before_ts,
            "transformed permanents should get a fresh timestamp"
        );

        let object = game.object(source).expect("source permanent should exist");
        assert_eq!(object.name, "Moonlit Howler");
        assert_eq!(object.card_types, vec![CardType::Creature]);
        assert_eq!(object.subtypes, vec![Subtype::Werewolf]);
        assert_eq!(object.base_power.map(|value| value.base_value()), Some(4));
        assert_eq!(
            object.base_toughness.map(|value| value.base_value()),
            Some(4)
        );
        assert_eq!(object.oracle_text, "Trample");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("transform back should execute");

        assert_eq!(outcome.events.len(), 1);
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 2);
        let object = game
            .object(source)
            .expect("source permanent should still exist");
        assert_eq!(object.name, "Trail Scout");
        assert_eq!(object.subtypes, vec![Subtype::Human, Subtype::Scout]);
        assert_eq!(object.oracle_text, "Vigilance");
    }

    #[test]
    fn conquerors_galleon_returns_transformed_at_end_of_combat() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front =
            register_conquerors_galleon_pair(CardId::from_raw(79_200), CardId::from_raw(79_201));
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);

        assert!(
            game.object(source)
                .unwrap()
                .abilities
                .iter()
                .any(|ability| matches!(&ability.kind, crate::ability::AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("attacks"))),
            "Conqueror's Galleon should have an attack trigger"
        );

        let attack_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source, AttackEventTarget::Player(PlayerId::from_index(1))),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in check_triggers(&game, &attack_event) {
            trigger_queue.add(trigger);
        }
        crate::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("should queue attack trigger");
        while !game.stack_is_empty() {
            crate::resolve_stack_entry(&mut game).expect("resolve attack trigger");
        }

        assert!(
            game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Galleon")
            }),
            "Galleon should remain on the battlefield until end of combat"
        );
        assert_eq!(
            game.delayed_triggers.len(),
            1,
            "attack trigger should schedule one delayed end-of-combat trigger"
        );
        assert!(
            game.exile.is_empty(),
            "Galleon should not exile immediately"
        );

        let end_of_combat_event = TriggerEvent::new_with_provenance(
            EndOfCombatEvent::new(),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_of_combat_event) {
            trigger_queue.add(trigger);
        }
        crate::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("should queue delayed end-of-combat trigger");
        while !game.stack_is_empty() {
            crate::resolve_stack_entry(&mut game).expect("resolve delayed end-of-combat trigger");
        }

        let foothold_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Foothold")
            })
            .expect("Conqueror's Foothold should return to the battlefield");
        assert!(
            !game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Galleon")
            }),
            "front face should leave the battlefield once the delayed trigger resolves"
        );
        assert!(
            game.is_face_down(foothold_id),
            "returned permanent should transform into the Foothold face"
        );
    }

    #[test]
    fn transform_requires_a_transform_like_permanent() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let normal = CardDefinitionBuilder::new(CardId::from_raw(79_102), "Ordinary Bear")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_definition(&normal, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("non-dfc transform should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 0);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Ordinary Bear"
        );
    }

    #[test]
    fn transform_does_nothing_if_other_face_is_an_instant_or_sorcery() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_103),
            "Test Alchemist",
            CardId::from_raw(79_104),
            "Forbidden Formula",
            vec![CardType::Sorcery],
            "Draw a card.",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("illegal transform should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 0);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Test Alchemist"
        );
    }

    #[test]
    fn transform_source_ability_fizzles_if_source_already_transformed_since_it_was_stacked() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_105),
            "Twilit Ranger",
            CardId::from_raw(79_106),
            "Midnight Stalker",
            vec![CardType::Creature],
            "Trample",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source should exist"), &game);

        let mut first_ctx = ExecutionContext::new_default(source, alice);
        TransformEffect::source()
            .execute(&mut game, &mut first_ctx)
            .expect("first transform should succeed");
        assert!(game.is_face_down(source));

        let mut stale_ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut stale_ctx)
            .expect("stale self-transform should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Midnight Stalker"
        );
    }

    #[test]
    fn convert_swaps_faces_emits_converted_event_and_not_transform_event() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_107),
            "Autobot Engineer",
            CardId::from_raw(79_108),
            "Autobot Racer",
            vec![CardType::Artifact],
            "Haste",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let before_ts = game
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("battlefield permanent should have an entry timestamp");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = ConvertEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("convert should execute");

        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].kind(), EventKind::Converted);
        assert!(outcome.events[0].downcast::<ConvertedEvent>().is_some());
        assert!(outcome.events[0].downcast::<TransformedEvent>().is_none());
        assert!(game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        assert_eq!(
            game.object(source)
                .expect("source permanent should exist")
                .name,
            "Autobot Racer"
        );
        let after_ts = game
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("converted permanent should keep an entry timestamp");
        assert!(after_ts > before_ts);

        let trigger = TransformsTrigger;
        let trigger_ctx = TriggerContext::for_source(source, alice, &game);
        assert!(
            !trigger.matches(&outcome.events[0], &trigger_ctx),
            "convert should not satisfy transform-only triggers"
        );
    }

    #[test]
    fn convert_respects_cant_transform_restrictions() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_109),
            "Ground Patrol",
            CardId::from_raw(79_110),
            "Sky Convoy",
            vec![CardType::Artifact],
            "Flying",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        game.cant_effects.cant_transform.insert(source);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = ConvertEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("restricted convert should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 0);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Ground Patrol"
        );
    }

    #[test]
    fn transform_uses_game_local_linked_face_cache_after_runtime_registry_is_cleared() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_113),
            "Cache Runner",
            CardId::from_raw(79_114),
            "Cache Cruiser",
            vec![CardType::Artifact],
            "Flying",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);

        crate::cards::clear_runtime_custom_cards();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("transform should still resolve from the game-local linked-face cache");

        assert_eq!(outcome.events.len(), 1);
        assert!(game.is_face_down(source));
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Cache Cruiser"
        );
    }

    #[test]
    fn convert_source_ability_fizzles_if_source_already_transformed_since_it_was_stacked() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_111),
            "Signal Runner",
            CardId::from_raw(79_112),
            "Signal Cruiser",
            vec![CardType::Artifact],
            "Flying",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source should exist"), &game);

        let mut first_ctx = ExecutionContext::new_default(source, alice);
        TransformEffect::source()
            .execute(&mut game, &mut first_ctx)
            .expect("first transform should succeed");
        assert!(game.is_face_down(source));

        let mut stale_ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
        let outcome = ConvertEffect::source()
            .execute(&mut game, &mut stale_ctx)
            .expect("stale self-convert should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Signal Cruiser"
        );
    }
}
